use crate::runtime::native;
use crate::value::Value;

pub fn create() -> Value {
    let m = Value::empty_object();

    let create_context = native("vm.createContext", 1, |_, args| {
        if let Some(Value::Object(o)) = args.first() {
            Ok(Value::Object(o.clone()))
        } else if let Some(v) = args.first() {
            Ok(v.clone())
        } else {
            Ok(Value::empty_object())
        }
    });

    let script_ctor = native("vm.Script", 1, |_, args| {
        let code = args.first().map(|v| v.as_string()).unwrap_or_default();
        let script = Value::empty_object();
        let _ = script.set_prop("_code", Value::String(code.clone()));
        let code2 = code;
        let _ = script.set_prop(
            "runInContext",
            native("Script.runInContext", 1, move |vm, args| {
                let sandbox = args.first().cloned();
                if let Some(Value::Object(sandbox)) = sandbox {
                    vm.push_scope_public();
                    for (k, v) in sandbox.borrow().iter() {
                        vm.define_public(k, v.clone());
                    }
                    vm.define_public("global", Value::Object(sandbox.clone()));
                    let result = vm.eval_source(&code2, "<vm.Script>");
                    let keys: Vec<String> = {
                        let last = vm.scopes_len() - 1;
                        vm.scope_keys(last)
                    };
                    for k in keys {
                        if k == "global" {
                            continue;
                        }
                        if let Some(val) = vm.get_local_public(&k) {
                            let _ = Value::Object(sandbox.clone()).set_prop(&k, val);
                        }
                    }
                    vm.pop_scope_public();
                    result
                } else {
                    vm.eval_source(&code2, "<vm.Script>")
                }
            }),
        );
        let code3 = script.get_prop("_code").as_string();
        let _ = script.set_prop(
            "runInThisContext",
            native("Script.runInThisContext", 0, move |vm, _| {
                vm.eval_source(&code3, "<vm.Script>")
            }),
        );
        Ok(script)
    });

    let script_ns = Value::empty_object();
    let _ = script_ns.set_prop("createContext", create_context.clone());

    let _ = m.set_prop("createContext", create_context);
    let _ = m.set_prop("Script", script_ctor);
    let _ = m.set_prop(
        "runInThisContext",
        native("vm.runInThisContext", 1, |vm, args| {
            let code = args.first().map(|v| v.as_string()).unwrap_or_default();
            vm.eval_source(&code, "<vm>")
        }),
    );
    let _ = m.set_prop(
        "runInNewContext",
        native("vm.runInNewContext", 2, |vm, args| {
            let code = args.first().map(|v| v.as_string()).unwrap_or_default();
            let sandbox = args
                .get(1)
                .cloned()
                .unwrap_or_else(Value::empty_object);
            if let Value::Object(sandbox) = sandbox {
                vm.push_scope_public();
                for (k, v) in sandbox.borrow().iter() {
                    vm.define_public(k, v.clone());
                }
                vm.define_public("global", Value::Object(sandbox.clone()));
                let result = vm.eval_source(&code, "<vm>");
                vm.pop_scope_public();
                result
            } else {
                vm.eval_source(&code, "<vm>")
            }
        }),
    );

    let _ = script_ns;
    m
}
