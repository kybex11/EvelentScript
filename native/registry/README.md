# Native registry (awesome-coffeescript ports)

Index of CoffeeScript libraries from [uhub/awesome-coffeescript](https://github.com/uhub/awesome-coffeescript), wired into `esc`.

## Reality check

The upstream list has **350+** repos. Most are Atom packages, full apps, browser/Node frameworks, or abandoned — they cannot run as-is in the native Rust VM.

What this registry provides:

1. **`catalog.json`** — every entry from the awesome list (name, git URL, status)
2. **`packages/`** — native-compatible EvelentScript (`.es`) ports you can `esc add`
3. **`esc search`** / **`esc add <name>`** — install available ports into `evelent_modules/`

## Available ports (`status = available`)

| Package | Inspired by |
|---------|-------------|
| `heap` | qiao/heap.js |
| `easie` | jimjeffers/Easie |
| `shellwords` | jimmycuadra/shellwords |
| `linear-partition` | crispymtn/linear-partition |
| `normat` | rferro/normat |
| `sentimood` | soops/sentimood |
| `priority-queue` | STRd6/priority_queue |
| `parse-decimal-number` | AndreasPizsa/parse-decimal-number |

## Usage

```bash
# from repo root (or set EVELENT_REGISTRY=path/to/native/registry)
cd native/examples/packages/registry-demo
esc add heap
esc add easie
esc install
esc run
```

Or in any package:

```toml
[dependencies]
heap = "*"
shellwords = "*"
```

```coffee
heap = require 'heap'
h = heap.create()
h.push 3
console.log h.pop()
```

Search:

```bash
esc search heap
esc search atom --available   # only native ports
```

Unported catalog entries still show up in `esc search`; install those with `--git` and plan on porting, or contribute a package under `registry/packages/`.

## Rebuild catalog

```bash
python scripts/build-awesome-catalog.py
```
