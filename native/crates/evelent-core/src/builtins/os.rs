use crate::runtime::native;
use crate::value::Value;

pub fn create() -> Value {
    let m = Value::empty_object();

    let _ = m.set_prop(
        "platform",
        native("os.platform", 0, |_, _| {
            Ok(Value::String(
                if cfg!(windows) {
                    "win32"
                } else if cfg!(target_os = "macos") {
                    "darwin"
                } else if cfg!(target_os = "linux") {
                    "linux"
                } else {
                    "unknown"
                }
                .into(),
            ))
        }),
    );

    let _ = m.set_prop(
        "arch",
        native("os.arch", 0, |_, _| {
            Ok(Value::String(
                if cfg!(target_arch = "x86_64") {
                    "x64"
                } else if cfg!(target_arch = "aarch64") {
                    "arm64"
                } else if cfg!(target_arch = "x86") {
                    "ia32"
                } else {
                    "unknown"
                }
                .into(),
            ))
        }),
    );

    let _ = m.set_prop(
        "hostname",
        native("os.hostname", 0, |_, _| {
            Ok(Value::String(
                hostname::get_hostname().unwrap_or_else(|| "localhost".into()),
            ))
        }),
    );

    let _ = m.set_prop(
        "homedir",
        native("os.homedir", 0, |_, _| {
            Ok(Value::String(
                dirs_home().unwrap_or_else(|| ".".into()),
            ))
        }),
    );

    let _ = m.set_prop(
        "tmpdir",
        native("os.tmpdir", 0, |_, _| {
            Ok(Value::String(std::env::temp_dir().to_string_lossy().into_owned()))
        }),
    );

    let _ = m.set_prop(
        "EOL",
        Value::String(if cfg!(windows) { "\r\n" } else { "\n" }.into()),
    );

    let _ = m.set_prop(
        "endianness",
        native("os.endianness", 0, |_, _| {
            Ok(Value::String(
                if cfg!(target_endian = "little") {
                    "LE"
                } else {
                    "BE"
                }
                .into(),
            ))
        }),
    );

    let _ = m.set_prop(
        "type",
        native("os.type", 0, |_, _| {
            Ok(Value::String(std::env::consts::OS.into()))
        }),
    );

    let _ = m.set_prop(
        "release",
        native("os.release", 0, |_, _| {
            Ok(Value::String(
                std::env::var("OS").unwrap_or_else(|_| std::env::consts::OS.into()),
            ))
        }),
    );

    let _ = m.set_prop(
        "cpus",
        native("os.cpus", 0, |_, _| {
            let n = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1);
            let mut arr = Vec::new();
            for _ in 0..n {
                let cpu = Value::empty_object();
                let _ = cpu.set_prop("model", Value::String("CPU".into()));
                let _ = cpu.set_prop("speed", Value::Number(0.0));
                arr.push(cpu);
            }
            Ok(Value::Array(std::rc::Rc::new(std::cell::RefCell::new(arr))))
        }),
    );

    let _ = m.set_prop(
        "totalmem",
        native("os.totalmem", 0, |_, _| Ok(Value::Number(0.0))),
    );
    let _ = m.set_prop(
        "freemem",
        native("os.freemem", 0, |_, _| Ok(Value::Number(0.0))),
    );

    m
}

fn dirs_home() -> Option<String> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(|p| p.to_string_lossy().into_owned())
}

mod hostname {
    pub fn get_hostname() -> Option<String> {
        std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .ok()
    }
}
