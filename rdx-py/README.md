# rdx-py

Python bindings for the [RDX](https://github.com/rdx-lang/rdx) parser via [PyO3](https://pyo3.rs) and [maturin](https://www.maturin.rs). Parse `.rdx` documents at Rust speed, get plain Python dicts back.

## Installation

```sh
pip install rdx
```

## Usage

```python
import rdx

ast = rdx.parse("""---
title: API Reference
---

# {$title}

<Notice type="warning">
  This endpoint is deprecated.
</Notice>
""")

print(ast["frontmatter"])  # {'title': 'API Reference'}
print(ast["children"][0]["type"])  # 'heading'
```

## API

### `rdx.parse(input: str) -> dict`

Parse an RDX document into an AST dict.

### `rdx.parse_with_defaults(input: str) -> dict`

Parse with built-in transforms (auto-slug headings + table of contents).

### `rdx.parse_with_transforms(input: str, transforms: list[str]) -> dict`

Parse with selected transforms. Available: `"auto-slug"`, `"toc"`.

### `rdx.validate(ast: dict, schema: dict) -> list[dict]`

Validate an AST against a component schema.

```python
schema = {
    "strict": True,
    "components": {
        "Notice": {
            "props": {
                "type": {"type": "enum", "required": True, "values": ["info", "warning", "error"]}
            }
        }
    }
}

diagnostics = rdx.validate(ast, schema)
for d in diagnostics:
    print(f"{d['severity']}: {d['message']} at line {d['line']}")
```

### `rdx.collect_text(ast: dict) -> str`

Extract all plain text from the AST. Useful for search indexing, embeddings, and reading time estimation.

```python
text = rdx.collect_text(ast)
words = text.split()
reading_time = len(words) // 200  # minutes
```

### `rdx.query_all(ast: dict, node_type: str) -> list[dict]`

Find all nodes of a given type.

```python
headings = rdx.query_all(ast, "heading")
components = rdx.query_all(ast, "component")
```

### `rdx.version() -> str`

Returns the RDX parser version.

## RAG / AI Pipeline Example

```python
import rdx

def prepare_for_embedding(rdx_source: str) -> list[str]:
    """Parse RDX and split into clean text chunks by heading."""
    ast = rdx.parse(rdx_source)
    chunks = []
    current = []

    for node in ast["children"]:
        if node["type"] == "heading":
            if current:
                chunks.append(rdx.collect_text({"type": "root", "frontmatter": None, "children": current, "position": ast["position"]}))
            current = [node]
        else:
            current.append(node)

    if current:
        chunks.append(rdx.collect_text({"type": "root", "frontmatter": None, "children": current, "position": ast["position"]}))

    return chunks
```

## Development

```sh
pip install maturin
maturin develop
python -c "import rdx; print(rdx.parse('# Hello'))"
```

## License

Licensed under either of [Apache License, Version 2.0](../LICENSE-APACHE) or [MIT License](../LICENSE-MIT) at your option.
