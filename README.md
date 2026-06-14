# EvelentScript

EvelentScript is a little language that compiles into JavaScript.

## Installation

Once you have Node.js installed:

```shell
# Install locally for a project:
npm install --save-dev evelentscript

# Install globally to execute .es files anywhere:
npm install --global evelentscript
```

## Getting Started

Execute a script:

```shell
es /path/to/script.es
```

Compile a script:

```shell
es -c /path/to/script.es
```

## File extensions

| Extension | Description |
|-----------|-------------|
| `.es` | EvelentScript source |
| `.lites` | Literate EvelentScript |
| `.es.md` | Literate EvelentScript (Markdown) |

## Editor support

- [`extensions/vscode-evelentscript`](extensions/vscode-evelentscript/README.md) — VS Code: подсветка, snippets, folding, native types
- [`extensions/zed-evelentscript`](extensions/zed-evelentscript/README.md) — Zed

### Native types

EvelentScript supports indentation-based `interface`, `type`, generics, unions, and function annotations. Types are stripped from JS output; use `npm run typecheck` for static checking. See [documentation/sections/native_types.md](documentation/sections/native_types.md).

## Documentation site

The HTML in `documentation/site/` is a template — build it first, then open the output:

```shell
npm run build
npm run docs
```

Open `docs/v2/index.html` in a browser, or serve locally:

```shell
npx --yes serve docs/v2
```

Do not open `documentation/site/index.html` directly; it is not the compiled site.

## Build

Install dependencies and compile the compiler from `src/` into `lib/`:

```shell
npm install
npm run build
```

Other build targets:

```shell
npm run build:full      # build twice and run tests
npm run build:browser   # compile + browser bundles
npm run build:all       # full build + browser bundles
npm run build:watch     # watch src/ and rebuild on changes
npm run build:release   # full release pipeline
```

You can also run the script directly:

```shell
node scripts/build.js --help
```


```javascript
const EvelentScript = require('evelentscript');
const js = EvelentScript.compile('square = (x) -> x * x');
```

Register `.es` files with Node.js:

```javascript
require('evelentscript/register');
```

## License

MIT
