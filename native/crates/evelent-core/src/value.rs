//! Runtime values for the native EvelentScript interpreter.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use crate::ast::{Param, Stmt};

#[derive(Clone)]
pub enum Value {
    Null,
    Undefined,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Rc<RefCell<Vec<Value>>>),
    Object(Rc<RefCell<HashMap<String, Value>>>),
    Function(FunctionValue),
    Native(NativeFunction),
}

#[derive(Clone)]
pub struct FunctionValue {
    pub params: Vec<Param>,
    pub body: Vec<Stmt>,
    pub bound_this: Option<Box<Value>>,
    /// Captured environment (shallow map of names → values at definition time + live links).
    pub closure: Rc<RefCell<HashMap<String, Value>>>,
}

#[derive(Clone)]
pub struct NativeFunction {
    pub name: String,
    pub arity: usize,
    pub func: Rc<dyn Fn(&mut crate::runtime::Vm, &[Value]) -> crate::error::Result<Value>>,
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Undefined => write!(f, "undefined"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Number(n) => write!(f, "{n}"),
            Value::String(s) => write!(f, "{s:?}"),
            Value::Array(a) => write!(f, "Array({})", a.borrow().len()),
            Value::Object(o) => write!(f, "Object({{{}}} )", o.borrow().len()),
            Value::Function(_) => write!(f, "[Function]"),
            Value::Native(n) => write!(f, "[Native:{}]", n.name),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Undefined => write!(f, "undefined"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Number(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{n}")
                }
            }
            Value::String(s) => write!(f, "{s}"),
            Value::Array(a) => {
                let parts: Vec<String> = a.borrow().iter().map(|v| v.to_string()).collect();
                write!(f, "[{}]", parts.join(", "))
            }
            Value::Object(o) => {
                let parts: Vec<String> = o
                    .borrow()
                    .iter()
                    .map(|(k, v)| format!("{k}: {v}"))
                    .collect();
                write!(f, "{{{}}}", parts.join(", "))
            }
            Value::Function(_) => write!(f, "[Function]"),
            Value::Native(n) => write!(f, "[Native {}]", n.name),
        }
    }
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null | Value::Undefined => false,
            Value::Bool(b) => *b,
            Value::Number(n) => *n != 0.0 && !n.is_nan(),
            Value::String(s) => !s.is_empty(),
            _ => true,
        }
    }

    pub fn exists(&self) -> bool {
        !matches!(self, Value::Null | Value::Undefined)
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Undefined => "undefined",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
            Value::Function(_) | Value::Native(_) => "function",
        }
    }

    pub fn as_number(&self) -> f64 {
        match self {
            Value::Number(n) => *n,
            Value::Bool(true) => 1.0,
            Value::Bool(false) | Value::Null | Value::Undefined => 0.0,
            Value::String(s) => s.parse().unwrap_or(f64::NAN),
            _ => f64::NAN,
        }
    }

    pub fn as_string(&self) -> String {
        self.to_string()
    }

    pub fn equals(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) | (Value::Undefined, Value::Undefined) => true,
            (Value::Null, Value::Undefined) | (Value::Undefined, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => Rc::ptr_eq(a, b),
            (Value::Object(a), Value::Object(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }

    pub fn strict_equals(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Undefined, Value::Undefined) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => Rc::ptr_eq(a, b),
            (Value::Object(a), Value::Object(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }

    pub fn get_prop(&self, key: &str) -> Value {
        match self {
            Value::Object(o) => o.borrow().get(key).cloned().unwrap_or(Value::Undefined),
            Value::Array(a) => {
                if key == "length" {
                    Value::Number(a.borrow().len() as f64)
                } else if let Ok(i) = key.parse::<usize>() {
                    a.borrow().get(i).cloned().unwrap_or(Value::Undefined)
                } else {
                    array_method(a, key)
                }
            }
            Value::String(s) => {
                if key == "length" {
                    Value::Number(s.chars().count() as f64)
                } else if let Ok(i) = key.parse::<usize>() {
                    s.chars()
                        .nth(i)
                        .map(|c| Value::String(c.to_string()))
                        .unwrap_or(Value::Undefined)
                } else {
                    string_method(s, key)
                }
            }
            _ => Value::Undefined,
        }
    }

    pub fn set_prop(&self, key: &str, value: Value) -> crate::error::Result<()> {
        match self {
            Value::Object(o) => {
                o.borrow_mut().insert(key.to_string(), value);
                Ok(())
            }
            Value::Array(a) => {
                if let Ok(i) = key.parse::<usize>() {
                    let mut arr = a.borrow_mut();
                    if i >= arr.len() {
                        arr.resize(i + 1, Value::Undefined);
                    }
                    arr[i] = value;
                    Ok(())
                } else {
                    Err(crate::error::Error::Other(format!(
                        "cannot set property {key} on array"
                    )))
                }
            }
            _ => Err(crate::error::Error::Other(format!(
                "cannot set property {key} on {}",
                self.type_name()
            ))),
        }
    }

    pub fn empty_object() -> Value {
        Value::Object(Rc::new(RefCell::new(HashMap::new())))
    }

    pub fn object_from(map: HashMap<String, Value>) -> Value {
        Value::Object(Rc::new(RefCell::new(map)))
    }
}

fn array_method(a: &Rc<RefCell<Vec<Value>>>, key: &str) -> Value {
    match key {
        "push" => {
            let arr = a.clone();
            native_fn("Array.push", 0, move |_, args| {
                let mut borrow = arr.borrow_mut();
                for arg in args {
                    borrow.push(arg.clone());
                }
                Ok(Value::Number(borrow.len() as f64))
            })
        }
        "pop" => {
            let arr = a.clone();
            native_fn("Array.pop", 0, move |_, _| {
                Ok(arr.borrow_mut().pop().unwrap_or(Value::Undefined))
            })
        }
        "slice" => {
            let arr = a.clone();
            native_fn("Array.slice", 0, move |_, args| {
                let list = arr.borrow();
                let len = list.len() as i64;
                let start = args.first().map(|v| v.as_number() as i64).unwrap_or(0);
                let end = args
                    .get(1)
                    .map(|v| v.as_number() as i64)
                    .unwrap_or(len);
                let start = normalize_index(start, len);
                let end = normalize_index(end, len).max(start);
                let sliced: Vec<Value> = list[start..end].to_vec();
                Ok(Value::Array(Rc::new(RefCell::new(sliced))))
            })
        }
        "indexOf" => {
            let arr = a.clone();
            native_fn("Array.indexOf", 1, move |_, args| {
                let needle = args.first().cloned().unwrap_or(Value::Undefined);
                let list = arr.borrow();
                for (i, item) in list.iter().enumerate() {
                    if item.strict_equals(&needle) {
                        return Ok(Value::Number(i as f64));
                    }
                }
                Ok(Value::Number(-1.0))
            })
        }
        "join" => {
            let arr = a.clone();
            native_fn("Array.join", 0, move |_, args| {
                let sep = args
                    .first()
                    .map(|v| v.as_string())
                    .unwrap_or_else(|| ",".into());
                let parts: Vec<String> = arr.borrow().iter().map(|v| v.as_string()).collect();
                Ok(Value::String(parts.join(&sep)))
            })
        }
        "shift" => {
            let arr = a.clone();
            native_fn("Array.shift", 0, move |_, _| {
                let mut borrow = arr.borrow_mut();
                if borrow.is_empty() {
                    Ok(Value::Undefined)
                } else {
                    Ok(borrow.remove(0))
                }
            })
        }
        _ => Value::Undefined,
    }
}

fn string_method(s: &str, key: &str) -> Value {
    match key {
        "charAt" => {
            let owned = s.to_string();
            native_fn("String.charAt", 1, move |_, args| {
                let i = args.first().map(|v| v.as_number() as usize).unwrap_or(0);
                Ok(owned
                    .chars()
                    .nth(i)
                    .map(|c| Value::String(c.to_string()))
                    .unwrap_or_else(|| Value::String(String::new())))
            })
        }
        "slice" => {
            let owned = s.to_string();
            native_fn("String.slice", 0, move |_, args| {
                let chars: Vec<char> = owned.chars().collect();
                let len = chars.len() as i64;
                let start = args.first().map(|v| v.as_number() as i64).unwrap_or(0);
                let end = args.get(1).map(|v| v.as_number() as i64).unwrap_or(len);
                let start = normalize_index(start, len);
                let end = normalize_index(end, len).max(start);
                Ok(Value::String(chars[start..end].iter().collect()))
            })
        }
        "indexOf" => {
            let owned = s.to_string();
            native_fn("String.indexOf", 1, move |_, args| {
                let needle = args.first().map(|v| v.as_string()).unwrap_or_default();
                Ok(Value::Number(
                    owned.find(&needle).map(|i| i as f64).unwrap_or(-1.0),
                ))
            })
        }
        "toLowerCase" => {
            let owned = s.to_string();
            native_fn("String.toLowerCase", 0, move |_, _| {
                Ok(Value::String(owned.to_lowercase()))
            })
        }
        "toUpperCase" => {
            let owned = s.to_string();
            native_fn("String.toUpperCase", 0, move |_, _| {
                Ok(Value::String(owned.to_uppercase()))
            })
        }
        "trim" => {
            let owned = s.to_string();
            native_fn("String.trim", 0, move |_, _| {
                Ok(Value::String(owned.trim().to_string()))
            })
        }
        "split" => {
            let owned = s.to_string();
            native_fn("String.split", 1, move |_, args| {
                let sep = args.first().map(|v| v.as_string()).unwrap_or_default();
                let parts: Vec<Value> = if sep.is_empty() {
                    owned
                        .chars()
                        .map(|c| Value::String(c.to_string()))
                        .collect()
                } else {
                    owned
                        .split(&sep)
                        .map(|p| Value::String(p.to_string()))
                        .collect()
                };
                Ok(Value::Array(Rc::new(RefCell::new(parts))))
            })
        }
        "replace" => {
            let owned = s.to_string();
            native_fn("String.replace", 2, move |_, args| {
                let from = args.first().map(|v| v.as_string()).unwrap_or_default();
                let to = args.get(1).map(|v| v.as_string()).unwrap_or_default();
                Ok(Value::String(owned.replacen(&from, &to, 1)))
            })
        }
        _ => Value::Undefined,
    }
}

fn normalize_index(idx: i64, len: i64) -> usize {
    let i = if idx < 0 { len + idx } else { idx };
    i.clamp(0, len) as usize
}

fn native_fn(
    name: &str,
    arity: usize,
    func: impl Fn(&mut crate::runtime::Vm, &[Value]) -> crate::error::Result<Value> + 'static,
) -> Value {
    Value::Native(NativeFunction {
        name: name.into(),
        arity,
        func: Rc::new(func),
    })
}
