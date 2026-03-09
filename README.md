# rdx

Reference implementation of the [RDX (Reactive Document eXpressions)](https://github.com/rdx-lang/rdx/blob/main/SPECIFICATION.md) specification — a strictly typed, declarative document format built on CommonMark.

RDX documents are pure data. There is no `import`, no code execution, no JavaScript runtime. The parser produces a typed AST that any language or framework can consume.

## Crates

| Crate | Description |
|---|---|
| [`rdx-parser`](rdx-parser/) | Parses `.rdx` documents into a spec-compliant AST |
| [`rdx-ast`](rdx-ast/) | AST type definitions with serde serialization |
| [`rdx-transform`](rdx-transform/) | Composable AST transform pipeline (auto-slug, table of contents) |
| [`rdx-github`](rdx-github/) | Optional transform — converts `#123`, `@user`, and commit SHAs to links |

## Usage

```rust
use rdx_parser::parse;

let root = parse("# Hello\n\n<Notice type=\"warning\">\n  Be careful.\n</Notice>\n");

// root.children contains Heading, Component, etc.
println!("{:#?}", root);
```

With transforms:

```rust
use rdx_transform::{Pipeline, AutoSlug, TableOfContents};

let root = Pipeline::new()
    .add(AutoSlug::new())
    .add(TableOfContents { min_depth: 2, max_depth: 3, auto_insert: true })
    .run("## Setup\n\n## Usage\n");
```

## What RDX parses

- **Frontmatter** — YAML metadata between `---` delimiters
- **Components** — `<Notice type="warning">` with five attribute types (string, primitive, JSON, variable, boolean shorthand)
- **Context variables** — `{$frontmatter.title}` interpolation in text and attributes
- **GFM extensions** — tables, strikethrough, task lists, footnotes
- **Math** — inline `$x^2$` and display `$$\n...\n$$` LaTeX
- **Escaping** — `\{$`, `\{{`, `\}}`, `\{`, `\\`
- **Code blocks** — fenced code with language tags (variables are not interpolated inside code)

All constructs produce dedicated AST node types — no stringly-typed fallbacks.

## Building

```sh
cargo build
cargo test
```

Requires Rust 2024 edition (1.85+).

## Specification

The full language specification is in [`SPECIFICATION.md`](SPECIFICATION.md). The spec defines every syntactic construct and its AST output. This parser is the reference implementation.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
