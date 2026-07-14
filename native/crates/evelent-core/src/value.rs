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
                    Value::Undefined
                }
            }
            Value::String(s) if key == "length" => Value::Number(s.len() as f64),
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
