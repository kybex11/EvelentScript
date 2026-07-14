use std::process::Command;

use crate::error::{Error, Result};
use crate::runtime::native;
use crate::value::Value;

pub fn create() -> Value {
    let m = Value::empty_object();

    let _ = m.set_prop(
        "execSync",
        native("child_process.execSync", 2, |_, args| {
            let cmd = args
                .first()
                .map(|v| v.as_string())
                .ok_or_else(|| Error::Other("execSync needs command".into()))?;
            let output = if cfg!(windows) {
                Command::new("cmd").args(["/C", &cmd]).output()
            } else {
                Command::new("sh").args(["-c", &cmd]).output()
            }
            .map_err(|e| Error::Other(e.to_string()))?;
            if !output.status.success() {
                return Err(Error::Other(format!(
                    "Command failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }
            Ok(Value::String(
                String::from_utf8_lossy(&output.stdout).into_owned(),
            ))
        }),
    );

    let _ = m.set_prop(
        "spawnSync",
        native("child_process.spawnSync", 3, |_, args| {
            let program = args
                .first()
                .map(|v| v.as_string())
                .ok_or_else(|| Error::Other("spawnSync needs command".into()))?;
            let argv: Vec<String> = match args.get(1) {
                Some(Value::Array(a)) => a.borrow().iter().map(|v| v.as_string()).collect(),
                _ => vec![],
            };
            let output = Command::new(&program)
                .args(&argv)
                .output()
                .map_err(|e| Error::Other(e.to_string()))?;
            let result = Value::empty_object();
            let _ = result.set_prop(
                "status",
                Value::Number(output.status.code().unwrap_or(1) as f64),
            );
            let _ = result.set_prop(
                "stdout",
                Value::String(String::from_utf8_lossy(&output.stdout).into_owned()),
            );
            let _ = result.set_prop(
                "stderr",
                Value::String(String::from_utf8_lossy(&output.stderr).into_owned()),
            );
            let _ = result.set_prop("pid", Value::Number(0.0));
            Ok(result)
        }),
    );

    let _ = m.set_prop(
        "execFileSync",
        native("child_process.execFileSync", 2, |_, args| {
            let program = args
                .first()
                .map(|v| v.as_string())
                .ok_or_else(|| Error::Other("execFileSync needs file".into()))?;
            let argv: Vec<String> = match args.get(1) {
                Some(Value::Array(a)) => a.borrow().iter().map(|v| v.as_string()).collect(),
                _ => vec![],
            };
            let output = Command::new(&program)
                .args(&argv)
                .output()
                .map_err(|e| Error::Other(e.to_string()))?;
            if !output.status.success() {
                return Err(Error::Other(String::from_utf8_lossy(&output.stderr).into()));
            }
            Ok(Value::String(
                String::from_utf8_lossy(&output.stdout).into_owned(),
            ))
        }),
    );

    m
}

#[allow(dead_code)]
fn _r(_: Result<Value>) {}
