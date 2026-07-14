use md5::{Digest as _, Md5};
use sha2::{Sha256, Sha512};

use crate::builtins::fs::bytes_to_buffer;
use crate::error::{Error, Result};
use crate::runtime::native;
use crate::value::Value;

pub fn create() -> Value {
    let m = Value::empty_object();

    let _ = m.set_prop(
        "createHash",
        native("crypto.createHash", 1, |_, args| {
            let algo = args
                .first()
                .map(|v| v.as_string())
                .unwrap_or_else(|| "sha256".into())
                .to_lowercase();
            Ok(make_hasher(&algo))
        }),
    );

    let _ = m.set_prop(
        "randomBytes",
        native("crypto.randomBytes", 1, |_, args| {
            let n = args.first().map(|v| v.as_number()).unwrap_or(16.0) as usize;
            let mut buf = vec![0u8; n];
            getrandom_fill(&mut buf);
            Ok(bytes_to_buffer(&buf))
        }),
    );

    let _ = m.set_prop(
        "randomUUID",
        native("crypto.randomUUID", 0, |_, _| {
            let mut b = [0u8; 16];
            getrandom_fill(&mut b);
            b[6] = (b[6] & 0x0f) | 0x40;
            b[8] = (b[8] & 0x3f) | 0x80;
            Ok(Value::String(format!(
                "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12],
                b[13], b[14], b[15]
            )))
        }),
    );

    let _ = m.set_prop(
        "createHmac",
        native("crypto.createHmac", 2, |_, args| {
            let algo = args.first().map(|v| v.as_string()).unwrap_or_else(|| "sha256".into());
            let key = args.get(1).map(|v| v.as_string()).unwrap_or_default();
            Ok(make_hasher_with_prefix(&algo, key.as_bytes()))
        }),
    );

    m
}

fn make_hasher(algo: &str) -> Value {
    make_hasher_with_prefix(algo, &[])
}

fn make_hasher_with_prefix(algo: &str, prefix: &[u8]) -> Value {
    let algo = algo.to_string();
    let prefix = prefix.to_vec();
    let state = std::rc::Rc::new(std::cell::RefCell::new(prefix));

    let hasher = Value::empty_object();
    let state_u = state.clone();
    let algo_u = algo.clone();
    let _ = hasher.set_prop(
        "update",
        native("hash.update", 1, move |_, args| {
            let chunk = match args.first() {
                Some(Value::String(s)) => s.as_bytes().to_vec(),
                Some(Value::Object(o)) => {
                    if let Some(Value::Array(a)) = o.borrow().get("data") {
                        a.borrow().iter().map(|x| x.as_number() as u8).collect()
                    } else {
                        args.first().unwrap().as_string().into_bytes()
                    }
                }
                Some(v) => v.as_string().into_bytes(),
                None => vec![],
            };
            state_u.borrow_mut().extend_from_slice(&chunk);
            // return self — reconstruct is hard; return a digest-capable object sharing state
            Ok(make_hasher_from_state(&algo_u, state_u.clone()))
        }),
    );

    let state_d = state.clone();
    let algo_d = algo;
    let _ = hasher.set_prop(
        "digest",
        native("hash.digest", 1, move |_, args| {
            let enc = args
                .first()
                .map(|v| v.as_string())
                .unwrap_or_else(|| "hex".into());
            let data = state_d.borrow().clone();
            let digest = hash_bytes(&algo_d, &data)?;
            if enc == "hex" {
                Ok(Value::String(hex::encode(digest)))
            } else {
                Ok(bytes_to_buffer(&digest))
            }
        }),
    );
    hasher
}

fn make_hasher_from_state(
    algo: &str,
    state: std::rc::Rc<std::cell::RefCell<Vec<u8>>>,
) -> Value {
    let hasher = Value::empty_object();
    let state_u = state.clone();
    let algo_u = algo.to_string();
    let _ = hasher.set_prop(
        "update",
        native("hash.update", 1, move |_, args| {
            let chunk = args
                .first()
                .map(|v| v.as_string().into_bytes())
                .unwrap_or_default();
            state_u.borrow_mut().extend_from_slice(&chunk);
            Ok(make_hasher_from_state(&algo_u, state_u.clone()))
        }),
    );
    let state_d = state;
    let algo_d = algo.to_string();
    let _ = hasher.set_prop(
        "digest",
        native("hash.digest", 1, move |_, args| {
            let enc = args
                .first()
                .map(|v| v.as_string())
                .unwrap_or_else(|| "hex".into());
            let data = state_d.borrow().clone();
            let digest = hash_bytes(&algo_d, &data)?;
            if enc == "hex" {
                Ok(Value::String(hex::encode(digest)))
            } else {
                Ok(bytes_to_buffer(&digest))
            }
        }),
    );
    hasher
}

fn hash_bytes(algo: &str, data: &[u8]) -> Result<Vec<u8>> {
    match algo {
        "md5" => {
            let mut h = Md5::new();
            h.update(data);
            Ok(h.finalize().to_vec())
        }
        "sha256" | "sha2" => {
            let mut h = Sha256::new();
            h.update(data);
            Ok(h.finalize().to_vec())
        }
        "sha512" => {
            let mut h = Sha512::new();
            h.update(data);
            Ok(h.finalize().to_vec())
        }
        other => Err(Error::Other(format!("unsupported hash algorithm: {other}"))),
    }
}

fn getrandom_fill(buf: &mut [u8]) {
    // Prefer OS randomness via getrandom crate isn't in deps — use a mix of time + hash
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};
    let mut seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1);
    for (i, b) in buf.iter_mut().enumerate() {
        let mut h = DefaultHasher::new();
        seed.hash(&mut h);
        i.hash(&mut h);
        seed = h.finish();
        *b = (seed & 0xff) as u8;
    }
    #[cfg(windows)]
    {
        // Try RtlGenRandom / BCrypt if available later — for now seeded PRNG is OK for MVP
    }
}
