# rdx-wasm

WebAssembly bindings for the [RDX](https://github.com/rdx-lang/rdx) parser, transforms, and schema validator. Parse `.rdx` documents at Rust speed from JavaScript, TypeScript, Deno, and Cloudflare Workers.

## Installation

Build from source with [wasm-pack](https://rustwasm.github.io/wasm-pack/):

```sh
wasm-pack build rdx-wasm --target web     # browsers, Deno, Cloudflare Workers
wasm-pack build rdx-wasm --target bundler  # webpack, Vite, esbuild
wasm-pack build rdx-wasm --target nodejs   # Node.js (prefer rdx-node for native speed)
```

## API

### `parse(input: string): RdxRoot`

Parse an RDX document into an AST.

```js
import init, { parse } from './pkg/rdx_wasm.js';

await init();

const ast = parse(`---
title: Hello
---

# {$title}

<Notice type="warning">
  This API is deprecated.
</Notice>
`);

console.log(ast.frontmatter); // { title: "Hello" }
console.log(ast.children);    // [Heading, Component, ...]
```

### `parseWithDefaults(input: string): RdxRoot`

Parse with built-in transforms (auto-slug headings + table of contents).

```js
const ast = parseWithDefaults("## Setup\n\n## Usage\n");
```

### `parseWithTransforms(input: string, transforms: string[]): RdxRoot`

Parse with a specific set of transforms.

```js
const ast = parseWithTransforms("## Setup\n", ["auto-slug"]);
```

Available transforms: `"auto-slug"`, `"toc"`.

### `validate(ast: RdxRoot, schema: Schema): Diagnostic[]`

Validate a parsed AST against a component schema.

```js
const ast = parse('<Badge label={42} />\n');

const schema = {
  strict: true,
  components: {
    Badge: {
      self_closing: true,
      props: {
        label: { type: "string", required: true }
      }
    }
  }
};

const diagnostics = validate(ast, schema);
// [{ severity: "error", message: "prop `label` on <Badge> expects string, got number", ... }]
```

### `version(): string`

Returns the RDX parser version.

## Output size

| Target | Size (gzip) |
|---|---|
| `--target web` | ~1.1 MB (uncompressed), ~300 KB (gzip) |

## License

Licensed under either of [Apache License, Version 2.0](../LICENSE-APACHE) or [MIT License](../LICENSE-MIT) at your option.
