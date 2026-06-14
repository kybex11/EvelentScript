## Native type system

EvelentScript supports a **native type syntax** inspired by TypeScript, while keeping the familiar indentation-based language style. Types are stripped from JavaScript output and can be checked with the TypeScript compiler.

### Interfaces

```es
interface User
  id: number
  name: string
  greet: (msg: string): void
```

### Type aliases

```es
type Status = 'ok' | 'error'
type Nullable<T> = T | null
```

### Function annotations

```es
fetchUser = (id: number): Promise<User> ->
  # ...
```

### Optional and readonly members

```es
interface Config
  host: string
  port?: number
  readonly token: string
```

### Unions and intersections

```es
type Id = string | number
type Named = Person & { name: string }
```

## Type checking

Compile sources and verify types with TypeScript:

```shell
npm run build
npm run typecheck -- path/to/file.es
node scripts/typecheck.js src/
```

The checker emits a temporary `.ts` file that merges interface/type declarations with compiled JavaScript, then runs `tsc --noEmit`.

## Editor support

VS Code extension (`extensions/vscode-evelentscript`) highlights native type syntax: `interface`, `type`, `readonly`, generics, unions, and primitives. See the [extension README](../../extensions/vscode-evelentscript/README.md).

### Flow comment types (legacy)

Flow comment syntax via `###` blocks is still supported. See `documentation/sections/type_annotations.md`.

## Reference

The `typescript_source/` directory contains the TypeScript compiler source tree for reference when extending the type system.
