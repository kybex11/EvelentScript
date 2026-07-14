use crate::runtime::native;
use crate::value::Value;

pub fn create() -> Value {
    let m = Value::empty_object();

    let argv: Vec<Value> = std::env::args().map(Value::String).collect();
    let _ = m.set_prop(
        "argv",
        Value::Array(std::rc::Rc::new(std::cell::RefCell::new(argv))),
    );

    let env = Value::empty_object();
    for (k, v) in std::env::vars() {
        let _ = env.set_prop(&k, Value::String(v));
    }
    let _ = m.set_prop("env", env);

    let _ = m.set_prop(
        "cwd",
        native("process.cwd", 0, |_, _| {
            Ok(Value::String(
                std::env::current_dir()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_else(|_| ".".into()),
            ))
        }),
    );

    let _ = m.set_prop(
        "chdir",
        native("process.chdir", 1, |_, args| {
            let p = args.first().map(|v| v.as_string()).unwrap_or_default();
            std::env::set_current_dir(p).map_err(crate::error::Error::from)?;
            Ok(Value::Undefined)
        }),
    );

    let _ = m.set_prop(
        "exit",
        native("process.exit", 1, |_, args| {
            let code = args.first().map(|v| v.as_number() as i32).unwrap_or(0);
            std::process::exit(code);
        }),
    );

    let _ = m.set_prop(
        "platform",
        Value::String(
            if cfg!(windows) {
                "win32"
            } else if cfg!(target_os = "macos") {
                "darwin"
            } else {
                "linux"
            }
            .into(),
        ),
    );

    let _ = m.set_prop(
        "arch",
        Value::String(
            if cfg!(target_arch = "x86_64") {
                "x64"
            } else if cfg!(target_arch = "aarch64") {
                "arm64"
            } else {
                "unknown"
            }
            .into(),
        ),
    );

    let _ = m.set_prop("version", Value::String(format!("v{}", env!("CARGO_PKG_VERSION"))));
    let _ = m.set_prop("title", Value::String("evelent".into()));
    let _ = m.set_prop("pid", Value::Number(std::process::id() as f64));

    let _ = m.set_prop(
        "nextTick",
        native("process.nextTick", 1, |vm, args| {
            if let Some(cb) = args.first() {
                let rest: Vec<Value> = args.iter().skip(1).cloned().collect();
                let _ = vm.call_value_public(cb.clone(), rest, None)?;
            }
            Ok(Value::Undefined)
        }),
    );

    let _ = m.set_prop(
        "stdout",
        {
            let s = Value::empty_object();
            let _ = s.set_prop(
                "write",
                native("stdout.write", 1, |_, args| {
                    print!("{}", args.first().map(|v| v.as_string()).unwrap_or_default());
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                    Ok(Value::Bool(true))
                }),
            );
            s
        },
    );

    let _ = m.set_prop(
        "stderr",
        {
            let s = Value::empty_object();
            let _ = s.set_prop(
                "write",
                native("stderr.write", 1, |_, args| {
                    eprint!("{}", args.first().map(|v| v.as_string()).unwrap_or_default());
                    Ok(Value::Bool(true))
                }),
            );
            s
        },
    );

    m
}
