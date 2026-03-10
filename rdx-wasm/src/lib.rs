use wasm_bindgen::prelude::*;

/// Parse an RDX document and return the AST as a JS object.
///
/// Returns an object matching the `RdxRoot` TypeScript interface.
#[wasm_bindgen]
pub fn parse(input: &str) -> Result<JsValue, JsError> {
    let root = rdx_parser::parse(input);
    serde_wasm_bindgen::to_value(&root).map_err(|e| JsError::new(&e.to_string()))
}

/// Parse an RDX document with default transforms (auto-slug, table of contents).
///
/// Returns an object matching the `RdxRoot` TypeScript interface.
#[wasm_bindgen(js_name = "parseWithDefaults")]
pub fn parse_with_defaults(input: &str) -> Result<JsValue, JsError> {
    let root = rdx_transform::parse_with_defaults(input);
    serde_wasm_bindgen::to_value(&root).map_err(|e| JsError::new(&e.to_string()))
}

/// Parse and apply a custom set of transforms.
///
/// `transforms` is an array of transform names. Supported values:
/// - `"auto-slug"` — generate heading IDs
/// - `"toc"` — generate table of contents
#[wasm_bindgen(js_name = "parseWithTransforms")]
pub fn parse_with_transforms(input: &str, transforms: &JsValue) -> Result<JsValue, JsError> {
    let names: Vec<String> = serde_wasm_bindgen::from_value(transforms.clone())
        .map_err(|e| JsError::new(&e.to_string()))?;

    let mut pipeline = rdx_transform::Pipeline::new();
    for name in &names {
        match name.as_str() {
            "auto-slug" => {
                pipeline = pipeline.add(rdx_transform::AutoSlug::new());
            }
            "toc" => {
                pipeline = pipeline.add(rdx_transform::TableOfContents::default());
            }
            other => {
                return Err(JsError::new(&format!("unknown transform: \"{other}\"")));
            }
        }
    }

    let root = pipeline.run(input);
    serde_wasm_bindgen::to_value(&root).map_err(|e| JsError::new(&e.to_string()))
}

/// Validate an RDX AST against a component schema.
///
/// `ast` must be an `RdxRoot` object (as returned by `parse`).
/// `schema` must be a schema object matching `Schema` shape:
///
/// ```js
/// {
///   strict: true,
///   components: {
///     "Notice": {
///       props: {
///         "type": { type: "enum", required: true, values: ["info", "warning", "error"] }
///       }
///     }
///   }
/// }
/// ```
///
/// Returns an array of diagnostic objects.
#[wasm_bindgen]
pub fn validate(ast: &JsValue, schema: &JsValue) -> Result<JsValue, JsError> {
    let root: rdx_ast::Root =
        serde_wasm_bindgen::from_value(ast.clone()).map_err(|e| JsError::new(&e.to_string()))?;
    let schema: rdx_schema::Schema =
        serde_wasm_bindgen::from_value(schema.clone()).map_err(|e| JsError::new(&e.to_string()))?;

    let diagnostics = rdx_schema::validate(&root, &schema);

    let results: Vec<DiagnosticJs> = diagnostics
        .into_iter()
        .map(|d| DiagnosticJs {
            severity: match d.severity {
                rdx_schema::Severity::Error => "error".to_string(),
                rdx_schema::Severity::Warning => "warning".to_string(),
            },
            message: d.message,
            component: d.component,
            line: d.line,
            column: d.column,
        })
        .collect();

    serde_wasm_bindgen::to_value(&results).map_err(|e| JsError::new(&e.to_string()))
}

#[derive(serde::Serialize)]
struct DiagnosticJs {
    severity: String,
    message: String,
    component: String,
    line: usize,
    column: usize,
}

/// Walk an RDX AST and collect all plain text content.
///
/// Useful for search indexing, reading time estimation, or generating summaries.
/// Returns the concatenated text from all `TextNode` entries in the tree.
#[wasm_bindgen(js_name = "collectText")]
pub fn collect_text(ast: &JsValue) -> Result<String, JsError> {
    let root: rdx_ast::Root =
        serde_wasm_bindgen::from_value(ast.clone()).map_err(|e| JsError::new(&e.to_string()))?;
    Ok(rdx_transform::collect_text(&root.children))
}

/// Walk an RDX AST and return a flat array of all nodes matching a given type.
///
/// `node_type` is a string like `"component"`, `"heading"`, `"text"`, `"variable"`, etc.
///
/// ```js
/// const headings = queryAll(ast, "heading");
/// const components = queryAll(ast, "component");
/// ```
#[wasm_bindgen(js_name = "queryAll")]
pub fn query_all(ast: &JsValue, node_type: &str) -> Result<JsValue, JsError> {
    let root: rdx_ast::Root =
        serde_wasm_bindgen::from_value(ast.clone()).map_err(|e| JsError::new(&e.to_string()))?;

    let mut results: Vec<&rdx_ast::Node> = Vec::new();
    collect_by_type(&root.children, node_type, &mut results);

    serde_wasm_bindgen::to_value(&results).map_err(|e| JsError::new(&e.to_string()))
}

fn collect_by_type<'a>(
    nodes: &'a [rdx_ast::Node],
    node_type: &str,
    results: &mut Vec<&'a rdx_ast::Node>,
) {
    for node in nodes {
        if node_type_matches(node, node_type) {
            results.push(node);
        }
        if let Some(children) = node.children() {
            collect_by_type(children, node_type, results);
        }
    }
}

#[allow(clippy::match_like_matches_macro)]
fn node_type_matches(node: &rdx_ast::Node, expected: &str) -> bool {
    match (node, expected) {
        (rdx_ast::Node::Text(_), "text") => true,
        (rdx_ast::Node::CodeInline(_), "code_inline") => true,
        (rdx_ast::Node::CodeBlock(_), "code_block") => true,
        (rdx_ast::Node::Paragraph(_), "paragraph") => true,
        (rdx_ast::Node::Heading(_), "heading") => true,
        (rdx_ast::Node::List(_), "list") => true,
        (rdx_ast::Node::ListItem(_), "list_item") => true,
        (rdx_ast::Node::Blockquote(_), "blockquote") => true,
        (rdx_ast::Node::ThematicBreak(_), "thematic_break") => true,
        (rdx_ast::Node::Html(_), "html") => true,
        (rdx_ast::Node::Table(_), "table") => true,
        (rdx_ast::Node::TableRow(_), "table_row") => true,
        (rdx_ast::Node::TableCell(_), "table_cell") => true,
        (rdx_ast::Node::Link(_), "link") => true,
        (rdx_ast::Node::Image(_), "image") => true,
        (rdx_ast::Node::Emphasis(_), "emphasis") => true,
        (rdx_ast::Node::Strong(_), "strong") => true,
        (rdx_ast::Node::Strikethrough(_), "strikethrough") => true,
        (rdx_ast::Node::FootnoteDefinition(_), "footnote_definition") => true,
        (rdx_ast::Node::FootnoteReference(_), "footnote_reference") => true,
        (rdx_ast::Node::MathInline(_), "math_inline") => true,
        (rdx_ast::Node::MathDisplay(_), "math_display") => true,
        (rdx_ast::Node::Component(_), "component") => true,
        (rdx_ast::Node::Variable(_), "variable") => true,
        (rdx_ast::Node::Error(_), "error") => true,
        _ => false,
    }
}

/// Return the version of the RDX parser.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
