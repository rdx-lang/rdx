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

/// Return the version of the RDX parser.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
