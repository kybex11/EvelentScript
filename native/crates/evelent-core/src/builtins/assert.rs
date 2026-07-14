use crate::error::{Error, Result};
use crate::runtime::native;
use crate::value::Value;

pub fn create() -> Value {
    let m = Value::empty_object();

    let _ = m.set_prop(
        "ok",
        native("assert.ok", 2, |_, args| {
            let v = args.first().cloned().unwrap_or(Value::Undefined);
            if !v.is_truthy() {
                let msg = args
                    .get(1)
                    .map(|v| v.as_string())
                    .unwrap_or_else(|| "assertion failed".into());
                return Err(Error::Other(msg));
            }
            Ok(Value::Undefined)
        }),
    );

    let _ = m.set_prop(
        "equal",
        native("assert.equal", 3, |_, args| {
            let a = args.first().cloned().unwrap_or(Value::Undefined);
            let b = args.get(1).cloned().unwrap_or(Value::Undefined);
            if !a.equals(&b) {
                let msg = args.get(2).map(|v| v.as_string()).unwrap_or_else(|| {
                    format!("expected {} == {}", a, b)
                });
                return Err(Error::Other(msg));
            }
            Ok(Value::Undefined)
        }),
    );

    let _ = m.set_prop(
        "strictEqual",
        native("assert.strictEqual", 3, |_, args| {
            let a = args.first().cloned().unwrap_or(Value::Undefined);
            let b = args.get(1).cloned().unwrap_or(Value::Undefined);
            if !a.strict_equals(&b) {
                let msg = args.get(2).map(|v| v.as_string()).unwrap_or_else(|| {
                    format!("expected {} === {}", a, b)
                });
                return Err(Error::Other(msg));
            }
            Ok(Value::Undefined)
        }),
    );

    let _ = m.set_prop(
        "notEqual",
        native("assert.notEqual", 3, |_, args| {
            let a = args.first().cloned().unwrap_or(Value::Undefined);
            let b = args.get(1).cloned().unwrap_or(Value::Undefined);
            if a.equals(&b) {
                return Err(Error::Other("expected values to differ".into()));
            }
            Ok(Value::Undefined)
        }),
    );

    let _ = m.set_prop(
        "deepEqual",
        native("assert.deepEqual", 3, |_, args| {
            let a = args.first().cloned().unwrap_or(Value::Undefined);
            let b = args.get(1).cloned().unwrap_or(Value::Undefined);
            if !deep_eq(&a, &b) {
                return Err(Error::Other(format!("deepEqual failed: {} vs {}", a, b)));
            }
            Ok(Value::Undefined)
        }),
    );

    let _ = m.set_prop(
        "throws",
        native("assert.throws", 1, |vm, args| {
            let fn_v = args.first().cloned().ok_or_else(|| Error::Other("throws needs fn".into()))?;
            match vm.call_value_public(fn_v, vec![], None) {
                Ok(_) => Err(Error::Other("expected function to throw".into())),
                Err(_) => Ok(Value::Undefined),
            }
        }),
    );

    // default export style: assert(value)
    let _ = m.set_prop(
        "default",
        native("assert", 2, |_, args| {
            let v = args.first().cloned().unwrap_or(Value::Undefined);
            if !v.is_truthy() {
                return Err(Error::Other(
                    args.get(1)
                        .map(|v| v.as_string())
                        .unwrap_or_else(|| "assertion failed".into()),
                ));
            }
            Ok(Value::Undefined)
        }),
    );

    m
}

fn deep_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Array(x), Value::Array(y)) => {
            let x = x.borrow();
            let y = y.borrow();
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(a, b)| deep_eq(a, b))
        }
        (Value::Object(x), Value::Object(y)) => {
            let x = x.borrow();
            let y = y.borrow();
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).map(|o| deep_eq(v, o)).unwrap_or(false))
        }
        _ => a.strict_equals(b) || a.equals(b),
    }
}

#[allow(dead_code)]
fn _unused_result(_: Result<Value>) {}
