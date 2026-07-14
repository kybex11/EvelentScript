//! Native EvelentScript interpreter (runs AST in-process, no JavaScript).

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::ast::*;
use crate::error::{Error, Result};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::value::{FunctionValue, NativeFunction, Value};

#[derive(Debug)]
enum Flow {
    None(Value),
    Return(Value),
    Break,
    Continue,
}

pub struct Vm {
    /// Scope stack: globals at [0], innermost last.
    scopes: Vec<Rc<RefCell<HashMap<String, Value>>>>,
    /// Module cache: absolute path → exports object
    modules: HashMap<PathBuf, Value>,
    /// Current script directory (for relative require)
    dirname: PathBuf,
    /// Native plugin host (optional)
    native_dirs: Vec<PathBuf>,
    /// Extra roots for bare `require 'pkg'` (project root, evelent_modules, …)
    package_roots: Vec<PathBuf>,
}

impl Vm {
    pub fn new() -> Self {
        let mut vm = Self {
            scopes: vec![Rc::new(RefCell::new(HashMap::new()))],
            modules: HashMap::new(),
            dirname: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            native_dirs: vec![PathBuf::from("native-modules")],
            package_roots: Vec::new(),
        };
        vm.install_builtins();
        vm
    }

    pub fn with_native_dirs(mut self, dirs: Vec<PathBuf>) -> Self {
        self.native_dirs = dirs;
        self
    }

    pub fn with_package_roots(mut self, roots: Vec<PathBuf>) -> Self {
        self.package_roots = roots;
        self
    }

    fn install_builtins(&mut self) {
        let console = Value::empty_object();
        let log = native("log", 0, |_, args| {
            let parts: Vec<String> = args.iter().map(|a| a.to_string()).collect();
            println!("{}", parts.join(" "));
            Ok(Value::Undefined)
        });
        let _ = console.set_prop("log", log);
        self.set_global("console", console);

        self.set_global(
            "print",
            native("print", 0, |_, args| {
                let parts: Vec<String> = args.iter().map(|a| a.to_string()).collect();
                println!("{}", parts.join(" "));
                Ok(Value::Undefined)
            }),
        );

        // typeof as a simple helper: use via Ident isn't possible; we compile typeof? skip

        // Math
        let math = Value::empty_object();
        let _ = math.set_prop(
            "sqrt",
            native("sqrt", 1, |_, args| {
                Ok(Value::Number(args.first().map(|v| v.as_number().sqrt()).unwrap_or(f64::NAN)))
            }),
        );
        let _ = math.set_prop(
            "floor",
            native("floor", 1, |_, args| {
                Ok(Value::Number(
                    args.first().map(|v| v.as_number().floor()).unwrap_or(f64::NAN),
                ))
            }),
        );
        let _ = math.set_prop(
            "ceil",
            native("ceil", 1, |_, args| {
                Ok(Value::Number(
                    args.first().map(|v| v.as_number().ceil()).unwrap_or(f64::NAN),
                ))
            }),
        );
        let _ = math.set_prop(
            "abs",
            native("abs", 1, |_, args| {
                Ok(Value::Number(
                    args.first().map(|v| v.as_number().abs()).unwrap_or(f64::NAN),
                ))
            }),
        );
        self.set_global("Math", math);

        // global alias
        // (set after more builtins via pointer — use getter later)
        self.set_global(
            "parseInt",
            native("parseInt", 1, |_, args| {
                let s = args.first().map(|v| v.as_string()).unwrap_or_default();
                let n = s
                    .trim()
                    .chars()
                    .take_while(|c| c.is_ascii_digit() || *c == '-' || *c == '+')
                    .collect::<String>()
                    .parse::<f64>()
                    .unwrap_or(f64::NAN);
                Ok(Value::Number(n))
            }),
        );

        self.set_global(
            "parseFloat",
            native("parseFloat", 1, |_, args| {
                let s = args.first().map(|v| v.as_string()).unwrap_or_default();
                Ok(Value::Number(s.trim().parse().unwrap_or(f64::NAN)))
            }),
        );

        // require
        self.set_global(
            "require",
            native("require", 1, |vm, args| {
                let spec = args
                    .first()
                    .map(|v| v.as_string())
                    .ok_or_else(|| Error::Other("require() needs a module path".into()))?;
                vm.require_module(&spec)
            }),
        );

        // EvelentScript host API (native eval)
        let es = Value::empty_object();
        let _ = es.set_prop(
            "eval",
            native("EvelentScript.eval", 1, |vm, args| {
                let code = args
                    .first()
                    .map(|v| v.as_string())
                    .unwrap_or_default();
                let sandbox = args.get(1).and_then(|opts| match opts {
                    Value::Object(o) => o.borrow().get("sandbox").cloned(),
                    _ => None,
                });

                if let Some(Value::Object(sandbox)) = sandbox {
                    // Script-context style: mutable shared object
                    vm.push_scope();
                    for (k, v) in sandbox.borrow().iter() {
                        vm.define(k, v.clone());
                    }
                    vm.define("global", Value::Object(sandbox.clone()));
                    let result = vm.eval_source(&code, "<eval>");
                    // Sync defined names back onto sandbox
                    let keys: Vec<String> = {
                        let last = vm.scopes.len() - 1;
                        vm.scopes[last].borrow().keys().cloned().collect()
                    };
                    for k in keys {
                        if k == "global" {
                            continue;
                        }
                        if let Some(val) = vm.get_local(&k) {
                            let _ = Value::Object(sandbox.clone()).set_prop(&k, val);
                        }
                    }
                    // Also sync properties written via global.*
                    vm.pop_scope();
                    result
                } else if let Some(sandbox) = sandbox {
                    // Ordinary object: run with a *copy* so caller object is unchanged
                    vm.push_scope();
                    if let Value::Object(o) = &sandbox {
                        for (k, v) in o.borrow().iter() {
                            vm.define(k, v.clone());
                        }
                    }
                    let copy = Value::empty_object();
                    if let Value::Object(o) = &sandbox {
                        for (k, v) in o.borrow().iter() {
                            let _ = copy.set_prop(k, v.clone());
                        }
                    }
                    vm.define("global", copy);
                    let result = vm.eval_source(&code, "<eval>");
                    vm.pop_scope();
                    result
                } else {
                    let globals = Value::Object(vm.scopes[0].clone());
                    vm.define("global", globals);
                    vm.eval_source(&code, "<eval>")
                }
            }),
        );
        let _ = es.set_prop("VERSION", Value::String(env!("CARGO_PKG_VERSION").into()));
        self.set_global("EvelentScript", es);

        // process global (Node-like)
        if let Ok(Some(process)) = crate::builtins::load("process") {
            self.set_global("process", process);
        }

        // Point global at globals object for scripts that use `global.x`
        let globals_obj = Value::Object(self.scopes[0].clone());
        self.set_global("global", globals_obj);
    }

    pub fn set_global(&mut self, name: &str, value: Value) {
        self.scopes[0].borrow_mut().insert(name.to_string(), value);
    }

    pub(crate) fn define_public(&mut self, name: &str, value: Value) {
        self.define(name, value);
    }

    pub(crate) fn pop_scope_public(&mut self) {
        self.pop_scope();
    }

    pub(crate) fn scopes_len(&self) -> usize {
        self.scopes.len()
    }

    pub(crate) fn scope_keys(&self, idx: usize) -> Vec<String> {
        self.scopes
            .get(idx)
            .map(|s| s.borrow().keys().cloned().collect())
            .unwrap_or_default()
    }

    pub(crate) fn get_local_public(&self, name: &str) -> Option<Value> {
        self.get_local(name)
    }

    pub(crate) fn push_scope_public(&mut self) {
        self.push_scope();
    }

    fn push_scope(&mut self) {
        self.scopes.push(Rc::new(RefCell::new(HashMap::new())));
    }

    fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    fn define(&mut self, name: &str, value: Value) {
        let last = self.scopes.len() - 1;
        self.scopes[last]
            .borrow_mut()
            .insert(name.to_string(), value);
    }

    fn get_local(&self, name: &str) -> Option<Value> {
        let last = self.scopes.len() - 1;
        self.scopes[last].borrow().get(name).cloned()
    }

    fn resolve(&self, name: &str) -> Option<Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.borrow().get(name) {
                return Some(v.clone());
            }
        }
        None
    }

    fn assign_name(&mut self, name: &str, value: Value) -> Result<()> {
        for scope in self.scopes.iter().rev() {
            if scope.borrow().contains_key(name) {
                scope.borrow_mut().insert(name.to_string(), value);
                return Ok(());
            }
        }
        // declare in current scope (CoffeeScript-like)
        self.define(name, value);
        Ok(())
    }

    /// Parse and run a source string; returns last expression value.
    pub fn eval_source(&mut self, source: &str, path: &str) -> Result<Value> {
        let tokens = Lexer::new(source, path).tokenize()?;
        let program = Parser::new(tokens, path).parse()?;
        self.run_program(&program)
    }

    /// Load and run a `.es` file natively (as a CommonJS module: `exports` / `module`).
    pub fn run_file(&mut self, path: &Path) -> Result<Value> {
        let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if let Some(cached) = self.modules.get(&abs) {
            return Ok(cached.clone());
        }
        let source = std::fs::read_to_string(&abs)?;
        let prev_dir = self.dirname.clone();
        self.dirname = abs.parent().unwrap_or(Path::new(".")).to_path_buf();

        let exports = Value::empty_object();
        let module = Value::empty_object();
        let _ = module.set_prop("exports", exports.clone());
        self.push_scope();
        self.define("exports", exports.clone());
        self.define("module", module.clone());
        let last = self.eval_source(&source, &abs.display().to_string())?;
        let result = self
            .resolve("module")
            .map(|m| m.get_prop("exports"))
            .unwrap_or(exports);
        self.pop_scope();
        self.dirname = prev_dir;
        self.modules.insert(abs, result.clone());
        // Prefer last expression if exports is empty and last is meaningful
        if matches!(result, Value::Object(_)) {
            // still return exports for module scripts; fall back to last for scripts
            let empty = match &result {
                Value::Object(o) => o.borrow().is_empty(),
                _ => false,
            };
            if empty && !matches!(last, Value::Undefined) {
                return Ok(last);
            }
        }
        Ok(result)
    }

    pub fn run_program(&mut self, program: &Program) -> Result<Value> {
        let mut last = Value::Undefined;
        for stmt in &program.body {
            match self.exec_stmt(stmt)? {
                Flow::None(v) => {
                    if !matches!(v, Value::Undefined) {
                        last = v;
                    } else if let Stmt::Expr(_) = stmt {
                        last = v;
                    }
                }
                Flow::Return(v) => return Ok(v),
                Flow::Break => {
                    return Err(Error::Other("break outside loop".into()));
                }
                Flow::Continue => {
                    return Err(Error::Other("continue outside loop".into()));
                }
            }
        }
        Ok(last)
    }

    fn exec_block(&mut self, body: &[Stmt]) -> Result<Flow> {
        for stmt in body {
            match self.exec_stmt(stmt)? {
                Flow::None(_) => {}
                other => return Ok(other),
            }
        }
        Ok(Flow::None(Value::Undefined))
    }

    fn exec_stmt(&mut self, stmt: &Stmt) -> Result<Flow> {
        match stmt {
            Stmt::Expr(e) => Ok(Flow::None(self.eval_expr(e)?)),
            Stmt::Assign { target, value, op } => {
                let v = self.eval_expr(value)?;
                self.assign(target, v, *op)?;
                Ok(Flow::None(Value::Undefined))
            }
            Stmt::Return(None) => Ok(Flow::Return(Value::Undefined)),
            Stmt::Return(Some(e)) => Ok(Flow::Return(self.eval_expr(e)?)),
            Stmt::Break => Ok(Flow::Break),
            Stmt::Continue => Ok(Flow::Continue),
            Stmt::Throw(e) => {
                let v = self.eval_expr(e)?;
                Err(Error::Other(format!("uncaught: {v}")))
            }
            Stmt::If {
                test,
                body,
                else_body,
                inverted,
            } => {
                let mut ok = self.eval_expr(test)?.is_truthy();
                if *inverted {
                    ok = !ok;
                }
                if ok {
                    self.exec_block(body)
                } else if let Some(els) = else_body {
                    self.exec_block(els)
                } else {
                    Ok(Flow::None(Value::Undefined))
                }
            }
            Stmt::While {
                test,
                body,
                inverted,
            } => {
                loop {
                    let mut ok = self.eval_expr(test)?.is_truthy();
                    if *inverted {
                        ok = !ok;
                    }
                    if !ok {
                        break;
                    }
                    match self.exec_block(body)? {
                        Flow::None(_) | Flow::Continue => {}
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                    }
                }
                Ok(Flow::None(Value::Undefined))
            }
            Stmt::For {
                name,
                index,
                iter,
                body,
                ..
            } => {
                let it = self.eval_expr(iter)?;
                match it {
                    Value::Array(arr) => {
                        let items = arr.borrow().clone();
                        for (i, item) in items.into_iter().enumerate() {
                            self.define(name, item);
                            if let Some(idx) = index {
                                self.define(idx, Value::Number(i as f64));
                            }
                            match self.exec_block(body)? {
                                Flow::None(_) | Flow::Continue => {}
                                Flow::Break => break,
                                Flow::Return(v) => return Ok(Flow::Return(v)),
                            }
                        }
                    }
                    Value::Object(obj) => {
                        let keys: Vec<String> = obj.borrow().keys().cloned().collect();
                        for k in keys {
                            let val = obj.borrow().get(&k).cloned().unwrap_or(Value::Undefined);
                            self.define(name, val);
                            if let Some(idx) = index {
                                self.define(idx, Value::String(k));
                            }
                            match self.exec_block(body)? {
                                Flow::None(_) | Flow::Continue => {}
                                Flow::Break => break,
                                Flow::Return(v) => return Ok(Flow::Return(v)),
                            }
                        }
                    }
                    Value::String(s) => {
                        for (i, ch) in s.chars().enumerate() {
                            self.define(name, Value::String(ch.to_string()));
                            if let Some(idx) = index {
                                self.define(idx, Value::Number(i as f64));
                            }
                            match self.exec_block(body)? {
                                Flow::None(_) | Flow::Continue => {}
                                Flow::Break => break,
                                Flow::Return(v) => return Ok(Flow::Return(v)),
                            }
                        }
                    }
                    other => {
                        return Err(Error::Other(format!(
                            "for-in over {}",
                            other.type_name()
                        )));
                    }
                }
                Ok(Flow::None(Value::Undefined))
            }
            Stmt::Class { name, .. } => {
                self.define(name, Value::empty_object());
                Ok(Flow::None(Value::Undefined))
            }
            Stmt::Import(_) | Stmt::Export(_) => Ok(Flow::None(Value::Undefined)),
            Stmt::Try {
                body,
                catch,
                finally,
            } => {
                let result = self.exec_block(body);
                let flow = match result {
                    Ok(f) => f,
                    Err(e) => {
                        if let Some((binding, cbody)) = catch {
                            if let Some(name) = binding {
                                self.define(name, Value::String(e.to_string()));
                            }
                            self.exec_block(cbody)?
                        } else {
                            return Err(e);
                        }
                    }
                };
                if let Some(fbody) = finally {
                    let _ = self.exec_block(fbody)?;
                }
                Ok(flow)
            }
        }
    }

    fn eval_expr(&mut self, expr: &Expr) -> Result<Value> {
        match expr {
            Expr::Ident(n) => self
                .resolve(n)
                .ok_or_else(|| Error::Other(format!("undefined variable `{n}`"))),
            Expr::Number(n) => Ok(Value::Number(n.parse().unwrap_or(f64::NAN))),
            Expr::String(s) => {
                if let Some(rest) = s.strip_prefix("__TPL__") {
                    Ok(Value::String(self.expand_template(rest)?))
                } else {
                    Ok(Value::String(s.clone()))
                }
            }
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Null => Ok(Value::Null),
            Expr::Undefined => Ok(Value::Undefined),
            Expr::This => Ok(self.resolve("this").unwrap_or(Value::Undefined)),
            Expr::Array(els) => {
                let mut out = Vec::new();
                for e in els {
                    out.push(self.eval_expr(e)?);
                }
                Ok(Value::Array(Rc::new(RefCell::new(out))))
            }
            Expr::Object(props) => {
                let mut map = HashMap::new();
                for (k, v) in props {
                    let key = match k {
                        ObjectKey::Ident(s) | ObjectKey::String(s) => s.clone(),
                        ObjectKey::Computed(e) => self.eval_expr(e)?.as_string(),
                    };
                    map.insert(key, self.eval_expr(v)?);
                }
                Ok(Value::object_from(map))
            }
            Expr::Unary { op, arg } => {
                let v = self.eval_expr(arg)?;
                Ok(match op {
                    UnaryOp::Not => Value::Bool(!v.is_truthy()),
                    UnaryOp::Neg => Value::Number(-v.as_number()),
                    UnaryOp::Pos => Value::Number(v.as_number()),
                    UnaryOp::TypeOf => Value::String(v.type_name().into()),
                    UnaryOp::Delete => Value::Bool(true),
                })
            }
            Expr::Binary { op, left, right } => {
                let l = self.eval_expr(left)?;
                // short-circuit
                match op {
                    BinaryOp::And => {
                        return Ok(if l.is_truthy() {
                            self.eval_expr(right)?
                        } else {
                            l
                        });
                    }
                    BinaryOp::Or => {
                        return Ok(if l.is_truthy() {
                            l
                        } else {
                            self.eval_expr(right)?
                        });
                    }
                    _ => {}
                }
                let r = self.eval_expr(right)?;
                Ok(match op {
                    BinaryOp::Add => {
                        if matches!(l, Value::String(_)) || matches!(r, Value::String(_)) {
                            Value::String(format!("{}{}", l.as_string(), r.as_string()))
                        } else {
                            Value::Number(l.as_number() + r.as_number())
                        }
                    }
                    BinaryOp::Sub => Value::Number(l.as_number() - r.as_number()),
                    BinaryOp::Mul => Value::Number(l.as_number() * r.as_number()),
                    BinaryOp::Div => Value::Number(l.as_number() / r.as_number()),
                    BinaryOp::Mod => Value::Number(l.as_number() % r.as_number()),
                    BinaryOp::Eq => Value::Bool(l.equals(&r)),
                    BinaryOp::NotEq => Value::Bool(!l.equals(&r)),
                    BinaryOp::StrictEq => Value::Bool(l.strict_equals(&r)),
                    BinaryOp::StrictNotEq => Value::Bool(!l.strict_equals(&r)),
                    BinaryOp::Lt => Value::Bool(l.as_number() < r.as_number()),
                    BinaryOp::Gt => Value::Bool(l.as_number() > r.as_number()),
                    BinaryOp::LtEq => Value::Bool(l.as_number() <= r.as_number()),
                    BinaryOp::GtEq => Value::Bool(l.as_number() >= r.as_number()),
                    BinaryOp::And | BinaryOp::Or => unreachable!(),
                })
            }
            Expr::Call { callee, args } => {
                let f = self.eval_expr(callee)?;
                let mut argv = Vec::new();
                for a in args {
                    argv.push(self.eval_expr(a)?);
                }
                self.call_value(f, argv, None)
            }
            Expr::SoakedCall { callee, args } => {
                let f = self.eval_expr(callee)?;
                if !f.exists() {
                    return Ok(Value::Undefined);
                }
                let mut argv = Vec::new();
                for a in args {
                    argv.push(self.eval_expr(a)?);
                }
                self.call_value(f, argv, None)
            }
            Expr::Member {
                object,
                property,
                computed,
                optional,
            } => {
                let obj = self.eval_expr(object)?;
                if *optional && !obj.exists() {
                    return Ok(Value::Undefined);
                }
                let key = if *computed {
                    self.eval_expr(property)?.as_string()
                } else if let Expr::String(s) = &**property {
                    s.clone()
                } else {
                    self.eval_expr(property)?.as_string()
                };
                Ok(obj.get_prop(&key))
            }
            Expr::Func {
                params,
                body,
                bound,
                ..
            } => {
                let closure = Rc::new(RefCell::new(HashMap::new()));
                // capture current bindings shallowly
                for scope in &self.scopes {
                    for (k, v) in scope.borrow().iter() {
                        if !closure.borrow().contains_key(k) {
                            closure.borrow_mut().insert(k.clone(), v.clone());
                        }
                    }
                }
                let mut fv = FunctionValue {
                    params: params.clone(),
                    body: body.clone(),
                    bound_this: None,
                    closure,
                };
                if *bound {
                    if let Some(t) = self.resolve("this") {
                        fv.bound_this = Some(Box::new(t));
                    }
                }
                Ok(Value::Function(fv))
            }
            Expr::New { callee, args } => {
                let f = self.eval_expr(callee)?;
                let mut argv = Vec::new();
                for a in args {
                    argv.push(self.eval_expr(a)?);
                }
                let this = Value::empty_object();
                let _ = self.call_value(f, argv, Some(this.clone()))?;
                Ok(this)
            }
            Expr::Require(spec) => self.require_module(spec),
            Expr::Await(e) => self.eval_expr(e), // sync for now
            Expr::Existence(e) => Ok(Value::Bool(self.eval_expr(e)?.exists())),
            Expr::ExistentialDefault { value, default } => {
                let v = self.eval_expr(value)?;
                if v.exists() {
                    Ok(v)
                } else {
                    self.eval_expr(default)
                }
            }
            Expr::AssignExpr { target, value, op } => {
                let v = self.eval_expr(value)?;
                self.assign(target, v.clone(), *op)?;
                Ok(v)
            }
            Expr::IfExpr {
                test,
                then_branch,
                else_branch,
                inverted,
            } => {
                let mut ok = self.eval_expr(test)?.is_truthy();
                if *inverted {
                    ok = !ok;
                }
                if ok {
                    self.eval_expr(then_branch)
                } else if let Some(e) = else_branch {
                    self.eval_expr(e)
                } else {
                    Ok(Value::Undefined)
                }
            }
            Expr::Block(stmts) => {
                self.push_scope();
                let mut last = Value::Undefined;
                let result = (|| {
                    for stmt in stmts {
                        match self.exec_stmt(stmt)? {
                            Flow::None(v) => {
                                if let Stmt::Expr(_) = stmt {
                                    last = v;
                                }
                            }
                            Flow::Return(v) => {
                                last = v;
                                break;
                            }
                            Flow::Break | Flow::Continue => break,
                        }
                    }
                    Ok(last)
                })();
                self.pop_scope();
                result
            }
        }
    }

    fn expand_template(&mut self, s: &str) -> Result<String> {
        let mut out = String::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '$' && chars.peek() == Some(&'{') {
                chars.next();
                let mut expr = String::new();
                for c2 in chars.by_ref() {
                    if c2 == '}' {
                        break;
                    }
                    expr.push(c2);
                }
                // Evaluate simple identifier or dotted path in current scope
                let val = if let Some(v) = self.resolve(expr.trim()) {
                    v
                } else if expr.contains('.') {
                    // global.punctuation style
                    let mut parts = expr.split('.');
                    let first = parts.next().unwrap_or("");
                    let mut cur = self
                        .resolve(first)
                        .ok_or_else(|| Error::Other(format!("undefined `{first}` in template")))?;
                    for p in parts {
                        cur = cur.get_prop(p);
                    }
                    cur
                } else {
                    Value::Undefined
                };
                out.push_str(&val.as_string());
            } else {
                out.push(c);
            }
        }
        Ok(out)
    }

    fn assign(&mut self, target: &AssignTarget, value: Value, op: AssignOp) -> Result<()> {
        match target {
            AssignTarget::Ident(n) => {
                let new_val = if matches!(op, AssignOp::Eq | AssignOp::ColonEq) {
                    value
                } else {
                    let cur = self.resolve(n).unwrap_or(Value::Undefined);
                    apply_op(cur, value, op)?
                };
                self.assign_name(n, new_val)
            }
            AssignTarget::Member {
                object,
                property,
                computed,
            } => {
                let obj = self.eval_expr(object)?;
                let key = if *computed {
                    self.eval_expr(property)?.as_string()
                } else if let Expr::String(s) = &**property {
                    s.clone()
                } else {
                    self.eval_expr(property)?.as_string()
                };
                let new_val = if matches!(op, AssignOp::Eq | AssignOp::ColonEq) {
                    value
                } else {
                    apply_op(obj.get_prop(&key), value, op)?
                };
                obj.set_prop(&key, new_val)
            }
            AssignTarget::Array(targets) => {
                if let Value::Array(arr) = &value {
                    let items = arr.borrow();
                    for (i, t) in targets.iter().enumerate() {
                        if let Some(t) = t {
                            let v = items.get(i).cloned().unwrap_or(Value::Undefined);
                            self.assign(t, v, AssignOp::Eq)?;
                        }
                    }
                }
                Ok(())
            }
            AssignTarget::Object(_) => Err(Error::Other("object destructuring not yet supported in VM".into())),
        }
    }

    pub(crate) fn call_value_public(
        &mut self,
        callee: Value,
        args: Vec<Value>,
        this: Option<Value>,
    ) -> Result<Value> {
        self.call_value(callee, args, this)
    }

    fn call_value(&mut self, callee: Value, args: Vec<Value>, this: Option<Value>) -> Result<Value> {
        match callee {
            Value::Native(n) => (n.func)(self, &args),
            Value::Function(f) => {
                self.push_scope();
                // inject closure
                for (k, v) in f.closure.borrow().iter() {
                    if self.get_local(k).is_none() {
                        self.define(k, v.clone());
                    }
                }
                let this_val = f
                    .bound_this
                    .as_deref()
                    .cloned()
                    .or(this)
                    .unwrap_or(Value::Undefined);
                self.define("this", this_val);

                for (i, p) in f.params.iter().enumerate() {
                    if p.rest {
                        let rest: Vec<Value> = args.iter().skip(i).cloned().collect();
                        self.define(&p.name, Value::Array(Rc::new(RefCell::new(rest))));
                    } else {
                        let v = args.get(i).cloned().unwrap_or_else(|| {
                            p.default
                                .as_ref()
                                // can't eval default easily without recursive call — leave Undefined
                                .map(|_| Value::Undefined)
                                .unwrap_or(Value::Undefined)
                        });
                        // evaluate defaults when missing
                        let v = if matches!(v, Value::Undefined) {
                            if let Some(d) = &p.default {
                                self.eval_expr(d)?
                            } else {
                                Value::Undefined
                            }
                        } else {
                            v
                        };
                        self.define(&p.name, v);
                    }
                }

                let mut result = Value::Undefined;
                for stmt in &f.body {
                    match self.exec_stmt(stmt)? {
                        Flow::None(_) => {}
                        Flow::Return(v) => {
                            result = v;
                            break;
                        }
                        Flow::Break | Flow::Continue => {
                            self.pop_scope();
                            return Err(Error::Other("break/continue outside loop".into()));
                        }
                    }
                }
                self.pop_scope();
                Ok(result)
            }
            other => Err(Error::Other(format!(
                "{} is not a function",
                other.type_name()
            ))),
        }
    }

    fn require_module(&mut self, spec: &str) -> Result<Value> {
        if let Some(name) = spec.strip_prefix("native:") {
            return self.load_native_plugin(name);
        }

        // Node / node: builtins
        let builtin_name = spec.strip_prefix("node:").unwrap_or(spec);
        if let Some(module) = crate::builtins::load(builtin_name)? {
            return Ok(module);
        }

        // Bare package name → evelent_modules / package roots
        let is_relative = spec.starts_with('.') || spec.starts_with('/') || spec.contains('\\');
        if !is_relative {
            if let Some(lib) =
                crate::pkg::resolve_package_lib(spec, &self.dirname, &self.package_roots)
            {
                return self.load_es_module_file(&lib);
            }
        }

        let mut path = if is_relative {
            self.dirname.join(spec)
        } else {
            self.dirname.join(spec)
        };
        if path.extension().is_none() {
            let es = path.with_extension("es");
            if es.exists() {
                path = es;
            } else {
                let js = path.with_extension("js");
                if js.exists() {
                    return Err(Error::Other(format!(
                        "cannot require JS from native VM: {}",
                        js.display()
                    )));
                }
            }
        }

        let abs = path.canonicalize().unwrap_or(path.clone());
        if let Some(cached) = self.modules.get(&abs) {
            return Ok(cached.clone());
        }

        if !abs.exists() {
            return Err(Error::ModuleNotFound(spec.into()));
        }

        self.load_es_module_file(&abs)
    }

    fn load_es_module_file(&mut self, abs: &Path) -> Result<Value> {
        let abs = abs.canonicalize().unwrap_or_else(|_| abs.to_path_buf());
        if let Some(cached) = self.modules.get(&abs) {
            return Ok(cached.clone());
        }

        let source = std::fs::read_to_string(&abs)?;
        let prev_dir = self.dirname.clone();
        self.dirname = abs.parent().unwrap_or(Path::new(".")).to_path_buf();

        let exports = Value::empty_object();
        let module = Value::empty_object();
        let _ = module.set_prop("exports", exports.clone());
        self.push_scope();
        self.define("exports", exports.clone());
        self.define("module", module.clone());
        let tokens = Lexer::new(&source, abs.display().to_string()).tokenize()?;
        let program = Parser::new(tokens, abs.display().to_string()).parse()?;
        let _ = self.run_program(&program)?;
        let result = self
            .resolve("module")
            .map(|m| m.get_prop("exports"))
            .unwrap_or(exports);
        self.pop_scope();
        self.dirname = prev_dir;
        self.modules.insert(abs, result.clone());
        Ok(result)
    }

    fn load_native_plugin(&mut self, name: &str) -> Result<Value> {
        use crate::module::NativeHost;
        let mut host = NativeHost::new();
        for dir in &self.native_dirs {
            let _ = host.discover_dir(dir);
            let _ = host.discover_dir(&dir.join("target/debug"));
            let _ = host.discover_dir(&dir.join("target/release"));
        }
        let _ = host.discover_dir(Path::new("target/debug"));
        let _ = host.discover_dir(Path::new("target/release"));

        let list = host.list();
        let exports_names = list
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, e)| e.clone())
            .ok_or_else(|| Error::Native(format!("native module not found: {name}")))?;

        let obj = Value::empty_object();
        for export_name in exports_names {
            let export_name2 = export_name.clone();
            let mod_name = name.to_string();
            // Each call reloads via a temporary host — fine for MVP
            let f = native(&export_name, 0, move |vm, args| {
                let mut host = NativeHost::new();
                for dir in &vm.native_dirs {
                    let _ = host.discover_dir(dir);
                    let _ = host.discover_dir(&dir.join("target/debug"));
                    let _ = host.discover_dir(&dir.join("target/release"));
                }
                let json_args: Vec<serde_json::Value> = args
                    .iter()
                    .map(|v| match v {
                        Value::Null => serde_json::Value::Null,
                        Value::Undefined => serde_json::Value::Null,
                        Value::Bool(b) => serde_json::Value::Bool(*b),
                        Value::Number(n) => serde_json::json!(n),
                        Value::String(s) => serde_json::Value::String(s.clone()),
                        other => serde_json::Value::String(other.to_string()),
                    })
                    .collect();
                let result = host.call(&mod_name, &export_name2, &json_args)?;
                Ok(json_to_value(result))
            });
            let _ = obj.set_prop(&export_name, f);
        }
        Ok(obj)
    }

}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn native(
    name: &str,
    arity: usize,
    f: impl Fn(&mut Vm, &[Value]) -> Result<Value> + 'static,
) -> Value {
    Value::Native(NativeFunction {
        name: name.into(),
        arity,
        func: Rc::new(f),
    })
}

fn apply_op(left: Value, right: Value, op: AssignOp) -> Result<Value> {
    Ok(match op {
        AssignOp::Eq | AssignOp::ColonEq => right,
        AssignOp::PlusEq => {
            if matches!(left, Value::String(_)) || matches!(right, Value::String(_)) {
                Value::String(format!("{}{}", left.as_string(), right.as_string()))
            } else {
                Value::Number(left.as_number() + right.as_number())
            }
        }
        AssignOp::MinusEq => Value::Number(left.as_number() - right.as_number()),
        AssignOp::StarEq => Value::Number(left.as_number() * right.as_number()),
        AssignOp::SlashEq => Value::Number(left.as_number() / right.as_number()),
    })
}

fn json_to_value(v: serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(b),
        serde_json::Value::Number(n) => Value::Number(n.as_f64().unwrap_or(f64::NAN)),
        serde_json::Value::String(s) => Value::String(s),
        serde_json::Value::Array(a) => {
            Value::Array(Rc::new(RefCell::new(a.into_iter().map(json_to_value).collect())))
        }
        serde_json::Value::Object(o) => {
            let mut map = HashMap::new();
            for (k, v) in o {
                map.insert(k, json_to_value(v));
            }
            Value::object_from(map)
        }
    }
}

/// Convenience: parse + run a file.
pub fn run_file(path: &Path) -> Result<Value> {
    Vm::new().run_file(path)
}

/// Convenience: parse + run source.
pub fn run_source(source: &str, filename: &str) -> Result<Value> {
    Vm::new().eval_source(source, filename)
}
