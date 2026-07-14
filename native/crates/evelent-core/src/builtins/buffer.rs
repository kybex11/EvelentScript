use crate::builtins::fs::{bytes_to_buffer, value_to_bytes};
use crate::runtime::native;
use crate::value::Value;

pub fn create() -> Value {
    let m = Value::empty_object();

    let buffer_ctor = native("Buffer", 0, |_, args| {
        if let Some(v) = args.first() {
            let bytes = value_to_bytes(v).unwrap_or_default();
            Ok(bytes_to_buffer(&bytes))
        } else {
            Ok(bytes_to_buffer(&[]))
        }
    });

    // static methods on Buffer function object
    if let Value::Native(_) = &buffer_ctor {
        // Wrap as object with call + statics
    }

    let buf = Value::empty_object();
    let _ = buf.set_prop(
        "from",
        native("Buffer.from", 2, |_, args| {
            let v = args.first().cloned().unwrap_or(Value::String(String::new()));
            let enc = args.get(1).map(|v| v.as_string());
            let bytes = match (&v, enc.as_deref()) {
                (Value::String(s), Some("base64")) => base64_decode(s),
                (Value::String(s), Some("hex")) => hex::decode(s).unwrap_or_default(),
                (Value::String(s), _) => s.as_bytes().to_vec(),
                (Value::Array(a), _) => a.borrow().iter().map(|x| x.as_number() as u8).collect(),
                (other, _) => value_to_bytes(other).unwrap_or_default(),
            };
            Ok(bytes_to_buffer(&bytes))
        }),
    );

    let _ = buf.set_prop(
        "alloc",
        native("Buffer.alloc", 2, |_, args| {
            let n = args.first().map(|v| v.as_number()).unwrap_or(0.0) as usize;
            let fill = args.get(1).map(|v| v.as_number() as u8).unwrap_or(0);
            Ok(bytes_to_buffer(&vec![fill; n]))
        }),
    );

    let _ = buf.set_prop(
        "allocUnsafe",
        native("Buffer.allocUnsafe", 1, |_, args| {
            let n = args.first().map(|v| v.as_number()).unwrap_or(0.0) as usize;
            Ok(bytes_to_buffer(&vec![0u8; n]))
        }),
    );

    let _ = buf.set_prop(
        "concat",
        native("Buffer.concat", 1, |_, args| {
            let mut out = Vec::new();
            if let Some(Value::Array(a)) = args.first() {
                for item in a.borrow().iter() {
                    out.extend(value_to_bytes(item).unwrap_or_default());
                }
            }
            Ok(bytes_to_buffer(&out))
        }),
    );

    let _ = buf.set_prop(
        "isBuffer",
        native("Buffer.isBuffer", 1, |_, args| {
            let ok = matches!(args.first(), Some(Value::Object(o)) if o.borrow().get("type").map(|t| t.as_string() == "Buffer").unwrap_or(false));
            Ok(Value::Bool(ok))
        }),
    );

    let _ = buf.set_prop(
        "byteLength",
        native("Buffer.byteLength", 1, |_, args| {
            let n = args
                .first()
                .map(|v| value_to_bytes(v).map(|b| b.len()).unwrap_or(0))
                .unwrap_or(0);
            Ok(Value::Number(n as f64))
        }),
    );

    // Allow `new Buffer` via callable exported as Buffer
    let _ = m.set_prop("Buffer", buf.clone());
    // Also put statics on module root for `const { Buffer } = require('buffer')`
    let _ = m.set_prop("kMaxLength", Value::Number((isize::MAX as f64).min(1e9)));
    let _ = buffer_ctor; // silence
    m
}

fn base64_decode(s: &str) -> Vec<u8> {
    fn val(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let cleaned: Vec<u8> = s.bytes().filter(|b| !b.is_ascii_whitespace() && *b != b'=').collect();
    let mut out = Vec::new();
    for chunk in cleaned.chunks(4) {
        let mut n = 0u32;
        let mut pad = 0;
        for (i, b) in chunk.iter().enumerate() {
            if let Some(v) = val(*b) {
                n |= (v as u32) << (18 - 6 * i);
            } else {
                pad += 1;
            }
        }
        out.push(((n >> 16) & 0xff) as u8);
        if chunk.len() > 2 || pad < 2 {
            out.push(((n >> 8) & 0xff) as u8);
        }
        if chunk.len() > 3 || pad < 1 {
            out.push((n & 0xff) as u8);
        }
    }
    // trim over-push when padding
    out
}
