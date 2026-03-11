# RDX

The official parser and toolchain for [RDX (Reactive Document eXpressions)](https://github.com/rdx-lang/rdx/blob/main/SPECIFICATION.md) — a strictly typed, declarative document format built on CommonMark.

RDX documents are pure data. No `import`, no code execution, no JavaScript runtime. Parse `.rdx` files into a typed AST from Rust, Node.js, Python, or the browser.

## Install

**Rust**

```sh
cargo add rdx-parser
```

**Node.js** (native)

```sh
npm install @rdx-lang/node
```

**Python**

```sh
pip install rdx-parser
```

**Browser / Deno / Edge**

```sh
npm install @rdx-lang/wasm
```

**CLI**

```sh
cargo install rdx-cli
```

## Usage

```rust
use rdx_parser::parse;

let root = parse("# Hello\n\n<Notice type=\"warning\">\n  Be careful.\n</Notice>\n");
println!("{:#?}", root);
```

```python
import rdx
ast = rdx.parse("# Hello\n\n<Notice type=\"warning\">\n  Be careful.\n</Notice>\n")
print(ast["children"][0]["type"])  # heading
```

```typescript
import { parse } from "@rdx-lang/node";
const ast = parse(
  '# Hello\n\n<Notice type="warning">\n  Be careful.\n</Notice>\n',
);
console.log(ast.children[0].type); // heading
```

## Crates

| Crate                             | Description                                                             |
| --------------------------------- | ----------------------------------------------------------------------- |
| [`rdx-ast`](rdx-ast/)             | AST type definitions with serde serialization                           |
| [`rdx-parser`](rdx-parser/)       | Parses `.rdx` documents into a spec-compliant AST                       |
| [`rdx-schema`](rdx-schema/)       | Schema validation for components — required props, types, enum values   |
| [`rdx-transform`](rdx-transform/) | Composable AST transform pipeline (auto-slug, table of contents)        |
| [`rdx-github`](rdx-github/)       | Optional transform — converts `#123`, `@user`, and commit SHAs to links |
| [`rdx-wasm`](rdx-wasm/)           | WebAssembly bindings for browsers, Deno, and edge runtimes              |
| [`rdx-node`](rdx-node/)           | Native Node.js bindings via napi-rs                                     |
| [`rdx-py`](rdx-py/)               | Python bindings via PyO3 — `pip install rdx-parser`                     |
| [`rdx-cli`](rdx-cli/)             | CLI — parse, validate, convert MDX→RDX, format                          |

## What RDX parses

- **Frontmatter** — YAML metadata between `---` delimiters
- **Components** — `<Notice type="warning">` with five attribute types (string, primitive, JSON, variable, boolean shorthand)
- **Context variables** — `{$frontmatter.title}` interpolation in text and attributes
- **GFM extensions** — tables, strikethrough, task lists, footnotes
- **Math** — inline `$x^2$` and display `$$..$$` LaTeX
- **Escaping** — `\{$`, `\{{`, `\}}`, `\{`, `\\`
- **Code blocks** — fenced code with language tags (variables are not interpolated inside code)
- **HTML pass-through** — lowercase tags handled per CommonMark rules

All constructs produce dedicated AST node types — no stringly-typed fallbacks.

## Schema validation

Define what components your project allows and validate at build time:

```rust
use rdx_schema::{Schema, ComponentSchema, PropSchema, PropType, validate};
use rdx_parser::parse;

let schema = Schema::new()
    .strict(true)
    .component("Notice", ComponentSchema::new()
        .prop("type", PropSchema::enum_required(vec!["info", "warning", "error"]))
    );

let root = parse("<Notice type=\"info\">\nContent.\n</Notice>\n");
let diagnostics = validate(&root, &schema);
assert!(diagnostics.is_empty());
```

## Transforms

```rust
use rdx_transform::{Pipeline, AutoSlug, TableOfContents};

let root = Pipeline::new()
    .add(AutoSlug::new())
    .add(TableOfContents { min_depth: 2, max_depth: 3, auto_insert: true })
    .run("## Setup\n\n## Usage\n");
```

## CLI

```sh
rdx parse document.rdx --pretty        # Output AST as JSON
rdx validate document.rdx --schema s.json  # Validate against schema
rdx convert page.mdx --in-place        # Convert MDX → RDX
```

Prebuilt binaries available on [GitHub Releases](https://github.com/rdx-lang/rdx/releases).

## Building

```sh
cargo build
cargo test
```

Requires Rust 2024 edition (1.85+).

## Ecosystem

| Repo                                                             | Description                                                       |
| ---------------------------------------------------------------- | ----------------------------------------------------------------- |
| [`rdx`](https://github.com/rdx-lang/rdx)                         | Parser, schema, transforms, and language bindings (this repo)     |
| [`rdx-js`](https://github.com/rdx-lang/rdx-js)                   | TypeScript types, JS transform pipeline, and JS-native extensions |
| [`tree-sitter-rdx`](https://github.com/rdx-lang/tree-sitter-rdx) | Syntax highlighting for Neovim, Helix, Zed                        |

## Specification

The full language specification is in [`SPECIFICATION.md`](SPECIFICATION.md).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
