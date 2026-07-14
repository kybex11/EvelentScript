# Native EvelentScript compiler (Rust)

A **native** EvelentScript runtime (`esc`) that **executes `.es` in a Rust VM** — not via Node/JS.

```bash
cd native
cargo build --release

# global install (puts esc on PATH via ~/.cargo/bin)
cargo install --path crates/evelent-cli --force

# or Windows: compiler\install.bat (adds compiler\bin to user PATH)

# run natively (Rust interpreter)
./target/release/esc run examples/native_hello.es

# optional: emit JavaScript for interop
./target/release/esc compile examples/hello.es -o hello.js
```

## Packages (`Evelent.toml`) — Cargo-like CLI

```bash
esc new my-app              # binary package
esc new greeter --lib       # library package
esc init                    # Evelent.toml in cwd

esc add greeter --path ../greeter   # path dependency
esc add utils --git https://…       # git dependency
esc remove greeter
esc install                         # vendor into evelent_modules/

esc run                     # runs package.entry from Evelent.toml
```

Manifest:

```toml
[package]
name = "my-app"
version = "0.1.0"
entry = "src/main.es"

[dependencies]
greeter = { path = "../greeter" }
```

In code, bare `require` resolves from `evelent_modules/`:

```coffee
greet = require 'greeter'
console.log greet.hello 'world'
```

Example: `examples/packages/demo-app` + `examples/packages/greeter`.

Registry versions (`esc add foo --version 0.1.0`) are stubbed — use `--path` / `--git` for now.

## Project (`esconfig.json`)

```bash
# Native: run entry from Evelent.toml or esconfig
esc run
esc run -p ./esconfig.json

# Optional: emit JavaScript for interop
esc build
esc build -p ./esconfig.json
esc compile examples/hello.es -o hello.js
```

Example `esconfig.json`:

```json
{
  "compilerOptions": {
    "rootDir": "./src",
    "outDir": "./dist",
    "bare": true,
    "include": ["*.es", "**/*.es"],
    "exclude": ["**/node_modules/**", "**/dist/**"]
  }
}
```

With `"bundle": true` and `"entry": "index.es"`, the compiler emits a single concatenated file (`outFile`, default `index.js`).

### Builtin modules (Node-compatible)

`require 'fs'`, `require 'node:path'`, etc. Available:

`assert`, `buffer`, `child_process`, `console`, `constants`, `crypto`, `events`, `fs`, `http`, `https`, `module`, `os`, `path`, `perf_hooks`, `process`, `punycode`, `querystring`, `readline`, `stream`, `string_decoder`, `timers`, `tty`, `url`, `util`, `vm`, `zlib`

```coffee
fs = require 'fs'
os = require 'os'
console.log os.platform()
fs.writeFileSync 'out.txt', 'hi'
```

### `import` / `export` (ESM-style)

```coffee
import { square } from './math_esm'
import greet from './greet_default'
export square = (x) -> x * x
export default (name) -> name
```

Relative `.es` / `.js` deps are resolved into the module graph when you pass `--graph`.

## Native modules

Write a `cdylib` crate that depends on `evelent-native` and exports `evelent_native_init`:

```rust
use evelent_native::{declare_native_module, parse_argv, return_json};

unsafe extern "C" fn greet(argv: *const c_char) -> *mut c_char {
    let args = parse_argv(argv).unwrap_or_default();
    let name = args.first().and_then(|v| v.as_str()).unwrap_or("world");
    return_json(serde_json::json!(format!("hello, {name}")))
}

declare_native_module!("hello_native", {
    "greet" => greet,
});
```

Build the example plugin:

```bash
cargo build -p hello_native --release
cargo run -p evelent-cli -- native --native-dir target/release
```

From EvelentScript:

```coffee
native = require 'native:hello_native'
```

The compiler emits a `require('native:…')` / `__evelent_native.load(…)` bridge call so a Node (or custom) host can dispatch into the loaded plugin.

## Status

This is an early native compiler covering a practical subset of EvelentScript (CoffeeScript-like syntax):

- assignments, operators, `is` / `isnt` / `and` / `or` / `not`
- functions `->` / `=>`, implicit calls
- `if` / `unless` / `while` / `until` / `for…in|of`
- arrays, objects, classes (basic)
- `import` / `export` / `require`
- indentation blocks

Not yet a 1:1 port of the full JS compiler (comprehensions, JSX, source maps, full rewriter, etc.). Contributions can grow the grammar inside `crates/evelent-core`.
