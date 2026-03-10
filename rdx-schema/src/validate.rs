use rdx_ast::{AttributeValue, ComponentNode, Node, Root};

use crate::{PropType, Schema, type_matches, value_type_name};

/// Severity level for validation diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// A single validation diagnostic tied to a source location.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    /// Component name that caused the diagnostic.
    pub component: String,
    /// Line number in the source document (1-indexed).
    pub line: usize,
    /// Column number in the source document (1-indexed).
    pub column: usize,
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let level = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(
            f,
            "{}:{}:{}: {}: {}",
            self.component, self.line, self.column, level, self.message
        )
    }
}

/// Validate an RDX AST against a schema. Returns a list of diagnostics.
///
/// An empty list means the document is valid.
///
/// # Example
///
/// ```rust
/// use rdx_schema::{Schema, ComponentSchema, PropSchema, PropType, validate};
/// use rdx_parser::parse;
///
/// let schema = Schema::new()
///     .strict(true)
///     .component("Notice", ComponentSchema::new()
///         .prop("type", PropSchema::enum_required(vec!["info", "warning", "error"]))
///     );
///
/// let root = parse("<Notice type=\"info\">\nSome text.\n</Notice>\n");
/// let diagnostics = validate(&root, &schema);
/// assert!(diagnostics.is_empty());
/// ```
pub fn validate(root: &Root, schema: &Schema) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    validate_nodes(&root.children, schema, &mut diagnostics, None);
    diagnostics
}

fn validate_nodes(
    nodes: &[Node],
    schema: &Schema,
    diagnostics: &mut Vec<Diagnostic>,
    parent_allowed_children: Option<&[String]>,
) {
    for node in nodes {
        if let Node::Component(comp) = node {
            validate_component(comp, schema, diagnostics, parent_allowed_children);
        }
        // Recurse into non-component nodes that have children
        if !matches!(node, Node::Component(_)) {
            if let Some(children) = node.children() {
                validate_nodes(children, schema, diagnostics, None);
            }
        }
    }
}

fn validate_component(
    comp: &ComponentNode,
    schema: &Schema,
    diagnostics: &mut Vec<Diagnostic>,
    parent_allowed_children: Option<&[String]>,
) {
    let line = comp.position.start.line;
    let column = comp.position.start.column;
    let name = &comp.name;

    // Check if this component is allowed as a child of its parent
    if let Some(allowed) = parent_allowed_children {
        if !allowed.iter().any(|a| a == name) {
            diagnostics.push(Diagnostic {
                severity: Severity::Error,
                message: format!("<{name}> is not allowed as a child here"),
                component: name.clone(),
                line,
                column,
            });
        }
    }

    let Some(comp_schema) = schema.components.get(name.as_str()) else {
        // Unknown component
        if schema.strict {
            diagnostics.push(Diagnostic {
                severity: Severity::Error,
                message: format!("unknown component <{name}>"),
                component: name.clone(),
                line,
                column,
            });
        }
        return;
    };

    // Check self-closing constraint
    if comp_schema.self_closing && !comp.children.is_empty() {
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            message: format!("<{name}> must be self-closing (no children)"),
            component: name.clone(),
            line,
            column,
        });
    }

    // Check required props
    for (prop_name, prop_schema) in &comp_schema.props {
        if prop_schema.required {
            let found = comp.attributes.iter().any(|a| a.name == *prop_name);
            if !found {
                diagnostics.push(Diagnostic {
                    severity: Severity::Error,
                    message: format!("missing required prop `{prop_name}`"),
                    component: name.clone(),
                    line,
                    column,
                });
            }
        }
    }

    // Check each provided attribute
    for attr in &comp.attributes {
        let attr_line = attr.position.start.line;
        let attr_col = attr.position.start.column;

        let Some(prop_schema) = comp_schema.props.get(&attr.name) else {
            // Unknown prop
            diagnostics.push(Diagnostic {
                severity: Severity::Warning,
                message: format!("unknown prop `{}` on <{name}>", attr.name),
                component: name.clone(),
                line: attr_line,
                column: attr_col,
            });
            continue;
        };

        // Type check (skip variables — they resolve at runtime)
        if matches!(attr.value, AttributeValue::Variable(_))
            && prop_schema.prop_type != PropType::Variable
        {
            // Variables are accepted for any type — the host resolves them
            continue;
        }

        if !type_matches(&attr.value, &prop_schema.prop_type) {
            diagnostics.push(Diagnostic {
                severity: Severity::Error,
                message: format!(
                    "prop `{}` on <{name}> expects {}, got {}",
                    attr.name,
                    format_expected_type(&prop_schema.prop_type),
                    value_type_name(&attr.value),
                ),
                component: name.clone(),
                line: attr_line,
                column: attr_col,
            });
        }

        // Enum value check
        if prop_schema.prop_type == PropType::Enum {
            if let (Some(allowed), AttributeValue::String(val)) = (&prop_schema.values, &attr.value)
            {
                if !allowed.contains(val) {
                    diagnostics.push(Diagnostic {
                        severity: Severity::Error,
                        message: format!(
                            "prop `{}` on <{name}> must be one of [{}], got \"{}\"",
                            attr.name,
                            allowed.join(", "),
                            val,
                        ),
                        component: name.clone(),
                        line: attr_line,
                        column: attr_col,
                    });
                }
            }
        }
    }

    // Recurse into children with allowed_children constraint
    validate_nodes(
        &comp.children,
        schema,
        diagnostics,
        comp_schema.allowed_children.as_deref(),
    );
}

fn format_expected_type(t: &PropType) -> &'static str {
    match t {
        PropType::String => "string",
        PropType::Number => "number",
        PropType::Boolean => "boolean",
        PropType::Enum => "string (enum)",
        PropType::Object => "object",
        PropType::Array => "array",
        PropType::Variable => "variable",
        PropType::Any => "any",
    }
}
