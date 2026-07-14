use crate::error::{Error, Result};
use crate::runtime::native;
use crate::value::Value;

pub fn create(https: bool) -> Value {
    let m = Value::empty_object();

    let _ = m.set_prop(
        "getSync",
        native("http.getSync", 1, move |_, args| fetch(https, "GET", args)),
    );
    let _ = m.set_prop(
        "postSync",
        native("http.postSync", 2, move |_, args| fetch(https, "POST", args)),
    );
    let _ = m.set_prop(
        "requestSync",
        native("http.requestSync", 2, move |_, args| {
            let method = args
                .first()
                .and_then(|v| match v {
                    Value::Object(o) => o.borrow().get("method").map(|m| m.as_string()),
                    Value::String(s) if args.len() > 1 => Some("GET".into()),
                    _ => None,
                })
                .unwrap_or_else(|| "GET".into());
            fetch(https, &method, args)
        }),
    );

    // Node-ish aliases → sync implementations (VM has no event loop)
    let _ = m.set_prop(
        "get",
        native("http.get", 2, move |vm, args| {
            let result = fetch(https, "GET", args)?;
            if let Some(cb) = args.get(1) {
                if matches!(cb, Value::Function(_) | Value::Native(_)) {
                    let incoming = to_incoming(result)?;
                    let _ = vm.call_value_public(cb.clone(), vec![incoming], None)?;
                }
            }
            Ok(dummy_request())
        }),
    );

    let _ = m.set_prop(
        "request",
        native("http.request", 2, move |vm, args| {
            let method = args
                .first()
                .and_then(|v| match v {
                    Value::Object(o) => o.borrow().get("method").map(|m| m.as_string()),
                    _ => None,
                })
                .unwrap_or_else(|| "GET".into());
            let result = fetch(https, &method, args)?;
            if let Some(cb) = args.iter().find(|v| matches!(v, Value::Function(_) | Value::Native(_)))
            {
                let incoming = to_incoming(result)?;
                let _ = vm.call_value_public(cb.clone(), vec![incoming], None)?;
            }
            Ok(dummy_request())
        }),
    );

    let _ = m.set_prop(
        "createServer",
        native("http.createServer", 1, |_, _| {
            Err(Error::Other(
                "http.createServer is not supported in the sync VM".into(),
            ))
        }),
    );

    m
}

fn ensure_scheme(https: bool, url: &str) -> String {
    if url.contains("://") {
        url.into()
    } else if https {
        format!("https://{url}")
    } else {
        format!("http://{url}")
    }
}

fn url_from_args(args: &[Value]) -> Result<String> {
    match args.first() {
        Some(Value::String(s)) => Ok(s.clone()),
        Some(Value::Object(o)) => {
            let map = o.borrow();
            if let Some(u) = map.get("href").or_else(|| map.get("url")) {
                return Ok(u.as_string());
            }
            let host = map
                .get("hostname")
                .or_else(|| map.get("host"))
                .map(|v| v.as_string())
                .unwrap_or_default();
            let path = map
                .get("path")
                .or_else(|| map.get("pathname"))
                .map(|v| v.as_string())
                .unwrap_or_else(|| "/".into());
            let port = map.get("port").map(|v| v.as_string()).unwrap_or_default();
            if port.is_empty() || host.contains(':') {
                Ok(format!("{host}{path}"))
            } else {
                Ok(format!("{host}:{port}{path}"))
            }
        }
        _ => Err(Error::Other("url required".into())),
    }
}

fn fetch(https: bool, method: &str, args: &[Value]) -> Result<Value> {
    let url = ensure_scheme(https, &url_from_args(args)?);
    let method = method.to_uppercase();
    let body = args.get(1).and_then(|v| match v {
        Value::String(s) => Some(s.clone()),
        Value::Object(_) => None,
        other => Some(other.as_string()),
    });

    let resp = match method.as_str() {
        "POST" => {
            let mut req = ureq::post(&url);
            if let Some(b) = body {
                req = req.set("Content-Type", "text/plain");
                req.send_string(&b)
            } else {
                req.call()
            }
        }
        "PUT" => ureq::put(&url).call(),
        "DELETE" => ureq::delete(&url).call(),
        "HEAD" => ureq::head(&url).call(),
        _ => ureq::get(&url).call(),
    }
    .map_err(|e| Error::Other(e.to_string()))?;

    let status = resp.status();
    let text = resp
        .into_string()
        .map_err(|e| Error::Other(e.to_string()))?;
    let obj = Value::empty_object();
    let _ = obj.set_prop("statusCode", Value::Number(status as f64));
    let _ = obj.set_prop("body", Value::String(text));
    Ok(obj)
}

fn to_incoming(result: Value) -> Result<Value> {
    let status = result.get_prop("statusCode");
    let body = result.get_prop("body").as_string();
    let incoming = Value::empty_object();
    let _ = incoming.set_prop("statusCode", status);
    let _ = incoming.set_prop(
        "on",
        native("IncomingMessage.on", 2, move |vm, args| {
            let event = args.first().map(|v| v.as_string()).unwrap_or_default();
            let cb = args.get(1).cloned().unwrap_or(Value::Undefined);
            if event == "data" {
                let _ = vm.call_value_public(cb, vec![Value::String(body.clone())], None)?;
            } else if event == "end" {
                let _ = vm.call_value_public(cb, vec![], None)?;
            }
            Ok(Value::Undefined)
        }),
    );
    Ok(incoming)
}

fn dummy_request() -> Value {
    let req = Value::empty_object();
    let _ = req.set_prop(
        "on",
        native("ClientRequest.on", 2, |_, _| Ok(Value::Undefined)),
    );
    let _ = req.set_prop(
        "end",
        native("ClientRequest.end", 0, |_, _| Ok(Value::Undefined)),
    );
    let _ = req.set_prop(
        "write",
        native("ClientRequest.write", 1, |_, _| Ok(Value::Bool(true))),
    );
    req
}
