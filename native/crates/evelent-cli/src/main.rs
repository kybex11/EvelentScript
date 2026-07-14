use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use evelent_core::{
    add_dependency, build_project, compile_file, compile_graph, create_package, find_config_file,
    find_manifest, install_dependencies, load_from_cwd, load_pkg, remove_dependency, search_registry,
    CompileOptions, NativeHost, Package, Vm, MANIFEST_NAME, MODULES_DIR,
};

#[derive(Parser, Debug)]
#[command(
    name = "esc",
    version,
    about = "EvelentScript — native runtime + Cargo-like package manager"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Create a new package (like `cargo new`)
    New {
        /// Package directory / name
        name: String,
        /// Create a library instead of a binary
        #[arg(long)]
        lib: bool,
    },
    /// Create Evelent.toml in the current directory (like `cargo init`)
    Init {
        #[arg(long)]
        lib: bool,
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Add a dependency to Evelent.toml and install it
    Add {
        /// Package name as used in `require 'name'`
        name: String,
        /// Local path dependency
        #[arg(long)]
        path: Option<String>,
        /// Git URL dependency
        #[arg(long)]
        git: Option<String>,
        /// Registry version (default: latest available port)
        #[arg(long)]
        version: Option<String>,
        #[arg(long, default_value = ".")]
        cwd: PathBuf,
    },
    /// Search the bundled awesome-coffeescript / Evelent registry catalog
    Search {
        /// Substring matched against name, repo, description
        query: String,
        /// Only show packages ported for native EvelentScript
        #[arg(long)]
        available: bool,
    },
    /// Remove a dependency
    Remove {
        name: String,
        #[arg(long, default_value = ".")]
        cwd: PathBuf,
    },
    /// Install dependencies from Evelent.toml into evelent_modules/
    Install {
        #[arg(long, default_value = ".")]
        cwd: PathBuf,
    },
    /// Run .es natively (uses Evelent.toml entry or esconfig.json)
    Run {
        input: Option<PathBuf>,
        #[arg(long, short = 'p')]
        project: Option<PathBuf>,
        #[arg(long, default_value = ".")]
        cwd: PathBuf,
        #[arg(long = "native-dir")]
        native_dirs: Vec<PathBuf>,
        #[arg(long)]
        print: bool,
    },
    /// Emit JavaScript (interop). Prefer `run` for native execution.
    Compile {
        input: Option<PathBuf>,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        graph: bool,
        #[arg(long, default_value_t = true)]
        bare: bool,
        #[arg(long, short = 'p')]
        project: Option<PathBuf>,
        #[arg(long = "native-dir")]
        native_dirs: Vec<PathBuf>,
    },
    /// Build a whole project from esconfig.json to JS
    Build {
        #[arg(long, short = 'p')]
        project: Option<PathBuf>,
        #[arg(long, default_value = ".")]
        cwd: PathBuf,
    },
    /// Print tokens for a source file (debug)
    Lex {
        input: PathBuf,
    },
    /// List loaded native modules from directories
    Native {
        #[arg(long = "native-dir", default_value = "native-modules")]
        native_dirs: Vec<PathBuf>,
    },
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Commands::New { name, lib } => {
            let dest = PathBuf::from(&name);
            if dest.exists() {
                return Err(format!("destination already exists: {}", dest.display()).into());
            }
            let pkg_name = dest
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(&name)
                .to_string();
            create_package(&dest, &pkg_name, lib)?;
            println!(
                "Created {} package `{}` at {}",
                if lib { "library" } else { "bin" },
                pkg_name,
                dest.display()
            );
            println!("  {MANIFEST_NAME}");
            println!(
                "  {}",
                if lib { "src/lib.es" } else { "src/main.es" }
            );
            println!("\n  cd {name}");
            println!("  esc run");
        }
        Commands::Init { lib, path } => {
            let name = path
                .canonicalize()
                .ok()
                .and_then(|p| {
                    p.file_name()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| "app".into());
            if path.join(MANIFEST_NAME).exists() {
                return Err(format!("{MANIFEST_NAME} already exists").into());
            }
            create_package(&path, &name, lib)?;
            println!("Created {MANIFEST_NAME} in {}", path.display());
        }
        Commands::Add {
            name,
            path,
            git,
            version,
            cwd,
        } => {
            let version = if path.is_none() && git.is_none() && version.is_none() {
                Some("*".into())
            } else {
                version
            };
            if path.is_none() && git.is_none() && version.is_none() {
                return Err(
                    "specify --path, --git, or a registry package name (esc add heap)".into(),
                );
            }
            let pkg = load_pkg(&cwd)?;
            add_dependency(
                &pkg,
                &name,
                path.as_deref(),
                git.as_deref(),
                version.as_deref(),
            )?;
            println!("Added `{name}` → {MODULES_DIR}/{name}");
        }
        Commands::Search { query, available } => {
            let results = search_registry(&query)?;
            let results: Vec<_> = results
                .into_iter()
                .filter(|(_, status, _)| !available || status == "available")
                .collect();
            if results.is_empty() {
                println!("(no matches)");
            } else {
                for (name, status, desc) in results.iter().take(40) {
                    let mark = if status == "available" { "*" } else { " " };
                    let short = if desc.len() > 72 {
                        format!("{}…", &desc[..71])
                    } else {
                        desc.clone()
                    };
                    println!("{mark} {name:<28} [{status}] {short}");
                }
                if results.len() > 40 {
                    println!("… {} more", results.len() - 40);
                }
                println!("\n* = installed via `esc add <name>` (native EvelentScript port)");
            }
        }
        Commands::Remove { name, cwd } => {
            let pkg = load_pkg(&cwd)?;
            remove_dependency(&pkg, &name)?;
            println!("Removed `{name}`");
        }
        Commands::Install { cwd } => {
            let pkg = load_pkg(&cwd)?;
            let installed = install_dependencies(&pkg)?;
            if installed.is_empty() {
                println!("(no dependencies)");
            } else {
                for (name, dest) in installed {
                    println!("  {name} → {}", dest.display());
                }
                println!("Installed into {MODULES_DIR}/");
            }
        }
        Commands::Run {
            input,
            project,
            cwd,
            native_dirs,
            print,
        } => {
            let (input, package_roots) = resolve_run_context(input, project.as_deref(), &cwd)?;
            let mut vm = Vm::new()
                .with_native_dirs(if native_dirs.is_empty() {
                    vec![
                        PathBuf::from("native-modules"),
                        PathBuf::from("../native/native-modules"),
                    ]
                } else {
                    native_dirs
                })
                .with_package_roots(package_roots);
            let value = vm.run_file(&input)?;
            if print {
                println!("{value}");
            }
        }
        Commands::Compile {
            input,
            output,
            graph,
            bare,
            project,
            native_dirs,
        } => {
            if input.is_none() {
                let cfg = resolve_esconfig(project.as_deref(), Path::new("."))?;
                let (written, errors) = build_project(&cfg)?;
                for (src, dest) in &written {
                    println!("{} -> {}", src.display(), dest.display());
                }
                for (_src, err) in &errors {
                    eprintln!("error: {err}");
                }
                if written.is_empty() && errors.is_empty() {
                    println!("(no .es files matched include patterns)");
                }
                if !errors.is_empty() {
                    return Err(format!("{} file(s) failed to compile", errors.len()).into());
                }
                return Ok(());
            }

            let input = input.expect("input");
            let opts = CompileOptions {
                bare,
                native_dirs: if native_dirs.is_empty() {
                    vec![PathBuf::from("native-modules")]
                } else {
                    native_dirs
                },
            };

            if graph {
                let g = compile_graph(&input, &opts)?;
                let out_dir = output.unwrap_or_else(|| {
                    input
                        .parent()
                        .unwrap_or_else(|| Path::new("."))
                        .join("dist")
                });
                std::fs::create_dir_all(&out_dir)?;
                for path in &g.order {
                    let compiled = &g.modules[path];
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("out");
                    let dest = out_dir.join(format!("{name}.js"));
                    std::fs::write(&dest, &compiled.js)?;
                    println!("{} -> {}", path.display(), dest.display());
                }
            } else {
                let compiled = compile_file(&input, &opts)?;
                let dest = output.unwrap_or_else(|| input.with_extension("js"));
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&dest, &compiled.js)?;
                println!("{} -> {}", input.display(), dest.display());
            }
        }
        Commands::Build { project, cwd } => {
            let cfg = resolve_esconfig(project.as_deref(), &cwd)?;
            println!("Using {}", cfg.config_path.display());
            let (written, errors) = build_project(&cfg)?;
            for (src, dest) in &written {
                println!("  {} -> {}", src.display(), dest.display());
            }
            for (_src, err) in &errors {
                eprintln!("error: {err}");
            }
            println!(
                "Compiled {} file(s), {} error(s)",
                written.len(),
                errors.len()
            );
            if !errors.is_empty() {
                return Err(format!("{} file(s) failed to compile", errors.len()).into());
            }
        }
        Commands::Lex { input } => {
            let source = std::fs::read_to_string(&input)?;
            let tokens =
                evelent_core::lexer::Lexer::new(&source, input.display().to_string()).tokenize()?;
            for t in tokens {
                println!(
                    "{:?} {:?} @{}:{}",
                    t.kind, t.lexeme, t.span.line, t.span.column
                );
            }
        }
        Commands::Native { native_dirs } => {
            let mut host = NativeHost::new();
            for dir in &native_dirs {
                let candidates = [
                    dir.clone(),
                    dir.join("target/debug"),
                    dir.join("target/release"),
                    PathBuf::from("target/debug"),
                    PathBuf::from("target/release"),
                ];
                for c in candidates {
                    let _ = host.discover_dir(&c);
                }
            }
            let list = host.list();
            if list.is_empty() {
                println!("(no native modules found)");
            } else {
                for (name, exports) in list {
                    println!("{name}: {}", exports.join(", "));
                }
            }
        }
    }
    Ok(())
}

fn resolve_esconfig(
    project: Option<&Path>,
    cwd: &Path,
) -> Result<evelent_core::ProjectConfig, Box<dyn std::error::Error>> {
    let explicit = match project {
        Some(p) if p.is_dir() => find_config_file(p),
        Some(p) => Some(p.to_path_buf()),
        None => None,
    };
    Ok(load_from_cwd(cwd, explicit.as_deref())?)
}

fn resolve_run_context(
    input: Option<PathBuf>,
    project: Option<&Path>,
    cwd: &Path,
) -> Result<(PathBuf, Vec<PathBuf>), Box<dyn std::error::Error>> {
    // Prefer Evelent.toml package
    if input.is_none() {
        if let Some(manifest) = find_manifest(cwd) {
            let root = manifest.parent().unwrap_or(cwd);
            let pkg = Package::load(root)?;
            let entry = pkg.entry_path();
            if !entry.is_file() {
                return Err(format!("entry not found: {}", entry.display()).into());
            }
            let roots = vec![root.to_path_buf(), root.join(MODULES_DIR)];
            return Ok((entry, roots));
        }
    }

    if let Some(input) = input {
        let mut roots = Vec::new();
        if let Some(manifest) = find_manifest(cwd).or_else(|| find_manifest(&input)) {
            let root = manifest.parent().unwrap_or(cwd).to_path_buf();
            roots.push(root.clone());
            roots.push(root.join(MODULES_DIR));
        }
        return Ok((input, roots));
    }

    // Fall back to esconfig.json
    let cfg = resolve_esconfig(project, cwd)?;
    let entry = cfg.entry.clone().ok_or_else(|| {
        format!(
            "no entry — set package.entry in {MANIFEST_NAME}, compilerOptions.entry in esconfig.json, or pass a .es file"
        )
    })?;
    if !entry.is_file() {
        return Err(format!("entry not found: {}", entry.display()).into());
    }
    let roots = vec![
        cfg.config_dir.clone(),
        cfg.config_dir.join(MODULES_DIR),
    ];
    Ok((entry, roots))
}
