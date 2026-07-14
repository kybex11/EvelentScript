use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use evelent_native::{evelent_native_string_free, NativeInitFn, NativeFn, ABI_VERSION};
use libloading::Library;

use crate::ast::{ImportDecl, Program, Stmt};
use crate::codegen::Codegen;
use crate::error::{Error, Result};
use crate::lexer::Lexer;
use crate::parser::Parser;

#[derive(Debug, Clone)]
pub struct CompileOptions {
    pub bare: bool,
    pub native_dirs: Vec<PathBuf>,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            bare: true,
            native_dirs: vec![PathBuf::from("native-modules")],
        }
    }
}

#[derive(Debug)]
pub struct CompiledModule {
    pub path: PathBuf,
    pub source: String,
    pub js: String,
    pub program: Program,
    pub dependencies: Vec<String>,
}

#[derive(Default)]
pub struct NativeHost {
    libs: Vec<Library>,
    modules: HashMap<String, HashMap<String, NativeFn>>,
}

impl NativeHost {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_file(&mut self, path: &Path, alias: Option<&str>) -> Result<String> {
        unsafe {
            let lib = Library::new(path).map_err(|e| {
                Error::Native(format!("failed to load {}: {e}", path.display()))
            })?;
            let init: libloading::Symbol<NativeInitFn> = lib
                .get(b"evelent_native_init")
                .map_err(|e| Error::Native(format!("missing evelent_native_init: {e}")))?;
            let info = &*init();
            if info.abi_version != ABI_VERSION {
                return Err(Error::Native(format!(
                    "ABI mismatch: got {}, expected {ABI_VERSION}",
                    info.abi_version
                )));
            }
            let mod_name = if info.name.is_null() {
                "anonymous".into()
            } else {
                std::ffi::CStr::from_ptr(info.name)
                    .to_string_lossy()
                    .into_owned()
            };
            let key = alias.unwrap_or(&mod_name).to_string();
            let mut exports = HashMap::new();
            if !info.exports.is_null() {
                let slice = std::slice::from_raw_parts(info.exports, info.export_count);
                for exp in slice {
                    if exp.name.is_null() {
                        continue;
                    }
                    let n = std::ffi::CStr::from_ptr(exp.name)
                        .to_string_lossy()
                        .into_owned();
                    exports.insert(n, exp.func);
                }
            }
            self.modules.insert(key.clone(), exports);
            self.libs.push(lib);
            Ok(key)
        }
    }

    pub fn discover_dir(&mut self, dir: &Path) -> Result<Vec<String>> {
        let mut loaded = Vec::new();
        if !dir.exists() {
            return Ok(loaded);
        }
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            let is_lib = matches!(ext, "dll" | "so" | "dylib")
                || path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib") && n.contains("hello"))
                    .unwrap_or(false);
            if is_lib {
                match self.load_file(&path, None) {
                    Ok(name) => loaded.push(name),
                    Err(e) => {
                        // Also try target/release / debug layouts
                        eprintln!("warning: {}", e);
                    }
                }
            }
        }
        Ok(loaded)
    }

    pub fn call(
        &self,
        module: &str,
        export: &str,
        args: &[serde_json::Value],
    ) -> Result<serde_json::Value> {
        let exports = self
            .modules
            .get(module)
            .ok_or_else(|| Error::Native(format!("native module not loaded: {module}")))?;
        let f = exports
            .get(export)
            .ok_or_else(|| Error::Native(format!("export not found: {module}.{export}")))?;
        let argv = serde_json::Value::Array(args.to_vec()).to_string();
        let c_argv = std::ffi::CString::new(argv)
            .map_err(|e| Error::Native(e.to_string()))?;
        unsafe {
            let raw = f(c_argv.as_ptr());
            if raw.is_null() {
                return Ok(serde_json::Value::Null);
            }
            let out = std::ffi::CStr::from_ptr(raw)
                .to_string_lossy()
                .into_owned();
            evelent_native_string_free(raw);
            serde_json::from_str(&out).map_err(|e| Error::Native(format!("bad return json: {e}")))
        }
    }

    pub fn list(&self) -> Vec<(String, Vec<String>)> {
        let mut out: Vec<_> = self
            .modules
            .iter()
            .map(|(k, v)| {
                let mut names: Vec<_> = v.keys().cloned().collect();
                names.sort();
                (k.clone(), names)
            })
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }
}

pub struct ModuleGraph {
    pub modules: HashMap<PathBuf, CompiledModule>,
    pub order: Vec<PathBuf>,
}

pub fn compile_source(source: &str, path: &str, opts: &CompileOptions) -> Result<CompiledModule> {
    let tokens = Lexer::new(source, path).tokenize()?;
    let program = Parser::new(tokens, path).parse()?;
    let deps = collect_deps(&program);
    let mut cg = Codegen::new(opts.bare);
    let js = cg.emit_program(&program);
    Ok(CompiledModule {
        path: PathBuf::from(path),
        source: source.to_string(),
        js,
        program,
        dependencies: deps,
    })
}

pub fn compile_file(path: &Path, opts: &CompileOptions) -> Result<CompiledModule> {
    let source = std::fs::read_to_string(path)?;
    compile_source(&source, &path.display().to_string(), opts)
}

pub fn compile_graph(entry: &Path, opts: &CompileOptions) -> Result<ModuleGraph> {
    let mut graph = ModuleGraph {
        modules: HashMap::new(),
        order: Vec::new(),
    };
    let mut visiting = HashSet::new();
    compile_recursive(entry, opts, &mut graph, &mut visiting)?;
    Ok(graph)
}

fn compile_recursive(
    path: &Path,
    opts: &CompileOptions,
    graph: &mut ModuleGraph,
    visiting: &mut HashSet<PathBuf>,
) -> Result<()> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if graph.modules.contains_key(&canonical) {
        return Ok(());
    }
    if !visiting.insert(canonical.clone()) {
        return Err(Error::Other(format!(
            "circular dependency involving {}",
            path.display()
        )));
    }

    let compiled = compile_file(path, opts)?;
    let deps = compiled.dependencies.clone();
    let base = path.parent().unwrap_or_else(|| Path::new("."));

    for dep in &deps {
        if dep.starts_with("native:") {
            continue;
        }
        // Only resolve relative / local .es modules into the graph
        if dep.starts_with('.') || dep.ends_with(".es") || dep.ends_with(".js") {
            let resolved = resolve_module(base, dep)?;
            compile_recursive(&resolved, opts, graph, visiting)?;
        }
    }

    graph.order.push(canonical.clone());
    graph.modules.insert(canonical, compiled);
    visiting.remove(path);
    Ok(())
}

pub fn resolve_module(from_dir: &Path, spec: &str) -> Result<PathBuf> {
    let mut candidate = from_dir.join(spec);
    if candidate.extension().is_none() {
        let es = candidate.with_extension("es");
        if es.exists() {
            candidate = es;
        } else {
            let js = candidate.with_extension("js");
            if js.exists() {
                candidate = js;
            }
        }
    }
    if candidate.exists() {
        return Ok(candidate);
    }
    Err(Error::ModuleNotFound(format!(
        "{spec} (from {})",
        from_dir.display()
    )))
}

fn collect_deps(program: &Program) -> Vec<String> {
    let mut deps = Vec::new();
    for stmt in &program.body {
        collect_stmt_deps(stmt, &mut deps);
    }
    deps
}

fn collect_stmt_deps(stmt: &Stmt, deps: &mut Vec<String>) {
    match stmt {
        Stmt::Import(ImportDecl::Default { source, .. })
        | Stmt::Import(ImportDecl::Named { source, .. })
        | Stmt::Import(ImportDecl::Namespace { source, .. })
        | Stmt::Import(ImportDecl::SideEffect { source }) => {
            deps.push(source.clone());
        }
        Stmt::Expr(e) | Stmt::Return(Some(e)) | Stmt::Throw(e) => collect_expr_deps(e, deps),
        Stmt::Assign { value, .. } => collect_expr_deps(value, deps),
        Stmt::If {
            test,
            body,
            else_body,
            ..
        } => {
            collect_expr_deps(test, deps);
            for s in body {
                collect_stmt_deps(s, deps);
            }
            if let Some(b) = else_body {
                for s in b {
                    collect_stmt_deps(s, deps);
                }
            }
        }
        Stmt::While { test, body, .. } | Stmt::For { iter: test, body, .. } => {
            collect_expr_deps(test, deps);
            for s in body {
                collect_stmt_deps(s, deps);
            }
        }
        Stmt::Class { superclass, body, .. } => {
            if let Some(e) = superclass {
                collect_expr_deps(e, deps);
            }
            let _ = body;
        }
        Stmt::Export(_) | Stmt::Try { .. } | Stmt::Return(None) | Stmt::Break | Stmt::Continue => {}
    }
}

fn collect_expr_deps(expr: &crate::ast::Expr, deps: &mut Vec<String>) {
    use crate::ast::Expr::*;
    match expr {
        Require(s) => deps.push(s.clone()),
        SoakedCall { callee, args } | Call { callee, args } => {
            collect_expr_deps(callee, deps);
            for a in args {
                collect_expr_deps(a, deps);
            }
        }
        Member { object, property, .. } => {
            collect_expr_deps(object, deps);
            collect_expr_deps(property, deps);
        }
        Binary { left, right, .. } => {
            collect_expr_deps(left, deps);
            collect_expr_deps(right, deps);
        }
        Unary { arg, .. } | Await(arg) | Existence(arg) => collect_expr_deps(arg, deps),
        ExistentialDefault { value, default } => {
            collect_expr_deps(value, deps);
            collect_expr_deps(default, deps);
        }
        Array(els) => {
            for e in els {
                collect_expr_deps(e, deps);
            }
        }
        Object(props) => {
            for (_, v) in props {
                collect_expr_deps(v, deps);
            }
        }
        Func { body, .. } => {
            for s in body {
                collect_stmt_deps(s, deps);
            }
        }
        New { callee, args } => {
            collect_expr_deps(callee, deps);
            for a in args {
                collect_expr_deps(a, deps);
            }
        }
        AssignExpr { value, .. } => collect_expr_deps(value, deps),
        IfExpr {
            test,
            then_branch,
            else_branch,
            ..
        } => {
            collect_expr_deps(test, deps);
            collect_expr_deps(then_branch, deps);
            if let Some(e) = else_branch {
                collect_expr_deps(e, deps);
            }
        }
        Block(stmts) => {
            for s in stmts {
                collect_stmt_deps(s, deps);
            }
        }
        _ => {}
    }
}
