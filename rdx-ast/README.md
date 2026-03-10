# rdx-ast

Core AST (Abstract Syntax Tree) type definitions for [RDX](https://github.com/rdx-lang/rdx) documents.

This crate defines the data structures that every RDX tool operates on. The AST is the canonical contract between the parser and any downstream consumer — renderers, validators, transforms, and language bindings all depend on these types.

## Types

| Type | Description |
|---|---|
| `Root` | Document root with optional frontmatter and children |
| `Node` | Enum of all AST node types |
| `ComponentNode` | Custom component (`<Notice>`, `<Badge>`) with typed attributes |
| `StandardBlockNode` | CommonMark blocks: paragraphs, headings, lists, tables, etc. |
| `CodeBlockNode` | Fenced code block with language and meta |
| `LinkNode` / `ImageNode` | Links and images with URL, title, alt text |
| `FootnoteNode` | Footnote definitions and references |
| `TextNode` | Literal text, inline code, inline/display math |
| `VariableNode` | Context variable interpolation (`{$path}`) |
| `ErrorNode` | Malformed construct for error boundaries |
| `AttributeNode` | Component attribute with name, typed value, and position |
| `Position` / `Point` | Source location (1-indexed lines/columns, 0-indexed byte offsets) |

## Attribute values

The `AttributeValue` enum covers all RDX attribute types:

- `String` — `label="Click Me"`
- `Number` — `count={42}`
- `Bool` — `active={true}`
- `Null` — `value={null}`
- `Object` — `config={{"type": "bar"}}`
- `Array` — `items={{[1, 2, 3]}}`
- `Variable` — `title={$frontmatter.title}`

## Serialization

All types derive `Serialize` and `Deserialize` via serde. The JSON output matches the [RDX Specification](https://github.com/rdx-lang/rdx/blob/main/SPECIFICATION.md) AST schema exactly.

```rust
use rdx_ast::{Node, ComponentNode};

let node = Node::Component(ComponentNode { /* ... */ });
let json = serde_json::to_string_pretty(&node).unwrap();
```

## License

Licensed under either of [Apache License, Version 2.0](../LICENSE-APACHE) or [MIT License](../LICENSE-MIT) at your option.
