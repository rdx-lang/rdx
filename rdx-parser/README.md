# rdx-parser

Reference parser for [RDX](https://github.com/rdx-lang/rdx) documents. Converts `.rdx` source text into a spec-compliant AST.

## Usage

```rust
use rdx_parser::parse;

let root = parse("# Hello\n\n<Notice type=\"warning\">\n  Be careful.\n</Notice>\n");

for child in &root.children {
    println!("{:#?}", child);
}
```

## What it parses

- **Frontmatter** — YAML metadata between `---` delimiters
- **CommonMark** — headings, paragraphs, lists, blockquotes, thematic breaks, links, images
- **GFM extensions** — tables, strikethrough, task lists, footnotes
- **Components** — `<Notice type="warning">` with five attribute types
- **Variables** — `{$frontmatter.title}` interpolation in text and attributes
- **Math** — inline `$x^2$` and display `$$..$$` LaTeX
- **Escaping** — `\{$`, `\{{`, `\}}`, `\{`, `\\`
- **HTML pass-through** — lowercase tags handled per CommonMark rules

## Error handling

The parser never panics on malformed input. Instead, it emits `ErrorNode` entries in the AST for:

- Unclosed component tags
- Misnested tags (`<A><B></A></B>`)
- Malformed JSON attributes
- Invalid variable paths

This allows host environments to render error boundaries without crashing the entire document.

## Entry point

```rust
pub fn parse(input: &str) -> Root
```

One function. String in, AST out. No configuration, no plugins, no runtime.

## Dependencies

- `rdx-ast` — AST type definitions
- `pulldown-cmark` — CommonMark parsing with GFM extensions
- `serde_json` — JSON attribute parsing
- `serde-saphyr` — YAML frontmatter parsing

## License

Licensed under either of [Apache License, Version 2.0](../LICENSE-APACHE) or [MIT License](../LICENSE-MIT) at your option.
