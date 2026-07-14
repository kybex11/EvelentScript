//! Node.js-compatible builtin modules for the native Rust VM.

mod assert;
mod buffer;
mod child_process;
mod crypto;
mod events;
mod fs;
mod http;
mod os;
mod path;
mod process;
mod querystring;
mod stream;
mod url;
mod util;
mod vm;
mod zlib;

use crate::error::{Error, Result};
use crate::value::Value;

/// Load a Node-style builtin by name (`fs`, `path`, …).
pub fn load(name: &str) -> Result<Option<Value>> {
    let mod_value = match name {
        "fs" | "node:fs" => fs::create(),
        "path" | "node:path" => path::create(),
        "os" | "node:os" => os::create(),
        "util" | "node:util" => util::create(),
        "assert" | "node:assert" => assert::create(),
        "url" | "node:url" => url::create(),
        "querystring" | "node:querystring" | "qs" => querystring::create(),
        "crypto" | "node:crypto" => crypto::create(),
        "buffer" | "node:buffer" => buffer::create(),
        "events" | "node:events" => events::create(),
        "process" | "node:process" => process::create(),
        "child_process" | "node:child_process" => child_process::create(),
        "http" | "node:http" => http::create(false),
        "https" | "node:https" => http::create(true),
        "stream" | "node:stream" => stream::create(),
        "zlib" | "node:zlib" => zlib::create(),
        "vm" | "node:vm" => vm::create(),
        "module" | "node:module" => create_module_meta(),
        "console" | "node:console" => create_console_module(),
        "timers" | "node:timers" => create_timers(),
        "string_decoder" | "node:string_decoder" => create_string_decoder(),
        "punycode" | "node:punycode" => create_punycode(),
        "readline" | "node:readline" => create_readline(),
        "perf_hooks" | "node:perf_hooks" => create_perf_hooks(),
        "tty" | "node:tty" => create_tty(),
        "constants" | "node:constants" => create_constants(),
        _ => return Ok(None),
    };
    Ok(Some(mod_value))
}

/// Names of all registered builtins (for docs / introspection).
pub fn names() -> &'static [&'static str] {
    &[
        "assert",
        "buffer",
        "child_process",
        "console",
        "constants",
        "crypto",
        "events",
        "fs",
        "http",
        "https",
        "module",
        "os",
        "path",
        "perf_hooks",
        "process",
        "punycode",
        "querystring",
        "readline",
        "stream",
        "string_decoder",
        "timers",
        "tty",
        "url",
        "util",
        "vm",
        "zlib",
    ]
}

fn create_module_meta() -> Value {
    use crate::runtime::native;
    let m = Value::empty_object();
    let _ = m.set_prop(
        "builtinModules",
        Value::Array(std::rc::Rc::new(std::cell::RefCell::new(
            names()
                .iter()
                .map(|n| Value::String((*n).into()))
                .collect(),
        ))),
    );
    let _ = m.set_prop(
        "isBuiltin",
        native("module.isBuiltin", 1, |_, args| {
            let name = args.first().map(|v| v.as_string()).unwrap_or_default();
            let name = name.strip_prefix("node:").unwrap_or(&name);
            Ok(Value::Bool(names().contains(&name)))
        }),
    );
    m
}

fn create_console_module() -> Value {
    use crate::runtime::native;
    let c = Value::empty_object();
    let log = native("console.log", 0, |_, args| {
        let parts: Vec<String> = args.iter().map(|a| a.to_string()).collect();
        println!("{}", parts.join(" "));
        Ok(Value::Undefined)
    });
    let _ = c.set_prop("log", log.clone());
    let _ = c.set_prop("info", log.clone());
    let _ = c.set_prop("debug", log.clone());
    let _ = c.set_prop(
        "warn",
        native("console.warn", 0, |_, args| {
            let parts: Vec<String> = args.iter().map(|a| a.to_string()).collect();
            eprintln!("{}", parts.join(" "));
            Ok(Value::Undefined)
        }),
    );
    let _ = c.set_prop(
        "error",
        native("console.error", 0, |_, args| {
            let parts: Vec<String> = args.iter().map(|a| a.to_string()).collect();
            eprintln!("{}", parts.join(" "));
            Ok(Value::Undefined)
        }),
    );
    c
}

fn create_timers() -> Value {
    use crate::runtime::native;
    let t = Value::empty_object();
    let _ = t.set_prop(
        "setTimeout",
        native("setTimeout", 2, |vm, args| {
            let ms = args.get(1).map(|v| v.as_number()).unwrap_or(0.0).max(0.0) as u64;
            std::thread::sleep(std::time::Duration::from_millis(ms));
            if let Some(cb) = args.first() {
                let _ = vm.call_value_public(cb.clone(), vec![], None)?;
            }
            Ok(Value::Number(1.0))
        }),
    );
    let _ = t.set_prop(
        "setInterval",
        native("setInterval", 2, |_, _| {
            Err(Error::Other(
                "setInterval is not supported in the sync VM; use a loop + setTimeout".into(),
            ))
        }),
    );
    let _ = t.set_prop(
        "clearTimeout",
        native("clearTimeout", 1, |_, _| Ok(Value::Undefined)),
    );
    let _ = t.set_prop(
        "clearInterval",
        native("clearInterval", 1, |_, _| Ok(Value::Undefined)),
    );
    t
}

fn create_string_decoder() -> Value {
    use crate::runtime::native;
    let m = Value::empty_object();
    let _ = m.set_prop(
        "StringDecoder",
        native("StringDecoder", 1, |_, args| {
            let encoding = args
                .first()
                .map(|v| v.as_string())
                .unwrap_or_else(|| "utf8".into());
            let obj = Value::empty_object();
            let _ = obj.set_prop("encoding", Value::String(encoding));
            let _ = obj.set_prop(
                "write",
                native("StringDecoder.write", 1, |_, args| {
                    Ok(args.first().cloned().unwrap_or(Value::String(String::new())))
                }),
            );
            let _ = obj.set_prop(
                "end",
                native("StringDecoder.end", 0, |_, args| {
                    Ok(args.first().cloned().unwrap_or(Value::String(String::new())))
                }),
            );
            Ok(obj)
        }),
    );
    m
}

fn create_punycode() -> Value {
    use crate::runtime::native;
    let m = Value::empty_object();
    // Minimal: identity encode/decode for ASCII hosts
    let id = native("punycode.toASCII", 1, |_, args| {
        Ok(args.first().cloned().unwrap_or(Value::String(String::new())))
    });
    let _ = m.set_prop("toASCII", id.clone());
    let _ = m.set_prop("toUnicode", id);
    m
}

fn create_readline() -> Value {
    use crate::runtime::native;
    let m = Value::empty_object();
    let _ = m.set_prop(
        "createInterface",
        native("readline.createInterface", 1, |_, _| {
            let iface = Value::empty_object();
            let _ = iface.set_prop(
                "question",
                native("readline.question", 2, |vm, args| {
                    let prompt = args.first().map(|v| v.as_string()).unwrap_or_default();
                    print!("{prompt}");
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                    let mut line = String::new();
                    let _ = std::io::stdin().read_line(&mut line);
                    let line = line.trim_end_matches(['\r', '\n']).to_string();
                    if let Some(cb) = args.get(1) {
                        let _ = vm.call_value_public(cb.clone(), vec![Value::String(line.clone())], None)?;
                    }
                    Ok(Value::String(line))
                }),
            );
            let _ = iface.set_prop(
                "close",
                native("readline.close", 0, |_, _| Ok(Value::Undefined)),
            );
            Ok(iface)
        }),
    );
    m
}

fn create_perf_hooks() -> Value {
    use crate::runtime::native;
    let m = Value::empty_object();
    let _ = m.set_prop(
        "performance",
        {
            let p = Value::empty_object();
            let _ = p.set_prop(
                "now",
                native("performance.now", 0, |_, _| {
                    use std::time::{SystemTime, UNIX_EPOCH};
                    let ms = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_secs_f64() * 1000.0)
                        .unwrap_or(0.0);
                    Ok(Value::Number(ms))
                }),
            );
            p
        },
    );
    m
}

fn create_tty() -> Value {
    use crate::runtime::native;
    let m = Value::empty_object();
    let _ = m.set_prop(
        "isatty",
        native("tty.isatty", 1, |_, _| {
            Ok(Value::Bool(atty_stdout()))
        }),
    );
    m
}

fn atty_stdout() -> bool {
    #[cfg(windows)]
    {
        use std::io::IsTerminal;
        std::io::stdout().is_terminal()
    }
    #[cfg(not(windows))]
    {
        use std::io::IsTerminal;
        std::io::stdout().is_terminal()
    }
}

fn create_constants() -> Value {
    let m = Value::empty_object();
    let _ = m.set_prop("OK", Value::Number(0.0));
    let fs = Value::empty_object();
    let _ = fs.set_prop("F_OK", Value::Number(0.0));
    let _ = fs.set_prop("R_OK", Value::Number(4.0));
    let _ = fs.set_prop("W_OK", Value::Number(2.0));
    let _ = fs.set_prop("X_OK", Value::Number(1.0));
    let _ = m.set_prop("fs", fs);
    m
}
