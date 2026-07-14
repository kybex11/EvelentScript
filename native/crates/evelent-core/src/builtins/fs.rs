use std::fs;
use std::path::PathBuf;

use crate::error::{Error, Result};
use crate::runtime::native;
use crate::value::Value;

pub fn create() -> Value {
    let m = Value::empty_object();

    let _ = m.set_prop(
        "readFileSync",
        native("fs.readFileSync", 2, |_, args| {
            let path = arg_path(args, 0)?;
            let encoding = args.get(1).map(|v| v.as_string());
            let bytes = fs::read(&path).map_err(Error::from)?;
            if encoding.as_deref() == Some("utf8")
                || encoding.as_deref() == Some("utf-8")
                || encoding.is_none()
            {
                // Node defaults to Buffer; we return utf8 string when encoding set, else Buffer-like
                if encoding.is_some() {
                    Ok(Value::String(
                        String::from_utf8_lossy(&bytes).into_owned(),
                    ))
                } else {
                    Ok(bytes_to_buffer(&bytes))
                }
            } else if encoding.as_deref() == Some("base64") {
                Ok(Value::String(base64_encode(&bytes)))
            } else {
                Ok(Value::String(
                    String::from_utf8_lossy(&bytes).into_owned(),
                ))
            }
        }),
    );

    let _ = m.set_prop(
        "writeFileSync",
        native("fs.writeFileSync", 2, |_, args| {
            let path = arg_path(args, 0)?;
            let data = args.get(1).cloned().unwrap_or(Value::String(String::new()));
            let bytes = value_to_bytes(&data)?;
            fs::write(&path, bytes).map_err(Error::from)?;
            Ok(Value::Undefined)
        }),
    );

    let _ = m.set_prop(
        "appendFileSync",
        native("fs.appendFileSync", 2, |_, args| {
            let path = arg_path(args, 0)?;
            let data = args.get(1).cloned().unwrap_or(Value::String(String::new()));
            let bytes = value_to_bytes(&data)?;
            use std::io::Write;
            let mut f = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .map_err(Error::from)?;
            f.write_all(&bytes).map_err(Error::from)?;
            Ok(Value::Undefined)
        }),
    );

    let _ = m.set_prop(
        "existsSync",
        native("fs.existsSync", 1, |_, args| {
            Ok(Value::Bool(arg_path(args, 0).map(|p| p.exists()).unwrap_or(false)))
        }),
    );

    let _ = m.set_prop(
        "mkdirSync",
        native("fs.mkdirSync", 2, |_, args| {
            let path = arg_path(args, 0)?;
            let recursive = args
                .get(1)
                .and_then(|v| match v {
                    Value::Object(o) => o.borrow().get("recursive").map(|r| r.is_truthy()),
                    Value::Bool(b) => Some(*b),
                    _ => None,
                })
                .unwrap_or(false);
            if recursive {
                fs::create_dir_all(&path).map_err(Error::from)?;
            } else {
                fs::create_dir(&path).map_err(Error::from)?;
            }
            Ok(Value::Undefined)
        }),
    );

    let _ = m.set_prop(
        "readdirSync",
        native("fs.readdirSync", 1, |_, args| {
            let path = arg_path(args, 0)?;
            let mut names = Vec::new();
            for entry in fs::read_dir(&path).map_err(Error::from)? {
                let entry = entry.map_err(Error::from)?;
                names.push(Value::String(
                    entry.file_name().to_string_lossy().into_owned(),
                ));
            }
            names.sort_by(|a, b| a.as_string().cmp(&b.as_string()));
            Ok(Value::Array(std::rc::Rc::new(std::cell::RefCell::new(names))))
        }),
    );

    let _ = m.set_prop(
        "unlinkSync",
        native("fs.unlinkSync", 1, |_, args| {
            fs::remove_file(arg_path(args, 0)?).map_err(Error::from)?;
            Ok(Value::Undefined)
        }),
    );

    let _ = m.set_prop(
        "rmdirSync",
        native("fs.rmdirSync", 1, |_, args| {
            fs::remove_dir(arg_path(args, 0)?).map_err(Error::from)?;
            Ok(Value::Undefined)
        }),
    );

    let _ = m.set_prop(
        "rmSync",
        native("fs.rmSync", 2, |_, args| {
            let path = arg_path(args, 0)?;
            let recursive = args
                .get(1)
                .and_then(|v| match v {
                    Value::Object(o) => o.borrow().get("recursive").map(|r| r.is_truthy()),
                    _ => None,
                })
                .unwrap_or(false);
            if path.is_dir() {
                if recursive {
                    fs::remove_dir_all(&path).map_err(Error::from)?;
                } else {
                    fs::remove_dir(&path).map_err(Error::from)?;
                }
            } else {
                fs::remove_file(&path).map_err(Error::from)?;
            }
            Ok(Value::Undefined)
        }),
    );

    let _ = m.set_prop(
        "statSync",
        native("fs.statSync", 1, |_, args| {
            let path = arg_path(args, 0)?;
            let meta = fs::metadata(&path).map_err(Error::from)?;
            let is_file = meta.is_file();
            let is_dir = meta.is_dir();
            let size = meta.len();
            let obj = Value::empty_object();
            let _ = obj.set_prop("size", Value::Number(size as f64));
            let _ = obj.set_prop(
                "isFile",
                native("stat.isFile", 0, move |_, _| Ok(Value::Bool(is_file))),
            );
            let _ = obj.set_prop(
                "isDirectory",
                native("stat.isDirectory", 0, move |_, _| Ok(Value::Bool(is_dir))),
            );
            Ok(obj)
        }),
    );

    let _ = m.set_prop(
        "copyFileSync",
        native("fs.copyFileSync", 2, |_, args| {
            let from = arg_path(args, 0)?;
            let to = arg_path(args, 1)?;
            fs::copy(&from, &to).map_err(Error::from)?;
            Ok(Value::Undefined)
        }),
    );

    let _ = m.set_prop(
        "renameSync",
        native("fs.renameSync", 2, |_, args| {
            fs::rename(arg_path(args, 0)?, arg_path(args, 1)?).map_err(Error::from)?;
            Ok(Value::Undefined)
        }),
    );

    let _ = m.set_prop(
        "realpathSync",
        native("fs.realpathSync", 1, |_, args| {
            let p = fs::canonicalize(arg_path(args, 0)?).map_err(Error::from)?;
            Ok(Value::String(p.to_string_lossy().into_owned()))
        }),
    );

    m
}

fn arg_path(args: &[Value], i: usize) -> Result<PathBuf> {
    args.get(i)
        .map(|v| PathBuf::from(v.as_string()))
        .ok_or_else(|| Error::Other("path argument required".into()))
}

pub(crate) fn value_to_bytes(v: &Value) -> Result<Vec<u8>> {
    match v {
        Value::String(s) => Ok(s.as_bytes().to_vec()),
        Value::Array(a) => Ok(a
            .borrow()
            .iter()
            .map(|x| x.as_number() as u8)
            .collect()),
        Value::Object(o) => {
            if let Some(Value::Array(a)) = o.borrow().get("data") {
                Ok(a.borrow().iter().map(|x| x.as_number() as u8).collect())
            } else if let Some(Value::String(s)) = o.borrow().get("toString") {
                Ok(s.as_bytes().to_vec())
            } else {
                Ok(v.as_string().into_bytes())
            }
        }
        other => Ok(other.as_string().into_bytes()),
    }
}

pub(crate) fn bytes_to_buffer(bytes: &[u8]) -> Value {
    let data: Vec<Value> = bytes.iter().map(|b| Value::Number(*b as f64)).collect();
    let obj = Value::empty_object();
    let _ = obj.set_prop("type", Value::String("Buffer".into()));
    let _ = obj.set_prop(
        "data",
        Value::Array(std::rc::Rc::new(std::cell::RefCell::new(data))),
    );
    let _ = obj.set_prop("length", Value::Number(bytes.len() as f64));
    let owned = bytes.to_vec();
    let _ = obj.set_prop(
        "toString",
        native("Buffer.toString", 1, move |_, args| {
            let enc = args.first().map(|v| v.as_string()).unwrap_or_else(|| "utf8".into());
            if enc == "base64" {
                Ok(Value::String(base64_encode(&owned)))
            } else if enc == "hex" {
                Ok(Value::String(hex::encode(&owned)))
            } else {
                Ok(Value::String(String::from_utf8_lossy(&owned).into_owned()))
            }
        }),
    );
    obj
}

fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let mut buf = [0u8; 3];
        for (i, b) in chunk.iter().enumerate() {
            buf[i] = *b;
        }
        let n = ((buf[0] as u32) << 16) | ((buf[1] as u32) << 8) | (buf[2] as u32);
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((n >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(n & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}
