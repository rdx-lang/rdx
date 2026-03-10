# rdx-node

Native Node.js bindings for the [RDX](https://github.com/rdx-lang/rdx) parser via [napi-rs](https://napi.rs). Rust speed, no WASM, no `init()` — just import and parse.

## Usage

```ts
import { parse, validate } from "@rdx-lang/node";

const ast = parse(`---
title: Hello
---

# {$title}

<Notice type="warning">
  This API is deprecated.
</Notice>
`);

console.log(ast.frontmatter); // { title: "Hello" }
```

## API

| Function | Description |
|---|---|
| `parse(input)` | Parse `.rdx` string to AST |
| `parseWithDefaults(input)` | Parse + auto-slug + TOC |
| `parseWithTransforms(input, ["auto-slug"])` | Parse + selected transforms |
| `validate(ast, schema)` | Validate AST against component schema |
| `collectText(ast)` | Extract plain text from AST |
| `queryAll(ast, "component")` | Find all nodes of a type |
| `version()` | Parser version string |

## vs `@rdx-lang/wasm`

| | `@rdx-lang/node` | `@rdx-lang/wasm` |
|---|---|---|
| **Runtime** | Node.js only | Browsers, Deno, CF Workers, Node |
| **Speed** | Fastest (native V8) | Fast (WASM overhead) |
| **Init** | None | `await init()` required |
| **Install** | Prebuilt binary per platform | Universal |

Use `@rdx-lang/node` for server-side / build tooling. Use `@rdx-lang/wasm` for browsers and edge runtimes.

## Supported platforms

- `x86_64-apple-darwin` (macOS Intel)
- `aarch64-apple-darwin` (macOS Apple Silicon)
- `x86_64-unknown-linux-gnu` (Linux x64)
- `aarch64-unknown-linux-gnu` (Linux ARM64)
- `x86_64-pc-windows-msvc` (Windows x64)

## License

Licensed under either of [Apache License, Version 2.0](../LICENSE-APACHE) or [MIT License](../LICENSE-MIT) at your option.
