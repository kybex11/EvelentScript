use crate::runtime::native;
use crate::value::Value;

pub fn create() -> Value {
    let m = Value::empty_object();

    let _ = m.set_prop(
        "format",
        native("util.format", 0, |_, args| {
            if args.is_empty() {
                return Ok(Value::String(String::new()));
            }
            let mut fmt = args[0].as_string();
            let mut i = 1;
            while let Some(pos) = fmt.find('%') {
                if pos + 1 >= fmt.len() {
                    break;
                }
                let code = fmt.as_bytes()[pos + 1] as char;
                let repl = match code {
                    's' | 'd' | 'i' | 'f' | 'j' | 'o' | 'O' => {
                        let v = args.get(i).map(|v| v.to_string()).unwrap_or_default();
                        i += 1;
                        v
                    }
                    '%' => "%".into(),
                    _ => continue,
                };
                fmt.replace_range(pos..pos + 2, &repl);
            }
            for extra in args.iter().skip(i) {
                fmt.push(' ');
                fmt.push_str(&extra.to_string());
            }
            Ok(Value::String(fmt))
        }),
    );

    let _ = m.set_prop(
        "inspect",
        native("util.inspect", 1, |_, args| {
            Ok(Value::String(
                args.first().map(|v| format!("{v:?}")).unwrap_or_default(),
            ))
        }),
    );

    let _ = m.set_prop(
        "isArray",
        native("util.isArray", 1, |_, args| {
            Ok(Value::Bool(matches!(args.first(), Some(Value::Array(_)))))
        }),
    );

    let _ = m.set_prop(
        "isDate",
        native("util.isDate", 1, |_, _| Ok(Value::Bool(false))),
    );
    let _ = m.set_prop(
        "isRegExp",
        native("util.isRegExp", 1, |_, _| Ok(Value::Bool(false))),
    );
    let _ = m.set_prop(
        "isPrimitive",
        native("util.isPrimitive", 1, |_, args| {
            Ok(Value::Bool(matches!(
                args.first(),
                Some(
                    Value::Null
                        | Value::Undefined
                        | Value::Bool(_)
                        | Value::Number(_)
                        | Value::String(_)
                ) | None
            )))
        }),
    );

    let _ = m.set_prop(
        "promisify",
        native("util.promisify", 1, |_, args| {
            // Sync VM: return the function unchanged
            Ok(args.first().cloned().unwrap_or(Value::Undefined))
        }),
    );

    let types = Value::empty_object();
    let _ = types.set_prop(
        "isArrayBuffer",
        native("types.isArrayBuffer", 1, |_, _| Ok(Value::Bool(false))),
    );
    let _ = m.set_prop("types", types);

    m
}
