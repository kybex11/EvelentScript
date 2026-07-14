use crate::runtime::native;
use crate::value::Value;

pub fn create() -> Value {
    let m = Value::empty_object();

    let _ = m.set_prop(
        "Readable",
        native("stream.Readable", 0, |_, _| Ok(make_readable())),
    );
    let _ = m.set_prop(
        "Writable",
        native("stream.Writable", 0, |_, _| Ok(make_writable())),
    );
    let _ = m.set_prop(
        "PassThrough",
        native("stream.PassThrough", 0, |_, _| Ok(make_readable())),
    );
    let _ = m.set_prop(
        "pipeline",
        native("stream.pipeline", 0, |_, _| Ok(Value::Undefined)),
    );
    m
}

fn make_readable() -> Value {
    let s = Value::empty_object();
    let _ = s.set_prop(
        "on",
        native("Readable.on", 2, |_, _| Ok(Value::Undefined)),
    );
    let _ = s.set_prop(
        "pipe",
        native("Readable.pipe", 1, |_, args| {
            Ok(args.first().cloned().unwrap_or(Value::Undefined))
        }),
    );
    let _ = s.set_prop(
        "read",
        native("Readable.read", 0, |_, _| Ok(Value::Null)),
    );
    s
}

fn make_writable() -> Value {
    let s = Value::empty_object();
    let _ = s.set_prop(
        "write",
        native("Writable.write", 1, |_, _| Ok(Value::Bool(true))),
    );
    let _ = s.set_prop(
        "end",
        native("Writable.end", 0, |_, _| Ok(Value::Undefined)),
    );
    let _ = s.set_prop(
        "on",
        native("Writable.on", 2, |_, _| Ok(Value::Undefined)),
    );
    s
}
