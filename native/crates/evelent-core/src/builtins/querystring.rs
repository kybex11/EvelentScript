use crate::runtime::native;
use crate::value::Value;

pub fn create() -> Value {
    let m = Value::empty_object();

    let _ = m.set_prop(
        "parse",
        native("querystring.parse", 1, |_, args| {
            let s = args.first().map(|v| v.as_string()).unwrap_or_default();
            let s = s.strip_prefix('?').unwrap_or(&s);
            let obj = Value::empty_object();
            for pair in s.split('&') {
                if pair.is_empty() {
                    continue;
                }
                let (k, v) = match pair.split_once('=') {
                    Some((k, v)) => (decode(k), decode(v)),
                    None => (decode(pair), String::new()),
                };
                let _ = obj.set_prop(&k, Value::String(v));
            }
            Ok(obj)
        }),
    );

    let _ = m.set_prop(
        "stringify",
        native("querystring.stringify", 1, |_, args| {
            let obj = args.first().cloned().unwrap_or(Value::empty_object());
            let mut parts = Vec::new();
            if let Value::Object(o) = obj {
                for (k, v) in o.borrow().iter() {
                    parts.push(format!("{}={}", encode(k), encode(&v.as_string())));
                }
            }
            Ok(Value::String(parts.join("&")))
        }),
    );

    let _ = m.set_prop(
        "escape",
        native("querystring.escape", 1, |_, args| {
            Ok(Value::String(encode(
                &args.first().map(|v| v.as_string()).unwrap_or_default(),
            )))
        }),
    );

    let _ = m.set_prop(
        "unescape",
        native("querystring.unescape", 1, |_, args| {
            Ok(Value::String(decode(
                &args.first().map(|v| v.as_string()).unwrap_or_default(),
            )))
        }),
    );

    m
}

fn encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn decode(s: &str) -> String {
    let mut out = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hex = &s[i + 1..i + 3];
                if let Ok(v) = u8::from_str_radix(hex, 16) {
                    out.push(v);
                }
                i += 3;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}
