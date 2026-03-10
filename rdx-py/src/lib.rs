use pyo3::prelude::*;
use pyo3::types::PyAny;
use pythonize::pythonize;

/// Parse an RDX document and return the AST as a Python dict.
#[pyfunction]
fn parse<'py>(py: Python<'py>, input: &str) -> PyResult<Bound<'py, PyAny>> {
    let root = rdx_parser::parse(input);
    let val = serde_json::to_value(&root)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    pythonize(py, &val).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
}

/// Parse with default transforms (auto-slug + table of contents).
#[pyfunction]
fn parse_with_defaults<'py>(py: Python<'py>, input: &str) -> PyResult<Bound<'py, PyAny>> {
    let root = rdx_transform::parse_with_defaults(input);
    let val = serde_json::to_value(&root)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    pythonize(py, &val).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
}

/// Parse with a specific set of transforms.
#[pyfunction]
fn parse_with_transforms<'py>(
    py: Python<'py>,
    input: &str,
    transforms: Vec<String>,
) -> PyResult<Bound<'py, PyAny>> {
    let mut pipeline = rdx_transform::Pipeline::new();
    for name in &transforms {
        match name.as_str() {
            "auto-slug" => {
                pipeline = pipeline.add(rdx_transform::AutoSlug::new());
            }
            "toc" => {
                pipeline = pipeline.add(rdx_transform::TableOfContents::default());
            }
            other => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "unknown transform: \"{other}\""
                )));
            }
        }
    }
    let root = pipeline.run(input);
    let val = serde_json::to_value(&root)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    pythonize(py, &val).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
}

/// Validate an AST dict against a schema dict.
/// Returns a list of diagnostic dicts.
#[pyfunction]
fn validate<'py>(
    py: Python<'py>,
    ast: &Bound<'_, PyAny>,
    schema: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let ast_val: serde_json::Value = pythonize::depythonize(ast)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let schema_val: serde_json::Value = pythonize::depythonize(schema)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    let root: rdx_ast::Root = serde_json::from_value(ast_val)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let schema: rdx_schema::Schema = serde_json::from_value(schema_val)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    let diagnostics = rdx_schema::validate(&root, &schema);

    let results: Vec<serde_json::Value> = diagnostics
        .into_iter()
        .map(|d| {
            serde_json::json!({
                "severity": match d.severity {
                    rdx_schema::Severity::Error => "error",
                    rdx_schema::Severity::Warning => "warning",
                },
                "message": d.message,
                "component": d.component,
                "line": d.line,
                "column": d.column,
            })
        })
        .collect();

    let val = serde_json::to_value(&results)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    pythonize(py, &val).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
}

/// Extract plain text from an AST dict.
#[pyfunction]
fn collect_text(ast: &Bound<'_, PyAny>) -> PyResult<String> {
    let ast_val: serde_json::Value = pythonize::depythonize(ast)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let root: rdx_ast::Root = serde_json::from_value(ast_val)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    Ok(rdx_transform::collect_text(&root.children))
}

/// Find all nodes of a given type. Returns a list of node dicts.
#[pyfunction]
fn query_all<'py>(
    py: Python<'py>,
    ast: &Bound<'_, PyAny>,
    node_type: &str,
) -> PyResult<Bound<'py, PyAny>> {
    let ast_val: serde_json::Value = pythonize::depythonize(ast)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let root: rdx_ast::Root = serde_json::from_value(ast_val)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    let mut results: Vec<&rdx_ast::Node> = Vec::new();
    collect_by_type(&root.children, node_type, &mut results);

    let val = serde_json::to_value(&results)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    pythonize(py, &val).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
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

/// Return the RDX parser version.
#[pyfunction]
fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// RDX Python module.
#[pymodule]
fn _rdx(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(parse_with_defaults, m)?)?;
    m.add_function(wrap_pyfunction!(parse_with_transforms, m)?)?;
    m.add_function(wrap_pyfunction!(validate, m)?)?;
    m.add_function(wrap_pyfunction!(collect_text, m)?)?;
    m.add_function(wrap_pyfunction!(query_all, m)?)?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    Ok(())
}
