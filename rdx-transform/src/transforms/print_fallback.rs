use rdx_ast::*;

use crate::{Transform, synthetic_pos};

// ---------------------------------------------------------------------------
// Transform
// ---------------------------------------------------------------------------

/// When producing print output, replaces components that carry a
/// `printFallback` attribute with a simpler representation.
///
/// - If the fallback value looks like an image path (ends with `.png`, `.jpg`,
///   `.jpeg`, `.gif`, `.svg`, or `.webp`) the component is replaced with a
///   [`Node::Image`].
/// - Otherwise the component is replaced with a [`Node::Text`] containing the
///   fallback string.
///
/// Components without a `printFallback` attribute are left unchanged.
///
/// The transform itself is **stateless** — the caller is responsible for
/// applying it only when targeting print output.
///
/// # Example
///
/// ```rust
/// use rdx_transform::{PrintFallback, Transform, parse};
///
/// let mut root = parse(
///     "<InteractiveChart printFallback=\"chart.png\" />\n",
/// );
/// PrintFallback.transform(&mut root, "");
/// // The component is now an Image node pointing to "chart.png".
/// assert!(matches!(root.children[0], rdx_transform::Node::Image(_)));
/// ```
pub struct PrintFallback;

impl Transform for PrintFallback {
    fn name(&self) -> &str {
        "print-fallback"
    }

    fn transform(&self, root: &mut Root, _source: &str) {
        replace_fallbacks(&mut root.children);
    }
}

/// Return true if the path looks like it refers to a raster or vector image.
fn looks_like_image(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".gif")
        || lower.ends_with(".svg")
        || lower.ends_with(".webp")
}

fn replace_fallbacks(nodes: &mut Vec<Node>) {
    for node in nodes.iter_mut() {
        if let Node::Component(ref comp) = *node {
            // Look for a `printFallback` attribute with a string value.
            let fallback = comp.attributes.iter().find_map(|a| {
                if a.name == "printFallback" {
                    if let AttributeValue::String(s) = &a.value {
                        Some(s.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            });

            if let Some(fb) = fallback {
                let replacement = if looks_like_image(&fb) {
                    Node::Image(ImageNode {
                        url: fb,
                        title: None,
                        alt: None,
                        children: vec![],
                        position: synthetic_pos(),
                    })
                } else {
                    Node::Text(TextNode {
                        value: fb,
                        position: synthetic_pos(),
                    })
                };
                *node = replacement;
                continue;
            }
        }

        // Recurse into children of nodes that were NOT replaced.
        if let Some(children) = node.children_mut() {
            replace_fallbacks(children);
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
    fn text_fallback_replaces_component() {
        let mut root = parse("<Widget printFallback=\"Widget content\" />\n");
        PrintFallback.transform(&mut root, "");
        match &root.children[0] {
            Node::Text(t) => assert_eq!(t.value, "Widget content"),
            other => panic!("Expected Text, got {:?}", other),
        }
    }

    #[test]
    fn image_fallback_for_png_path() {
        let mut root = parse("<Chart printFallback=\"chart.png\" />\n");
        PrintFallback.transform(&mut root, "");
        match &root.children[0] {
            Node::Image(i) => assert_eq!(i.url, "chart.png"),
            other => panic!("Expected Image, got {:?}", other),
        }
    }

    #[test]
    fn image_fallback_for_svg_path() {
        let mut root = parse("<Diagram printFallback=\"diagram.svg\" />\n");
        PrintFallback.transform(&mut root, "");
        match &root.children[0] {
            Node::Image(i) => assert_eq!(i.url, "diagram.svg"),
            other => panic!("Expected Image, got {:?}", other),
        }
    }

    #[test]
    fn image_fallback_for_jpg_path() {
        let mut root = parse("<Photo printFallback=\"photo.jpg\" />\n");
        PrintFallback.transform(&mut root, "");
        match &root.children[0] {
            Node::Image(i) => assert_eq!(i.url, "photo.jpg"),
            other => panic!("Expected Image, got {:?}", other),
        }
    }

    #[test]
    fn no_fallback_attribute_keeps_component() {
        let mut root = parse("<Widget />\n");
        PrintFallback.transform(&mut root, "");
        match &root.children[0] {
            Node::Component(c) => assert_eq!(c.name, "Widget"),
            other => panic!("Expected Component, got {:?}", other),
        }
    }

    #[test]
    fn nested_fallback_replaced() {
        let mut root = parse(
            "<Outer>\n\
             <Chart printFallback=\"inner.png\" />\n\
             </Outer>\n",
        );
        PrintFallback.transform(&mut root, "");
        match &root.children[0] {
            Node::Component(outer) => {
                assert_eq!(outer.name, "Outer");
                match &outer.children[0] {
                    Node::Image(i) => assert_eq!(i.url, "inner.png"),
                    other => panic!("Expected Image inside Outer, got {:?}", other),
                }
            }
            other => panic!("Expected Outer component, got {:?}", other),
        }
    }

    #[test]
    fn case_insensitive_extension_check() {
        // uppercase extension should still be detected as an image path.
        let mut root = parse("<Chart printFallback=\"chart.PNG\" />\n");
        PrintFallback.transform(&mut root, "");
        assert!(
            matches!(&root.children[0], Node::Image(_)),
            "Expected Image for .PNG extension"
        );
    }
}
