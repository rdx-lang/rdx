# rdx-schema

Schema validation for [RDX](https://github.com/rdx-lang/rdx) component ASTs. Define what components exist, what props they accept, and validate documents at build time.

## Usage

```rust
use rdx_parser::parse;
use rdx_schema::{Schema, ComponentSchema, PropSchema, PropType, validate};

let schema = Schema::new()
    .strict(true)
    .component("Notice", ComponentSchema::new()
        .prop("type", PropSchema::enum_required(vec!["info", "warning", "error"]))
        .prop("title", PropSchema::optional(PropType::String))
    )
    .component("Badge", ComponentSchema::new()
        .self_closing(true)
        .prop("label", PropSchema::required(PropType::String))
    );

let root = parse("<Notice type=\"info\">\nSome text.\n</Notice>\n");
let diagnostics = validate(&root, &schema);

for d in &diagnostics {
    println!("{d}"); // Notice:1:1: error: missing required prop `type`
}
```

## What it validates

| Check | Severity |
|---|---|
| Missing required props | Error |
| Wrong prop type (e.g., string where object expected) | Error |
| Invalid enum value | Error |
| Children on a self-closing component | Error |
| Disallowed child components | Error |
| Unknown component (strict mode only) | Error |
| Unknown prop on a known component | Warning |

Variable attributes (`{$path}`) are accepted for any prop type since they resolve at runtime.

## Schema definition

### PropType

| Type | Matches |
|---|---|
| `String` | `label="text"` |
| `Number` | `count={42}` |
| `Boolean` | `active={true}` |
| `Enum` | String restricted to specific values |
| `Object` | `config={{"key": "value"}}` |
| `Array` | `items={{[1, 2, 3]}}` |
| `Variable` | `title={$frontmatter.title}` |
| `Any` | Accepts all value types |

### Component constraints

```rust
ComponentSchema::new()
    .self_closing(true)                      // must not have children
    .allowed_children(vec!["Tab", "TabPanel"]) // restrict child components
    .description("A tabbed container")
```

## JSON schemas

`Schema` is `Serialize + Deserialize`, so schemas can be stored as JSON config files:

```json
{
  "strict": true,
  "components": {
    "Notice": {
      "props": {
        "type": { "type": "enum", "required": true, "values": ["info", "warning", "error"] },
        "title": { "type": "string", "required": false }
      }
    }
  }
}
```

## Design

`rdx-schema` depends only on `rdx-ast`, not `rdx-parser`. It validates any `RdxRoot` regardless of which parser produced it. This keeps schema validation parser-agnostic.

## License

Licensed under either of [Apache License, Version 2.0](../LICENSE-APACHE) or [MIT License](../LICENSE-MIT) at your option.
