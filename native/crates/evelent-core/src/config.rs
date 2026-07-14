//! TypeScript-style project config (`esconfig.json`).

use std::path::{Path, PathBuf};

use serde::Deserialize;
use walkdir::WalkDir;

use crate::error::{Error, Result};

const CONFIG_NAMES: &[&str] = &["esconfig.json", "esconfig.jsonc"];

const DEFAULT_INCLUDE: &[&str] = &[
    "*.es",
    "**/*.es",
    "*.lites",
    "**/*.lites",
    "*.es.md",
    "**/*.es.md",
];

const DEFAULT_EXCLUDE: &[&str] = &["**/node_modules/**", "**/.git/**", "**/dist/**"];

#[derive(Debug, Clone)]
pub struct ProjectConfig {
    pub config_path: PathBuf,
    pub config_dir: PathBuf,
    pub root_dir: PathBuf,
    pub out_dir: PathBuf,
    pub entry: Option<PathBuf>,
    pub bundle: bool,
    pub out_file: Option<String>,
    pub bare: bool,
    pub source_map: bool,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub native_dirs: Vec<PathBuf>,
}

#[derive(Debug, Deserialize, Default)]
struct RawConfig {
    #[serde(rename = "compilerOptions", default)]
    compiler_options: Option<RawCompilerOptions>,
    #[serde(flatten)]
    flat: RawCompilerOptions,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RawCompilerOptions {
    root_dir: Option<String>,
    input_dir: Option<String>,
    out_dir: Option<String>,
    output_dir: Option<String>,
    entry: Option<String>,
    bundle: Option<bool>,
    out_file: Option<String>,
    bare: Option<bool>,
    source_map: Option<bool>,
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
    native_dirs: Option<Vec<String>>,
}

/// Walk parents from `start` looking for `esconfig.json` / `esconfig.jsonc`.
pub fn find_config_file(start: &Path) -> Option<PathBuf> {
    let mut dir = start
        .canonicalize()
        .unwrap_or_else(|_| start.to_path_buf());
    if dir.is_file() {
        dir = dir.parent()?.to_path_buf();
    }
    loop {
        for name in CONFIG_NAMES {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

pub fn load_config(path: &Path) -> Result<ProjectConfig> {
    let config_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let config_dir = config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let raw_text = std::fs::read_to_string(&config_path)?;
    let stripped = strip_json_comments(&raw_text);
    let raw: RawConfig = serde_json::from_str(&stripped)
        .map_err(|e| Error::Other(format!("invalid {}: {e}", config_path.display())))?;

    let opts = merge_raw(raw);
    let root_rel = opts
        .root_dir
        .or(opts.input_dir)
        .unwrap_or_else(|| ".".into());
    let out_rel = opts
        .out_dir
        .or(opts.output_dir)
        .unwrap_or_else(|| "dist".into());

    let root_dir = config_dir.join(root_rel);
    let out_dir = config_dir.join(out_rel);

    let entry = opts.entry.map(|e| {
        let p = PathBuf::from(&e);
        if p.is_absolute() {
            p
        } else {
            root_dir.join(p)
        }
    });

    let mut include = DEFAULT_INCLUDE
        .iter()
        .map(|s| (*s).to_string())
        .collect::<Vec<_>>();
    if let Some(extra) = opts.include {
        for p in extra {
            if !include.contains(&p) {
                include.push(p);
            }
        }
    }

    let mut exclude = opts
        .exclude
        .unwrap_or_else(|| DEFAULT_EXCLUDE.iter().map(|s| (*s).to_string()).collect());
    // Always exclude outDir contents
    if let Ok(rel) = out_dir.strip_prefix(&root_dir) {
        let pat = format!("{}/**", rel.to_string_lossy().replace('\\', "/"));
        if !exclude.contains(&pat) {
            exclude.push(pat);
        }
    } else {
        exclude.push("**/dist/**".into());
    }

    let native_dirs = opts
        .native_dirs
        .unwrap_or_default()
        .into_iter()
        .map(|d| config_dir.join(d))
        .collect();

    Ok(ProjectConfig {
        config_path,
        config_dir,
        root_dir,
        out_dir,
        entry,
        bundle: opts.bundle.unwrap_or(false),
        out_file: opts.out_file,
        bare: opts.bare.unwrap_or(true),
        source_map: opts.source_map.unwrap_or(false),
        include,
        exclude,
        native_dirs,
    })
}

/// Load config from an explicit path, or search upward from `cwd`.
pub fn load_from_cwd(cwd: &Path, explicit: Option<&Path>) -> Result<ProjectConfig> {
    let path = if let Some(p) = explicit {
        p.to_path_buf()
    } else {
        find_config_file(cwd).ok_or_else(|| {
            Error::Other(
                "Could not find esconfig.json in this directory or any parent directory.".into(),
            )
        })?
    };
    load_config(&path)
}

fn merge_raw(raw: RawConfig) -> RawCompilerOptions {
    let mut base = raw.flat;
    if let Some(nested) = raw.compiler_options {
        if nested.root_dir.is_some() {
            base.root_dir = nested.root_dir;
        }
        if nested.input_dir.is_some() {
            base.input_dir = nested.input_dir;
        }
        if nested.out_dir.is_some() {
            base.out_dir = nested.out_dir;
        }
        if nested.output_dir.is_some() {
            base.output_dir = nested.output_dir;
        }
        if nested.entry.is_some() {
            base.entry = nested.entry;
        }
        if nested.bundle.is_some() {
            base.bundle = nested.bundle;
        }
        if nested.out_file.is_some() {
            base.out_file = nested.out_file;
        }
        if nested.bare.is_some() {
            base.bare = nested.bare;
        }
        if nested.source_map.is_some() {
            base.source_map = nested.source_map;
        }
        if nested.include.is_some() {
            base.include = nested.include;
        }
        if nested.exclude.is_some() {
            base.exclude = nested.exclude;
        }
        if nested.native_dirs.is_some() {
            base.native_dirs = nested.native_dirs;
        }
    }
    base
}

fn strip_json_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;
    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => {
                in_string = true;
                out.push(c);
            }
            '/' if chars.peek() == Some(&'/') => {
                chars.next();
                while let Some(c2) = chars.next() {
                    if c2 == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                while let Some(c2) = chars.next() {
                    if c2 == '*' && chars.peek() == Some(&'/') {
                        chars.next();
                        break;
                    }
                }
            }
            _ => out.push(c),
        }
    }
    // Allow trailing commas (JSONC): ,\s*}  and ,\s*]
    let re = regex::Regex::new(r",(\s*[}\]])").unwrap();
    re.replace_all(&out, "$1").into_owned()
}

pub fn matches_glob(rel_path: &str, pattern: &str) -> bool {
    let normalized = rel_path.replace('\\', "/");
    let pat = pattern.replace('\\', "/");
    let mut optional_prefix = false;
    let body = if let Some(rest) = pat.strip_prefix("**/") {
        optional_prefix = true;
        rest.to_string()
    } else {
        pat
    };
    let mut regex = String::from("^");
    if optional_prefix {
        regex.push_str("(?:.*/)?");
    }
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' => {
                if chars.peek() == Some(&'*') {
                    chars.next();
                    regex.push_str(".*");
                } else {
                    regex.push_str("[^/]*");
                }
            }
            '?' => regex.push_str("[^/]"),
            '.' | '+' | '^' | '$' | '(' | ')' | '|' | '[' | ']' | '{' | '}' | '\\' => {
                regex.push('\\');
                regex.push(c);
            }
            other => regex.push(other),
        }
    }
    regex.push('$');
    regex::Regex::new(&regex)
        .map(|re| re.is_match(&normalized))
        .unwrap_or(false)
}

pub fn matches_any(rel_path: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|p| matches_glob(rel_path, p))
}

pub fn is_evelent_source(path: &Path) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some("es") | Some("lites") => true,
        Some("md") => path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.ends_with(".es.md"))
            .unwrap_or(false),
        _ => false,
    }
}

/// Collect all source files under `rootDir` matching include/exclude.
pub fn collect_sources(cfg: &ProjectConfig) -> Result<Vec<PathBuf>> {
    if !cfg.root_dir.exists() {
        return Err(Error::Other(format!(
            "rootDir does not exist: {}",
            cfg.root_dir.display()
        )));
    }
    let mut files = Vec::new();
    for entry in WalkDir::new(&cfg.root_dir)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                let rel = e
                    .path()
                    .strip_prefix(&cfg.root_dir)
                    .unwrap_or(e.path())
                    .to_string_lossy()
                    .replace('\\', "/");
                if rel.is_empty() {
                    return true;
                }
                !matches_any(&format!("{rel}/"), &cfg.exclude)
                    && !matches_any(&format!("{rel}/**"), &cfg.exclude)
            } else {
                true
            }
        })
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if !is_evelent_source(path) {
            continue;
        }
        let rel = path
            .strip_prefix(&cfg.root_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        if matches_any(&rel, &cfg.exclude) {
            continue;
        }
        if !matches_any(&rel, &cfg.include) {
            continue;
        }
        files.push(path.to_path_buf());
    }
    files.sort();
    Ok(files)
}

/// Map a source path under rootDir to the matching .js path under outDir.
pub fn out_path_for(cfg: &ProjectConfig, source: &Path) -> PathBuf {
    let rel = source
        .strip_prefix(&cfg.root_dir)
        .unwrap_or(source);
    let mut out = cfg.out_dir.join(rel);
    out.set_extension("js");
    out
}
