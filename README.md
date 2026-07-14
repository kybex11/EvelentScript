# EvelentScript

EvelentScript is a little language that **runs natively** in a Rust VM (`esc`), and can optionally compile to JavaScript for interop.

## Native runtime (recommended)

Build the native CLI once:

```shell
npm run native:build
# or: cd native && cargo build --release
```

Binary: `native/target/release/esc` (Windows: `esc.exe`).

Run a `.es` file **without Node/JS**:

```shell
native\target\release\esc.exe run example_project\src\index.es

# or via esconfig.json entry:
cd example_project
..\native\target\release\esc.exe run

# npm helper from repo root:
npm run esc -- run -p example_project/esconfig.json
```

```shell
esc run path/to/script.es          # native VM
esc run                            # uses esconfig.json → entry
esc compile file.es -o out.js      # optional JS emit
esc build                          # project → JS (interop)
```

Details: [native/README.md](native/README.md).

## Full compiler package

Готовый пакет с бинарником, playground и всеми builtin-либами:

```shell
cd compiler
build.bat
run.bat
```

См. [compiler/README.md](compiler/README.md).

## JavaScript toolchain (optional)

The Node-based `es` CLI compiles `.es` → `.js` (legacy / browser / npm packages):

```shell
npm install --save-dev evelentscript
npm install --global evelentscript
```

```shell
node ./bin/es path/to/script.es
node ./bin/es -c path/to/script.es
node ./bin/es build
```

## File extensions

| Extension | Description |
|-----------|-------------|
| `.es` | EvelentScript source |
| `.lites` | Literate EvelentScript |
| `.es.md` | Literate EvelentScript (Markdown) |

## Editor support

- [`extensions/vscode-evelentscript`](extensions/vscode-evelentscript/README.md) — VS Code
- [`extensions/zed-evelentscript`](extensions/zed-evelentscript/README.md) — Zed

### Native types

EvelentScript supports indentation-based `interface`, `type`, generics, unions, and function annotations. Types are stripped from JS output; use `npm run typecheck` for static checking. See [documentation/sections/native_types.md](documentation/sections/native_types.md).

## Project config (`esconfig.json`)

Shared by both runtimes:

```json
{
  "compilerOptions": {
    "rootDir": "./src",
    "outDir": "./dist",
    "entry": "index.es",
    "bundle": true,
    "outFile": "index.js",
    "bare": true
  }
}
```

```shell
# Native (no JS):
esc run -p ./esconfig.json

# Emit JS (optional):
esc build -p ./esconfig.json
node ./bin/es build
```

See `esconfig.example.json` / `example_project/`.

## Documentation site

```shell
npm run build
npm run docs
```

Open `docs/v1/index.html`, or `npx --yes serve docs/v1`.

## Build (JS compiler sources)

```shell
npm install
npm run build
```

```javascript
const EvelentScript = require('evelentscript');
const js = EvelentScript.compile('square = (x) -> x * x');
```

```javascript
require('evelentscript/register');
```

## License

MIT
