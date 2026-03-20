use rdx_ast::*;

use crate::{Transform, synthetic_pos};
use crate::transforms::auto_number::{NumberEntry, NumberRegistry};

// ---------------------------------------------------------------------------
// Transform
// ---------------------------------------------------------------------------

/// Resolves [`Node::CrossRef`] nodes by looking up their `target` label in a
/// [`NumberRegistry`] produced by [`crate::AutoNumber`].
///
/// For the "web" target a `CrossRef` is replaced by a [`Node::Link`] whose
/// `url` is `#<target>` and whose display text is e.g. `"Figure 1"`.
///
/// For the "print" target the node is replaced by a plain [`Node::Text`]
/// containing the same display text (page numbers would be added by a
/// downstream typesetter).
///
/// If the target label is **not** found in the registry the `CrossRef` node is
/// left in place unchanged so that downstream tools can handle it or report the
/// error.
///
/// # Example
///
/// ```rust
/// use rdx_transform::{AutoNumber, CrossRefResolve, Transform, parse};
///
/// let mut root = parse(
///     "<Figure id=\"fig:arch\">\n</Figure>\n\
///      See {@fig:arch}.\n",
/// );
/// let numberer = AutoNumber::new();
/// numberer.transform(&mut root, "");
/// let registry = numberer.registry().entries.clone();
/// let resolver = CrossRefResolve::new(
///     rdx_transform::NumberRegistry { entries: registry },
///     "web",
/// );
/// resolver.transform(&mut root, "");
/// ```
pub struct CrossRefResolve {
    pub registry: NumberRegistry,
    /// Output target: `"web"` or `"print"`.
    pub target: String,
}

impl CrossRefResolve {
    pub fn new(registry: NumberRegistry, target: impl Into<String>) -> Self {
        CrossRefResolve {
            registry,
            target: target.into(),
        }
    }
}

impl Transform for CrossRefResolve {
    fn name(&self) -> &str {
        "cross-ref-resolve"
    }

    fn transform(&self, root: &mut Root, _source: &str) {
        resolve_nodes(&mut root.children, &self.registry, &self.target);
    }
}

// ---------------------------------------------------------------------------
// Display text helper
// ---------------------------------------------------------------------------

fn display_text(entry: &NumberEntry) -> String {
    format!("{} {}", entry.kind, entry.number)
}

// ---------------------------------------------------------------------------
// Recursive walker
// ---------------------------------------------------------------------------

fn resolve_nodes(nodes: &mut Vec<Node>, registry: &NumberRegistry, target: &str) {
    let mut i = 0;
    while i < nodes.len() {
        let replacement = if let Node::CrossRef(ref cr) = nodes[i] {
            if let Some(entry) = registry.entries.get(&cr.target) {
                let text = display_text(entry);
                if target == "print" {
                    Some(Node::Text(TextNode {
                        value: text,
                        position: synthetic_pos(),
                    }))
                } else {
                    // "web" (or any non-print target): produce a Link node.
                    Some(Node::Link(LinkNode {
                        url: format!("#{}", cr.target),
                        title: None,
                        children: vec![Node::Text(TextNode {
                            value: text,
                            position: synthetic_pos(),
                        })],
                        position: synthetic_pos(),
                    }))
                }
            } else {
                None // Not found — leave the CrossRef in place.
            }
        } else {
            None
        };

        if let Some(new_node) = replacement {
            nodes[i] = new_node;
        } else {
            // Recurse into children of non-CrossRef nodes.
            if let Some(children) = nodes[i].children_mut() {
                resolve_nodes(children, registry, target);
            }
        }
        i += 1;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::transforms::auto_number::NumberEntry;

    fn make_registry(entries: Vec<(&str, &str, &str)>) -> NumberRegistry {
        let mut map = HashMap::new();
        for (label, kind, number) in entries {
            map.insert(
                label.to_string(),
                NumberEntry {
                    kind: kind.to_string(),
                    number: number.to_string(),
                    title: None,
                },
            );
        }
        NumberRegistry { entries: map }
    }

    fn cross_ref_node(target: &str) -> Node {
        Node::CrossRef(CrossRefNode {
            target: target.to_string(),
            position: Position {
                start: Point { line: 1, column: 1, offset: 0 },
                end: Point { line: 1, column: 1, offset: 0 },
            },
        })
    }

    fn root_with_children(children: Vec<Node>) -> Root {
        Root {
            node_type: RootType::Root,
            frontmatter: None,
            children,
            position: Position {
                start: Point { line: 1, column: 1, offset: 0 },
                end: Point { line: 1, column: 1, offset: 0 },
            },
        }
    }

    #[test]
    fn resolves_known_ref_to_link_for_web() {
        let registry = make_registry(vec![("fig:arch", "Figure", "1")]);
        let resolver = CrossRefResolve::new(registry, "web");

        let mut root = root_with_children(vec![cross_ref_node("fig:arch")]);
        resolver.transform(&mut root, "");

        match &root.children[0] {
            Node::Link(l) => {
                assert_eq!(l.url, "#fig:arch");
                match &l.children[0] {
                    Node::Text(t) => assert_eq!(t.value, "Figure 1"),
                    other => panic!("Expected text child, got {:?}", other),
                }
            }
            other => panic!("Expected Link, got {:?}", other),
        }
    }

    #[test]
    fn resolves_known_ref_to_text_for_print() {
        let registry = make_registry(vec![("thm:main", "Theorem", "3")]);
        let resolver = CrossRefResolve::new(registry, "print");

        let mut root = root_with_children(vec![cross_ref_node("thm:main")]);
        resolver.transform(&mut root, "");

        match &root.children[0] {
            Node::Text(t) => assert_eq!(t.value, "Theorem 3"),
            other => panic!("Expected Text, got {:?}", other),
        }
    }

    #[test]
    fn unknown_ref_left_unchanged() {
        let registry = make_registry(vec![]);
        let resolver = CrossRefResolve::new(registry, "web");

        let mut root = root_with_children(vec![cross_ref_node("fig:unknown")]);
        resolver.transform(&mut root, "");

        assert!(
            matches!(&root.children[0], Node::CrossRef(cr) if cr.target == "fig:unknown"),
            "Expected CrossRef to remain, got {:?}",
            root.children[0]
        );
    }

    #[test]
    fn resolves_cross_ref_nested_in_paragraph() {
        let registry = make_registry(vec![("eq:euler", "Equation", "2")]);
        let resolver = CrossRefResolve::new(registry, "web");

        let para = Node::Paragraph(StandardBlockNode {
            depth: None,
            ordered: None,
            checked: None,
            id: None,
            children: vec![cross_ref_node("eq:euler")],
            position: Position {
                start: Point { line: 1, column: 1, offset: 0 },
                end: Point { line: 1, column: 1, offset: 0 },
            },
        });

        let mut root = root_with_children(vec![para]);
        resolver.transform(&mut root, "");

        match &root.children[0] {
            Node::Paragraph(p) => match &p.children[0] {
                Node::Link(l) => {
                    assert_eq!(l.url, "#eq:euler");
                }
                other => panic!("Expected link inside paragraph, got {:?}", other),
            },
            other => panic!("Expected paragraph, got {:?}", other),
        }
    }

    #[test]
    fn multiple_refs_resolved_independently() {
        let registry = make_registry(vec![
            ("fig:a", "Figure", "1"),
            ("fig:b", "Figure", "2"),
        ]);
        let resolver = CrossRefResolve::new(registry, "web");

        let mut root = root_with_children(vec![
            cross_ref_node("fig:a"),
            cross_ref_node("fig:b"),
            cross_ref_node("fig:unknown"),
        ]);
        resolver.transform(&mut root, "");

        assert!(matches!(&root.children[0], Node::Link(l) if l.url == "#fig:a"));
        assert!(matches!(&root.children[1], Node::Link(l) if l.url == "#fig:b"));
        assert!(matches!(&root.children[2], Node::CrossRef(_)));
    }
}
