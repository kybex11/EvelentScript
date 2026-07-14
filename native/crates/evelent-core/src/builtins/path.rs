use std::path::{Path, PathBuf};

use crate::runtime::native;
use crate::value::Value;

pub fn create() -> Value {
    let m = Value::empty_object();
    let sep = if cfg!(windows) { "\\" } else { "/" };
    let _ = m.set_prop("sep", Value::String(sep.into()));
    let _ = m.set_prop("delimiter", Value::String(if cfg!(windows) { ";" } else { ":" }.into()));

    let _ = m.set_prop(
        "join",
        native("path.join", 0, |_, args| {
            let mut parts: Vec<String> = args.iter().map(|a| a.as_string()).collect();
            parts.retain(|p| !p.is_empty());
            let joined = parts.join(if cfg!(windows) { "\\" } else { "/" });
            Ok(Value::String(normalize_str(&joined)))
        }),
    );

    let _ = m.set_prop(
        "resolve",
        native("path.resolve", 0, |_, args| {
            let mut cur = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            for a in args {
                let p = PathBuf::from(a.as_string());
                if p.is_absolute() {
                    cur = p;
                } else {
                    cur.push(p);
                }
            }
            Ok(Value::String(
                cur.canonicalize()
                    .unwrap_or(cur)
                    .to_string_lossy()
                    .into_owned(),
            ))
        }),
    );

    let _ = m.set_prop(
        "dirname",
        native("path.dirname", 1, |_, args| {
            let p = args.first().map(|v| v.as_string()).unwrap_or_default();
            let d = Path::new(&p)
                .parent()
                .map(|x| x.to_string_lossy().into_owned())
                .unwrap_or_else(|| ".".into());
            Ok(Value::String(d))
        }),
    );

    let _ = m.set_prop(
        "basename",
        native("path.basename", 2, |_, args| {
            let p = args.first().map(|v| v.as_string()).unwrap_or_default();
            let mut name = Path::new(&p)
                .file_name()
                .map(|x| x.to_string_lossy().into_owned())
                .unwrap_or_default();
            if let Some(ext) = args.get(1) {
                let e = ext.as_string();
                if name.ends_with(&e) {
                    name.truncate(name.len() - e.len());
                }
            }
            Ok(Value::String(name))
        }),
    );

    let _ = m.set_prop(
        "extname",
        native("path.extname", 1, |_, args| {
            let p = args.first().map(|v| v.as_string()).unwrap_or_default();
            let e = Path::new(&p)
                .extension()
                .map(|x| format!(".{}", x.to_string_lossy()))
                .unwrap_or_default();
            Ok(Value::String(e))
        }),
    );

    let _ = m.set_prop(
        "normalize",
        native("path.normalize", 1, |_, args| {
            let p = args.first().map(|v| v.as_string()).unwrap_or_default();
            Ok(Value::String(normalize_str(&p)))
        }),
    );

    let _ = m.set_prop(
        "isAbsolute",
        native("path.isAbsolute", 1, |_, args| {
            let p = args.first().map(|v| v.as_string()).unwrap_or_default();
            Ok(Value::Bool(Path::new(&p).is_absolute()))
        }),
    );

    let _ = m.set_prop(
        "relative",
        native("path.relative", 2, |_, args| {
            let from = args.first().map(|v| v.as_string()).unwrap_or_default();
            let to = args.get(1).map(|v| v.as_string()).unwrap_or_default();
            let rel = pathdiff_simple(&from, &to);
            Ok(Value::String(rel))
        }),
    );

    let _ = m.set_prop(
        "parse",
        native("path.parse", 1, |_, args| {
            let p = args.first().map(|v| v.as_string()).unwrap_or_default();
            let path = Path::new(&p);
            let obj = Value::empty_object();
            let _ = obj.set_prop(
                "root",
                Value::String(
                    path.components()
                        .next()
                        .map(|c| c.as_os_str().to_string_lossy().into_owned())
                        .unwrap_or_default(),
                ),
            );
            let _ = obj.set_prop(
                "dir",
                Value::String(
                    path.parent()
                        .map(|x| x.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                ),
            );
            let _ = obj.set_prop(
                "base",
                Value::String(
                    path.file_name()
                        .map(|x| x.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                ),
            );
            let _ = obj.set_prop(
                "ext",
                Value::String(
                    path.extension()
                        .map(|x| format!(".{}", x.to_string_lossy()))
                        .unwrap_or_default(),
                ),
            );
            let _ = obj.set_prop(
                "name",
                Value::String(
                    path.file_stem()
                        .map(|x| x.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                ),
            );
            Ok(obj)
        }),
    );

    m
}

fn normalize_str(p: &str) -> String {
    let sep = if cfg!(windows) { '\\' } else { '/' };
    let alt = if cfg!(windows) { '/' } else { '\\' };
    let p = p.replace(alt, &sep.to_string());
    let mut out: Vec<&str> = Vec::new();
    for part in p.split(sep) {
        match part {
            "" | "." => {
                if out.is_empty() && p.starts_with(sep) {
                    // keep absolute root marker later
                }
            }
            ".." => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    let mut s = out.join(&sep.to_string());
    if p.starts_with(sep) && !s.starts_with(sep) {
        s = format!("{sep}{s}");
    }
    if s.is_empty() {
        ".".into()
    } else {
        s
    }
}

fn pathdiff_simple(from: &str, to: &str) -> String {
    let from_c: Vec<_> = Path::new(from).components().collect();
    let to_c: Vec<_> = Path::new(to).components().collect();
    let mut i = 0;
    while i < from_c.len() && i < to_c.len() && from_c[i] == to_c[i] {
        i += 1;
    }
    let mut parts = Vec::new();
    for _ in i..from_c.len() {
        parts.push("..".to_string());
    }
    for c in &to_c[i..] {
        parts.push(c.as_os_str().to_string_lossy().into_owned());
    }
    if parts.is_empty() {
        ".".into()
    } else {
        parts.join(if cfg!(windows) { "\\" } else { "/" })
    }
}
