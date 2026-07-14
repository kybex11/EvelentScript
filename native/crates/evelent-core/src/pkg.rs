//! Package manager: `Evelent.toml` (Cargo-style) + local `evelent_modules/`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

pub const MANIFEST_NAME: &str = "Evelent.toml";
pub const MODULES_DIR: &str = "evelent_modules";
pub const LOCK_NAME: &str = "Evelent.lock";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub package: PackageMeta,
    #[serde(default)]
    pub dependencies: BTreeMap<String, Dependency>,
    #[serde(default)]
    pub lib: Option<LibSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMeta {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    /// Entry script relative to the package root (default: src/main.es or src/lib.es)
    #[serde(default)]
    pub entry: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

fn default_version() -> String {
    "0.1.0".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibSection {
    /// Library export file (default: src/lib.es)
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Dependency {
    /// `foo = "0.1.0"` — version from registry (stub: not registered yet)
    Version(String),
    Detailed(DepSource),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepSource {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub git: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Package {
    pub root: PathBuf,
    pub manifest: Manifest,
}

impl Package {
    pub fn load(root: &Path) -> Result<Self> {
        let manifest_path = root.join(MANIFEST_NAME);
        if !manifest_path.is_file() {
            return Err(Error::Other(format!(
                "no {} in {}",
                MANIFEST_NAME,
                root.display()
            )));
        }
        let text = fs::read_to_string(&manifest_path)?;
        let manifest: Manifest = toml::from_str(&text)
            .map_err(|e| Error::Other(format!("invalid {}: {e}", manifest_path.display())))?;
        Ok(Self {
            root: root.canonicalize().unwrap_or_else(|_| root.to_path_buf()),
            manifest,
        })
    }

    pub fn entry_path(&self) -> PathBuf {
        if let Some(e) = &self.manifest.package.entry {
            return self.root.join(e);
        }
        let main = self.root.join("src/main.es");
        if main.is_file() {
            return main;
        }
        let lib = self.root.join("src/lib.es");
        if lib.is_file() {
            return lib;
        }
        if let Some(lib) = &self.manifest.lib {
            if let Some(p) = &lib.path {
                return self.root.join(p);
            }
        }
        self.root.join("src/main.es")
    }

    /// File exported when another package does `require 'this-name'`.
    pub fn lib_path(&self) -> PathBuf {
        if let Some(lib) = &self.manifest.lib {
            if let Some(p) = &lib.path {
                return self.root.join(p);
            }
        }
        let lib = self.root.join("src/lib.es");
        if lib.is_file() {
            return lib;
        }
        self.entry_path()
    }
}

/// Walk parents looking for `Evelent.toml`.
pub fn find_manifest(start: &Path) -> Option<PathBuf> {
    let mut dir = start
        .canonicalize()
        .unwrap_or_else(|_| start.to_path_buf());
    if dir.is_file() {
        dir = dir.parent()?.to_path_buf();
    }
    loop {
        let candidate = dir.join(MANIFEST_NAME);
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

pub fn load_from_cwd(cwd: &Path) -> Result<Package> {
    let path = find_manifest(cwd).ok_or_else(|| {
        Error::Other(format!(
            "could not find {MANIFEST_NAME} in {} or parents — run `esc init`",
            cwd.display()
        ))
    })?;
    let root = path.parent().unwrap_or(cwd);
    Package::load(root)
}

/// Create a new application or library package scaffold.
pub fn create_package(dest: &Path, name: &str, is_lib: bool) -> Result<()> {
    fs::create_dir_all(dest.join("src"))?;
    let entry = if is_lib { "src/lib.es" } else { "src/main.es" };
    let mut manifest = String::new();
    manifest.push_str(&format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
entry = "{entry}"
"#
    ));
    if is_lib {
        manifest.push_str(
            r#"
[lib]
path = "src/lib.es"
"#,
        );
    }
    manifest.push_str(
        r#"
[dependencies]
"#,
    );
    fs::write(dest.join(MANIFEST_NAME), manifest)?;

    if is_lib {
        fs::write(
            dest.join("src/lib.es"),
            format!("# {name}\n\nexports.hello = (who) -> \"hello from {name}, #{{who}}\"\n"),
        )?;
    } else {
        fs::write(
            dest.join("src/main.es"),
            format!(
                r#"# {name}

console.log 'Hello from {name}!'
"#
            ),
        )?;
    }

    // Keep esconfig so toolchains that only know JSON still work
    let esconfig = serde_json::json!({
        "compilerOptions": {
            "rootDir": "./src",
            "outDir": "./dist",
            "entry": if is_lib { "lib.es" } else { "main.es" },
            "bare": true
        }
    });
    fs::write(
        dest.join("esconfig.json"),
        serde_json::to_string_pretty(&esconfig).unwrap() + "\n",
    )?;

    fs::write(
        dest.join(".gitignore"),
        format!("{MODULES_DIR}/\ndist/\n"),
    )?;

    Ok(())
}

/// Add a path / git / version dependency to the manifest and reinstall.
pub fn add_dependency(
    pkg: &Package,
    name: &str,
    path: Option<&str>,
    git: Option<&str>,
    version: Option<&str>,
) -> Result<()> {
    let mut deps = pkg.manifest.dependencies.clone();
    let dep = if let Some(p) = path {
        Dependency::Detailed(DepSource {
            path: Some(p.into()),
            git: None,
            branch: None,
            tag: None,
            version: None,
        })
    } else if let Some(g) = git {
        Dependency::Detailed(DepSource {
            path: None,
            git: Some(g.into()),
            branch: None,
            tag: None,
            version: None,
        })
    } else {
        Dependency::Version(version.unwrap_or("*").into())
    };
    deps.insert(name.to_string(), dep);

    write_manifest(&pkg.root, &pkg.manifest.package, &deps, pkg.manifest.lib.as_ref())?;
    install_dependencies(&Package::load(&pkg.root)?)?;
    Ok(())
}

pub fn remove_dependency(pkg: &Package, name: &str) -> Result<()> {
    let mut deps = pkg.manifest.dependencies.clone();
    if deps.remove(name).is_none() {
        return Err(Error::Other(format!("dependency not found: {name}")));
    }
    write_manifest(&pkg.root, &pkg.manifest.package, &deps, pkg.manifest.lib.as_ref())?;
    let vendor = pkg.root.join(MODULES_DIR).join(name);
    if vendor.exists() {
        fs::remove_dir_all(&vendor)?;
    }
    install_dependencies(&Package::load(&pkg.root)?)?;
    Ok(())
}

fn write_manifest(
    root: &Path,
    meta: &PackageMeta,
    deps: &BTreeMap<String, Dependency>,
    lib: Option<&LibSection>,
) -> Result<()> {
    let mut out = String::new();
    out.push_str("[package]\n");
    out.push_str(&format!("name = {:?}\n", meta.name));
    out.push_str(&format!("version = {:?}\n", meta.version));
    if let Some(e) = &meta.entry {
        out.push_str(&format!("entry = {:?}\n", e));
    }
    if let Some(d) = &meta.description {
        out.push_str(&format!("description = {:?}\n", d));
    }
    if let Some(lib) = lib {
        out.push_str("\n[lib]\n");
        if let Some(p) = &lib.path {
            out.push_str(&format!("path = {:?}\n", p));
        }
    }
    out.push_str("\n[dependencies]\n");
    for (name, dep) in deps {
        match dep {
            Dependency::Version(v) => out.push_str(&format!("{name} = {v:?}\n")),
            Dependency::Detailed(s) => {
                let mut parts = Vec::new();
                if let Some(p) = &s.path {
                    parts.push(format!("path = {p:?}"));
                }
                if let Some(g) = &s.git {
                    parts.push(format!("git = {g:?}"));
                }
                if let Some(b) = &s.branch {
                    parts.push(format!("branch = {b:?}"));
                }
                if let Some(t) = &s.tag {
                    parts.push(format!("tag = {t:?}"));
                }
                if let Some(v) = &s.version {
                    parts.push(format!("version = {v:?}"));
                }
                out.push_str(&format!("{name} = {{ {} }}\n", parts.join(", ")));
            }
        }
    }
    fs::write(root.join(MANIFEST_NAME), out)?;
    Ok(())
}

/// Vendor dependencies into `evelent_modules/<name>/`.
pub fn install_dependencies(pkg: &Package) -> Result<Vec<(String, PathBuf)>> {
    let modules = pkg.root.join(MODULES_DIR);
    fs::create_dir_all(&modules)?;
    let mut installed = Vec::new();
    let mut lock_lines = vec![
        "# Autogenerated by esc — do not edit by hand".into(),
        format!("root = {:?}", pkg.manifest.package.name),
        String::new(),
    ];

    for (name, dep) in &pkg.manifest.dependencies {
        let dest = modules.join(name);
        match resolve_dep_source(pkg, name, dep)? {
            ResolvedSource::Path(src) => {
                if dest.exists() {
                    fs::remove_dir_all(&dest)?;
                }
                copy_dir_recursive(&src, &dest)?;
                lock_lines.push(format!("{name} = {{ path = {:?} }}", src.display()));
                installed.push((name.clone(), dest));
            }
            ResolvedSource::Git { url, rev } => {
                if dest.exists() {
                    fs::remove_dir_all(&dest)?;
                }
                clone_git(&url, rev.as_deref(), &dest)?;
                lock_lines.push(format!(
                    "{name} = {{ git = {:?}, rev = {:?} }}",
                    url,
                    rev.unwrap_or_else(|| "HEAD".into())
                ));
                installed.push((name.clone(), dest));
            }
            ResolvedSource::Registry { version } => {
                // Last-chance resolve (paths already tried above).
                let src = resolve_registry_package(name, &version).map_err(|e| {
                    let hint = catalog_git_url(name)
                        .map(|g| format!("  Tip: esc add {name} --git {g}"))
                        .unwrap_or_default();
                    Error::Other(format!("{e}\n{hint}"))
                })?;
                if dest.exists() {
                    fs::remove_dir_all(&dest)?;
                }
                copy_dir_recursive(&src, &dest)?;
                lock_lines.push(format!(
                    "{name} = {{ registry = true, version = {:?}, path = {:?} }}",
                    version,
                    src.display()
                ));
                installed.push((name.clone(), dest));
            }
        }
    }

    fs::write(pkg.root.join(LOCK_NAME), lock_lines.join("\n") + "\n")?;
    Ok(installed)
}

enum ResolvedSource {
    Path(PathBuf),
    Git { url: String, rev: Option<String> },
    Registry { version: String },
}

fn resolve_dep_source(pkg: &Package, name: &str, dep: &Dependency) -> Result<ResolvedSource> {
    match dep {
        Dependency::Version(v) => {
            // Local registry ports only. Unported catalog entries need --git / --path.
            if let Ok(path) = resolve_registry_package(name, v) {
                return Ok(ResolvedSource::Path(path));
            }
            Ok(ResolvedSource::Registry {
                version: v.clone(),
            })
        }
        Dependency::Detailed(s) => {
            if let Some(p) = &s.path {
                let path = if Path::new(p).is_absolute() {
                    PathBuf::from(p)
                } else {
                    pkg.root.join(p)
                };
                let path = path
                    .canonicalize()
                    .map_err(|_| Error::Other(format!("path dependency not found: {p}")))?;
                // Ensure it is a package (has Evelent.toml) or at least has .es sources
                if !path.join(MANIFEST_NAME).is_file()
                    && !path.join("src/lib.es").is_file()
                    && !path.join("index.es").is_file()
                {
                    return Err(Error::Other(format!(
                        "dependency {name}: expected {MANIFEST_NAME} or src/lib.es in {}",
                        path.display()
                    )));
                }
                Ok(ResolvedSource::Path(path))
            } else if let Some(git) = &s.git {
                let rev = s.tag.clone().or_else(|| s.branch.clone());
                Ok(ResolvedSource::Git {
                    url: git.clone(),
                    rev,
                })
            } else if let Some(v) = &s.version {
                if let Ok(path) = resolve_registry_package(name, v) {
                    return Ok(ResolvedSource::Path(path));
                }
                Ok(ResolvedSource::Registry {
                    version: v.clone(),
                })
            } else {
                Err(Error::Other(format!(
                    "dependency {name}: need path, git, or version"
                )))
            }
        }
    }
}

/// Locate the bundled / env registry root (`native/registry` or `EVELENT_REGISTRY`).
pub fn registry_root() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("EVELENT_REGISTRY") {
        let path = PathBuf::from(p);
        if path.is_dir() {
            return Some(path);
        }
    }
    // Walk up from CWD and from the executable looking for native/registry
    let mut candidates = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        let mut dir = cwd;
        for _ in 0..8 {
            candidates.push(dir.join("native/registry"));
            candidates.push(dir.join("registry"));
            if !dir.pop() {
                break;
            }
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(mut dir) = exe.parent().map(|p| p.to_path_buf()) {
            for _ in 0..8 {
                candidates.push(dir.join("native/registry"));
                candidates.push(dir.join("registry"));
                candidates.push(dir.join("../registry"));
                if !dir.pop() {
                    break;
                }
            }
        }
    }
    candidates.into_iter().find(|p| p.join("catalog.json").is_file())
}

fn load_catalog() -> Option<serde_json::Value> {
    let root = registry_root()?;
    let text = fs::read_to_string(root.join("catalog.json")).ok()?;
    serde_json::from_str(&text).ok()
}

fn resolve_registry_package(name: &str, version: &str) -> Result<PathBuf> {
    let root = registry_root().ok_or_else(|| {
        Error::Other(
            "no Evelent registry found (set EVELENT_REGISTRY or run from the repo with native/registry)"
                .into(),
        )
    })?;
    let catalog: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root.join("catalog.json")).map_err(|e| {
            Error::Other(format!("cannot read catalog.json: {e}"))
        })?,
    )
    .map_err(|e| Error::Other(format!("invalid catalog.json: {e}")))?;

    let packages = catalog
        .get("packages")
        .and_then(|p| p.as_array())
        .ok_or_else(|| Error::Other("catalog.json: missing packages[]".into()))?;

    let entry = packages.iter().find(|p| {
        p.get("name").and_then(|n| n.as_str()) == Some(name)
            && p.get("status").and_then(|s| s.as_str()) == Some("available")
    });

    let entry = entry.ok_or_else(|| {
        Error::Other(format!(
            "package `{name}` is not in the local registry as an available EvelentScript port. \
             Try: esc search {name}  ·  or: esc add {name} --git <url>"
        ))
    })?;

    if version != "*" && version != "latest" {
        if let Some(v) = entry.get("version").and_then(|v| v.as_str()) {
            if v != version {
                return Err(Error::Other(format!(
                    "registry has {name}@{v}, requested {version}"
                )));
            }
        }
    }

    let rel = entry
        .get("path")
        .and_then(|p| p.as_str())
        .ok_or_else(|| Error::Other(format!("registry entry {name} has no path")))?;
    let path = root.join(rel);
    if !path.join(MANIFEST_NAME).is_file() {
        return Err(Error::Other(format!(
            "registry package missing {MANIFEST_NAME}: {}",
            path.display()
        )));
    }
    path.canonicalize()
        .map_err(|_| Error::Other(format!("registry path not found: {}", path.display())))
}

fn catalog_git_url(name: &str) -> Option<String> {
    let catalog = load_catalog()?;
    let packages = catalog.get("packages")?.as_array()?;
    packages.iter().find_map(|p| {
        if p.get("name").and_then(|n| n.as_str()) == Some(name) {
            p.get("git").and_then(|g| g.as_str()).map(|s| s.to_string())
        } else {
            None
        }
    })
}

/// Search the awesome-coffeescript / Evelent registry catalog.
pub fn search_registry(query: &str) -> Result<Vec<(String, String, String)>> {
    let catalog = load_catalog().ok_or_else(|| {
        Error::Other("no registry catalog found (native/registry/catalog.json)".into())
    })?;
    let q = query.to_lowercase();
    let mut out = Vec::new();
    if let Some(packages) = catalog.get("packages").and_then(|p| p.as_array()) {
        for p in packages {
            let name = p.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let desc = p.get("description").and_then(|d| d.as_str()).unwrap_or("");
            let status = p.get("status").and_then(|s| s.as_str()).unwrap_or("catalog");
            let repo = p.get("repo").and_then(|r| r.as_str()).unwrap_or("");
            if name.to_lowercase().contains(&q)
                || desc.to_lowercase().contains(&q)
                || repo.to_lowercase().contains(&q)
            {
                out.push((name.to_string(), status.to_string(), desc.to_string()));
            }
        }
    }
    Ok(out)
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest)?;
    for entry in walkdir::WalkDir::new(src)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        let rel = path.strip_prefix(src).unwrap_or(path);
        // skip vendor caches inside deps
        if rel.components().any(|c| {
            matches!(
                c.as_os_str().to_str(),
                Some("evelent_modules" | "node_modules" | "dist" | "target" | ".git")
            )
        }) {
            continue;
        }
        let target = dest.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(path, &target)?;
        }
    }
    Ok(())
}

fn clone_git(url: &str, rev: Option<&str>, dest: &Path) -> Result<()> {
    use std::process::Command;
    let status = Command::new("git")
        .args(["clone", "--depth", "1"])
        .arg(url)
        .arg(dest)
        .status()
        .map_err(|e| Error::Other(format!("git clone failed: {e}")))?;
    if !status.success() {
        return Err(Error::Other(format!("git clone exited with {status}")));
    }
    if let Some(rev) = rev {
        let status = Command::new("git")
            .current_dir(dest)
            .args(["checkout", rev])
            .status()
            .map_err(|e| Error::Other(format!("git checkout failed: {e}")))?;
        if !status.success() {
            return Err(Error::Other(format!("git checkout {rev} failed")));
        }
    }
    Ok(())
}

/// Resolve a bare package name to its library entry file.
/// Search order: `evelent_modules/<name>/`, then package roots passed by the VM.
pub fn resolve_package_lib(name: &str, from_dir: &Path, extra_roots: &[PathBuf]) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    // Walk up from from_dir looking for evelent_modules
    let mut dir = from_dir.to_path_buf();
    loop {
        candidates.push(dir.join(MODULES_DIR).join(name));
        if !dir.pop() {
            break;
        }
    }
    for root in extra_roots {
        candidates.push(root.join(name));
        candidates.push(root.join(MODULES_DIR).join(name));
    }

    for base in candidates {
        if let Some(lib) = package_lib_in(&base) {
            return Some(lib);
        }
    }
    None
}

fn package_lib_in(base: &Path) -> Option<PathBuf> {
    if !base.exists() {
        return None;
    }
    if let Ok(pkg) = Package::load(base) {
        let lib = pkg.lib_path();
        if lib.is_file() {
            return Some(lib);
        }
    }
    for name in ["src/lib.es", "lib.es", "index.es", "main.es", "src/main.es"] {
        let p = base.join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}
