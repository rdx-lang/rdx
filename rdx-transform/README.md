# rdx-transform

Composable AST transform pipeline for [RDX](https://github.com/rdx-lang/rdx) documents.

## Usage

```rust
use rdx_transform::{Pipeline, AutoSlug, TableOfContents};

let root = Pipeline::new()
    .add(AutoSlug::new())
    .add(TableOfContents::default())
    .run("## Setup\n\n## Usage\n");
```

Or use the convenience function:

```rust
use rdx_transform::parse_with_defaults;

let root = parse_with_defaults("## Setup\n\n## Usage\n");
```

## Built-in transforms

### AutoSlug

Generates URL-safe `id` attributes on headings for deep-linking.

```rust
use rdx_transform::AutoSlug;

let slug = AutoSlug::new();
// "## Getting Started" → id: "getting-started"
// Duplicate headings get "-1", "-2", etc.
```

### TableOfContents

Generates a nested list of links from document headings.

```rust
use rdx_transform::TableOfContents;

let toc = TableOfContents {
    min_depth: 2,   // skip h1
    max_depth: 3,   // include h2 and h3
    auto_insert: true, // insert at top of document
};
```

If `auto_insert` is false, the transform replaces a `<TableOfContents />` component placeholder instead.

## Writing custom transforms

Implement the `Transform` trait:

```rust
use rdx_transform::{Transform, Root};

struct StripImages;

impl Transform for StripImages {
    fn name(&self) -> &str { "strip-images" }
    fn transform(&self, root: &mut Root, _source: &str) {
        // modify root.children
    }
}
```

## Utilities

| Function | Description |
|---|---|
| `walk(nodes, callback)` | Immutable depth-first traversal |
| `walk_mut(nodes, callback)` | Mutable depth-first traversal |
| `collect_text(nodes)` | Extract plain text from a node tree |

## License

Licensed under either of [Apache License, Version 2.0](../LICENSE-APACHE) or [MIT License](../LICENSE-MIT) at your option.
