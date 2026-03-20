use rdx_ast::{AttributeValue, Node, Root};
use std::collections::HashMap;

/// Recursively walk all nodes in the AST, calling the visitor for each.
///
/// The visitor is called on each node before descending into its children
/// (pre-order traversal).
pub fn walk_nodes<F: FnMut(&Node)>(nodes: &[Node], visitor: &mut F) {
    for node in nodes {
        visitor(node);
        if let Some(children) = node.children() {
            walk_nodes(children, visitor);
        }
    }
}

// ---------------------------------------------------------------------------
// Shared label types and collection
// ---------------------------------------------------------------------------

/// The semantic kind of a document label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabelKind {
    Heading,
    Component,
    Math,
}

impl std::fmt::Display for LabelKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LabelKind::Heading => write!(f, "heading"),
            LabelKind::Component => write!(f, "component"),
            LabelKind::Math => write!(f, "math"),
        }
    }
}

/// Metadata about a defined label in the document.
#[derive(Debug, Clone)]
pub struct LabelEntry {
    /// The label identifier (e.g. `"sec:intro"`, `"fig:arch"`).
    pub key: String,
    /// The semantic kind of the label (heading, component, math).
    pub kind: LabelKind,
    /// Source line where the label is defined (1-indexed).
    pub line: usize,
}

/// Collect all defined labels from headings, components with `id`, and
/// `MathDisplay` nodes with `label`.
///
/// Returns a map from label key to its metadata. If duplicate keys exist,
/// the last definition wins.
pub fn collect_labels(root: &Root) -> HashMap<String, LabelEntry> {
    let mut labels: HashMap<String, LabelEntry> = HashMap::new();

    walk_nodes(&root.children, &mut |node| match node {
        Node::Heading(b) => {
            if let Some(ref id) = b.id {
                labels.insert(
                    id.clone(),
                    LabelEntry {
                        key: id.clone(),
                        kind: LabelKind::Heading,
                        line: b.position.start.line,
                    },
                );
            }
        }
        Node::Component(c) => {
            for attr in &c.attributes {
                if attr.name == "id" {
                    if let AttributeValue::String(ref id) = attr.value {
                        labels.insert(
                            id.clone(),
                            LabelEntry {
                                key: id.clone(),
                                kind: LabelKind::Component,
                                line: c.position.start.line,
                            },
                        );
                    }
                }
            }
        }
        Node::MathDisplay(m) => {
            if let Some(ref label) = m.label {
                labels.insert(
                    label.clone(),
                    LabelEntry {
                        key: label.clone(),
                        kind: LabelKind::Math,
                        line: m.position.start.line,
                    },
                );
            }
        }
        _ => {}
    });

    labels
}
