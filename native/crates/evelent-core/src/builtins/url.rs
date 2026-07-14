use crate::runtime::native;
use crate::value::Value;

pub fn create() -> Value {
    let m = Value::empty_object();

    let _ = m.set_prop(
        "parse",
        native("url.parse", 1, |_, args| {
            let raw = args.first().map(|v| v.as_string()).unwrap_or_default();
            Ok(parse_url(&raw))
        }),
    );

    let _ = m.set_prop(
        "format",
        native("url.format", 1, |_, args| {
            let obj = args.first().cloned().unwrap_or(Value::empty_object());
            let protocol = obj.get_prop("protocol").as_string();
            let host = {
                let h = obj.get_prop("host").as_string();
                if h.is_empty() {
                    let hostname = obj.get_prop("hostname").as_string();
                    let port = obj.get_prop("port").as_string();
                    if port.is_empty() {
                        hostname
                    } else {
                        format!("{hostname}:{port}")
                    }
                } else {
                    h
                }
            };
            let pathname = obj.get_prop("pathname").as_string();
            let search = obj.get_prop("search").as_string();
            let hash = obj.get_prop("hash").as_string();
            let mut out = String::new();
            if !protocol.is_empty() {
                out.push_str(&protocol);
                if !protocol.ends_with(':') {
                    out.push(':');
                }
                out.push_str("//");
            }
            out.push_str(&host);
            if !pathname.is_empty() {
                if !pathname.starts_with('/') {
                    out.push('/');
                }
                out.push_str(&pathname);
            }
            out.push_str(&search);
            out.push_str(&hash);
            Ok(Value::String(out))
        }),
    );

    let _ = m.set_prop(
        "fileURLToPath",
        native("url.fileURLToPath", 1, |_, args| {
            let u = args.first().map(|v| v.as_string()).unwrap_or_default();
            let path = u
                .strip_prefix("file:///")
                .or_else(|| u.strip_prefix("file://"))
                .unwrap_or(&u)
                .replace('/', std::path::MAIN_SEPARATOR_STR);
            Ok(Value::String(path))
        }),
    );

    let _ = m.set_prop(
        "pathToFileURL",
        native("url.pathToFileURL", 1, |_, args| {
            let p = args.first().map(|v| v.as_string()).unwrap_or_default();
            let normalized = p.replace('\\', "/");
            let url = if normalized.starts_with('/') {
                format!("file://{normalized}")
            } else {
                format!("file:///{normalized}")
            };
            let obj = Value::empty_object();
            let _ = obj.set_prop("href", Value::String(url));
            Ok(obj)
        }),
    );

    m
}

fn parse_url(raw: &str) -> Value {
    let obj = Value::empty_object();
    let _ = obj.set_prop("href", Value::String(raw.into()));

    let (rest, protocol) = if let Some(i) = raw.find("://") {
        (&raw[i + 3..], Some(&raw[..=i + 1]))
    } else {
        (raw, None)
    };
    if let Some(p) = protocol {
        let _ = obj.set_prop("protocol", Value::String(p.into()));
    }

    let (host_part, path_part) = if let Some(i) = rest.find('/') {
        (&rest[..i], &rest[i..])
    } else if let Some(i) = rest.find('?') {
        (&rest[..i], &rest[i..])
    } else {
        (rest, "")
    };

    let _ = obj.set_prop("host", Value::String(host_part.into()));
    let (hostname, port) = if let Some(i) = host_part.rfind(':') {
        // ignore IPv6 for MVP
        (&host_part[..i], &host_part[i + 1..])
    } else {
        (host_part, "")
    };
    let _ = obj.set_prop("hostname", Value::String(hostname.into()));
    let _ = obj.set_prop("port", Value::String(port.into()));

    let (pathname, search_hash) = if let Some(i) = path_part.find('?') {
        (&path_part[..i], &path_part[i..])
    } else if let Some(i) = path_part.find('#') {
        (&path_part[..i], &path_part[i..])
    } else {
        (path_part, "")
    };
    let _ = obj.set_prop(
        "pathname",
        Value::String(if pathname.is_empty() {
            "/".into()
        } else {
            pathname.into()
        }),
    );

    let (search, hash) = if let Some(i) = search_hash.find('#') {
        (&search_hash[..i], &search_hash[i..])
    } else {
        (search_hash, "")
    };
    let _ = obj.set_prop("search", Value::String(search.into()));
    let _ = obj.set_prop("hash", Value::String(hash.into()));
    let query = search.strip_prefix('?').unwrap_or("");
    let _ = obj.set_prop("query", Value::String(query.into()));
    let _ = obj.set_prop("path", Value::String(format!("{pathname}{search}")));
    obj
}
