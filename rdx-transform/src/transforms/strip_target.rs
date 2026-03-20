use rdx_ast::*;

use crate::Transform;

// ---------------------------------------------------------------------------
// Transform
// ---------------------------------------------------------------------------

/// Removes nodes that are targeted at an output that does not match the
/// current render target.
///
/// Rules applied to every [`Node::Component`]:
///
/// | Component name / attribute | `target="web"` | `target="print"` |
/// |---------------------------|---------------|-----------------|
/// | `target="web"` attr       | **kept**      | removed          |
/// | `target="print"` attr     | removed       | **kept**          |
/// | `target="all"` or absent  | **kept**      | **kept**          |
/// | Name == `WebOnly`         | **kept**      | removed          |
/// | Name == `PrintOnly`       | removed       | **kept**          |
///
/// The component is removed by filtering it out of its parent's `children`
/// vector.  All other node types are kept and recursed into.
///
/// # Example
///
/// ```rust
/// use rdx_transform::{StripTarget, Transform, parse};
///
/// let mut root = parse("<WebOnly>\nweb content\n</WebOnly>\n<PrintOnly>\nprint\n</PrintOnly>\n");
/// StripTarget { target: "web".into() }.transform(&mut root, "");
/// // Only the WebOnly component remains.
/// assert_eq!(root.children.len(), 1);
/// ```
pub struct StripTarget {
    /// The current output target, e.g. `"web"` or `"print"`.
    pub target: String,
}

/// Returns `true` if this component should be **removed** for `current_target`.
fn should_strip(comp: &ComponentNode, current_target: &str) -> bool {
    // Special component names take precedence.
    match comp.name.as_str() {
        "WebOnly" => return current_target != "web",
        "PrintOnly" => return current_target != "print",
        _ => {}
    }

    // Fall back to the `target` attribute.
    let target_attr = comp.attributes.iter().find_map(|a| {
        if a.name == "target" {
            if let AttributeValue::String(s) = &a.value {
                Some(s.as_str())
            } else {
                None
            }
        } else {
            None
        }
    });

    match target_attr {
        Some("all") | None => false,
        Some(t) => t != current_target,
    }
}

impl Transform for StripTarget {
    fn name(&self) -> &str {
        "strip-target"
    }

    fn transform(&self, root: &mut Root, _source: &str) {
        strip_nodes(&mut root.children, &self.target);
    }
}

fn strip_nodes(nodes: &mut Vec<Node>, current_target: &str) {
    nodes.retain_mut(|node| {
        if let Node::Component(comp) = &*node {
            if should_strip(comp, current_target) {
                return false; // Drop this node.
            }
        }
        true
    });

    // Now recurse into surviving nodes' children.
    for node in nodes.iter_mut() {
        if let Some(children) = node.children_mut() {
            strip_nodes(children, current_target);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rdx_parser::parse;

    #[test]
    fn web_mode_strips_print_only() {
        let mut root = parse(
            "<PrintOnly>\nprint content\n</PrintOnly>\n\
             <WebOnly>\nweb content\n</WebOnly>\n",
        );
        StripTarget { target: "web".into() }.transform(&mut root, "");
        assert_eq!(root.children.len(), 1, "Only WebOnly should remain");
        match &root.children[0] {
            Node::Component(c) => assert_eq!(c.name, "WebOnly"),
            other => panic!("Expected WebOnly, got {:?}", other),
        }
    }

    #[test]
    fn print_mode_strips_web_only() {
        let mut root = parse(
            "<PrintOnly>\nprint content\n</PrintOnly>\n\
             <WebOnly>\nweb content\n</WebOnly>\n",
        );
        StripTarget { target: "print".into() }.transform(&mut root, "");
        assert_eq!(root.children.len(), 1, "Only PrintOnly should remain");
        match &root.children[0] {
            Node::Component(c) => assert_eq!(c.name, "PrintOnly"),
            other => panic!("Expected PrintOnly, got {:?}", other),
        }
    }

    #[test]
    fn target_attribute_web_stripped_in_print() {
        let mut root = parse("<Note target=\"web\">\ncontent\n</Note>\n");
        StripTarget { target: "print".into() }.transform(&mut root, "");
        assert!(root.children.is_empty(), "web-targeted Note should be removed in print mode");
    }

    #[test]
    fn target_attribute_print_kept_in_print() {
        let mut root = parse("<Note target=\"print\">\ncontent\n</Note>\n");
        StripTarget { target: "print".into() }.transform(&mut root, "");
        assert_eq!(root.children.len(), 1);
    }

    #[test]
    fn target_all_always_kept() {
        let mut root = parse("<Note target=\"all\">\ncontent\n</Note>\n");
        StripTarget { target: "web".into() }.transform(&mut root, "");
        assert_eq!(root.children.len(), 1);
        StripTarget { target: "print".into() }.transform(&mut root, "");
        assert_eq!(root.children.len(), 1);
    }

    #[test]
    fn no_target_attribute_always_kept() {
        let mut root = parse("<Notice>\ncontent\n</Notice>\n");
        StripTarget { target: "web".into() }.transform(&mut root, "");
        assert_eq!(root.children.len(), 1);
    }

    #[test]
    fn nested_strip_works() {
        // A neutral outer component contains a PrintOnly child.
        let mut root = parse(
            "<Notice>\n\
             <PrintOnly>\nprint\n</PrintOnly>\n\
             Keep this.\n\
             </Notice>\n",
        );
        StripTarget { target: "web".into() }.transform(&mut root, "");
        // Outer Notice must still be present.
        assert_eq!(root.children.len(), 1);
        match &root.children[0] {
            Node::Component(c) => {
                // PrintOnly child should have been stripped.
                let has_print_only = c
                    .children
                    .iter()
                    .any(|n| matches!(n, Node::Component(inner) if inner.name == "PrintOnly"));
                assert!(!has_print_only, "PrintOnly should be stripped from nested position");
            }
            other => panic!("Expected Notice, got {:?}", other),
        }
    }

    #[test]
    fn both_kept_when_target_matches() {
        let mut root = parse(
            "<WebOnly>\nweb\n</WebOnly>\n\
             <PrintOnly>\nprint\n</PrintOnly>\n",
        );
        // Neither should be stripped because there are no conflicting targets.
        // Run with a third unknown target: both should be stripped.
        StripTarget { target: "pdf".into() }.transform(&mut root, "");
        assert!(
            root.children.is_empty(),
            "Both WebOnly and PrintOnly should be stripped for unknown target"
        );
    }
}
