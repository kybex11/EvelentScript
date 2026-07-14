use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::runtime::native;
use crate::value::Value;

pub fn create() -> Value {
    let m = Value::empty_object();
    let _ = m.set_prop("EventEmitter", native("EventEmitter", 0, |_, _| Ok(new_emitter())));
    // also export as default-ish
    let _ = m.set_prop(
        "once",
        native("events.once", 2, |_, args| {
            let emitter = args.first().cloned().unwrap_or(Value::empty_object());
            let event = args.get(1).map(|v| v.as_string()).unwrap_or_default();
            Ok(Value::Array(Rc::new(RefCell::new(vec![
                emitter,
                Value::String(event),
            ]))))
        }),
    );
    m
}

fn new_emitter() -> Value {
    let listeners: Rc<RefCell<HashMap<String, Vec<Value>>>> =
        Rc::new(RefCell::new(HashMap::new()));
    let ee = Value::empty_object();

    let list_on = listeners.clone();
    let _ = ee.set_prop(
        "on",
        native("EventEmitter.on", 2, move |_, args| {
            let event = args.first().map(|v| v.as_string()).unwrap_or_default();
            let cb = args.get(1).cloned().unwrap_or(Value::Undefined);
            list_on
                .borrow_mut()
                .entry(event)
                .or_default()
                .push(cb);
            Ok(Value::Undefined)
        }),
    );

    let list_once = listeners.clone();
    let _ = ee.set_prop(
        "once",
        native("EventEmitter.once", 2, move |_, args| {
            let event = args.first().map(|v| v.as_string()).unwrap_or_default();
            let cb = args.get(1).cloned().unwrap_or(Value::Undefined);
            // mark with a wrapper flag via object
            let wrap = Value::empty_object();
            let _ = wrap.set_prop("_once", Value::Bool(true));
            let _ = wrap.set_prop("_fn", cb);
            list_once
                .borrow_mut()
                .entry(event)
                .or_default()
                .push(wrap);
            Ok(Value::Undefined)
        }),
    );

    let list_off = listeners.clone();
    let _ = ee.set_prop(
        "off",
        native("EventEmitter.off", 2, move |_, args| {
            let event = args.first().map(|v| v.as_string()).unwrap_or_default();
            let cb = args.get(1).cloned();
            let mut map = list_off.borrow_mut();
            if let Some(vec) = map.get_mut(&event) {
                if let Some(cb) = cb {
                    vec.retain(|x| !x.strict_equals(&cb));
                } else {
                    vec.clear();
                }
            }
            Ok(Value::Undefined)
        }),
    );

    let list_emit = listeners.clone();
    let _ = ee.set_prop(
        "emit",
        native("EventEmitter.emit", 1, move |vm, args| {
            let event = args.first().map(|v| v.as_string()).unwrap_or_default();
            let payload: Vec<Value> = args.iter().skip(1).cloned().collect();
            let cbs = list_emit.borrow().get(&event).cloned().unwrap_or_default();
            let mut remove_once = Vec::new();
            for (i, cb) in cbs.iter().enumerate() {
                let (fn_v, is_once) = if let Value::Object(o) = cb {
                    let once = o
                        .borrow()
                        .get("_once")
                        .map(|v| v.is_truthy())
                        .unwrap_or(false);
                    let f = o
                        .borrow()
                        .get("_fn")
                        .cloned()
                        .unwrap_or(Value::Undefined);
                    (f, once)
                } else {
                    (cb.clone(), false)
                };
                let _ = vm.call_value_public(fn_v, payload.clone(), None)?;
                if is_once {
                    remove_once.push(i);
                }
            }
            if !remove_once.is_empty() {
                let mut map = list_emit.borrow_mut();
                if let Some(vec) = map.get_mut(&event) {
                    for i in remove_once.into_iter().rev() {
                        if i < vec.len() {
                            vec.remove(i);
                        }
                    }
                }
            }
            Ok(Value::Bool(!cbs.is_empty()))
        }),
    );

    let list_n = listeners;
    let _ = ee.set_prop(
        "listenerCount",
        native("EventEmitter.listenerCount", 1, move |_, args| {
            let event = args.first().map(|v| v.as_string()).unwrap_or_default();
            let n = list_n.borrow().get(&event).map(|v| v.len()).unwrap_or(0);
            Ok(Value::Number(n as f64))
        }),
    );

    ee
}
