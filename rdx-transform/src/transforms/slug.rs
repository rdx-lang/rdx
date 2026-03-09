use rdx_ast::*;

use crate::{Transform, collect_text};

/// Generates URL-safe slugs for headings and sets the `id` field.
///
/// Slug algorithm:
/// 1. Extract plain text from heading children
/// 2. Lowercase, replace non-alphanumeric runs with `-`, trim `-`
/// 3. Deduplicate by appending `-1`, `-2`, etc.
///
/// # Example
///
/// ```rust
/// use rdx_transform::{Pipeline, AutoSlug, parse};
///
/// let root = Pipeline::new().add(AutoSlug::new()).run("# Hello World\n");
/// // heading.id == Some("hello-world")
/// ```
pub struct AutoSlug {
    _private: (),
}

impl AutoSlug {
    pub fn new() -> Self {
        AutoSlug { _private: () }
    }
}

impl Default for AutoSlug {
    fn default() -> Self {
        Self::new()
    }
}

impl Transform for AutoSlug {
    fn name(&self) -> &str {
        "auto-slug"
    }

    fn transform(&self, root: &mut Root, _source: &str) {
        let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        slugify_headings(&mut root.children, &mut seen);
    }
}

fn slugify_headings(nodes: &mut [Node], seen: &mut std::collections::HashMap<String, usize>) {
    for node in nodes.iter_mut() {
        match node {
            Node::Heading(block) => {
                if block.id.is_none() {
                    let text = collect_text(&block.children);
                    let base = to_slug(&text);
                    if base.is_empty() {
                        continue;
                    }
                    let count = seen.entry(base.clone()).or_insert(0);
                    let slug = if *count == 0 {
                        base
                    } else {
                        format!("{}-{}", base, count)
                    };
                    *count += 1;
                    block.id = Some(slug);
                }
                // Recurse into heading children (unlikely to contain headings, but be thorough)
                slugify_headings(&mut block.children, seen);
            }
            Node::Paragraph(b)
            | Node::List(b)
            | Node::ListItem(b)
            | Node::Blockquote(b)
            | Node::Html(b)
            | Node::Table(b)
            | Node::TableRow(b)
            | Node::TableCell(b)
            | Node::Emphasis(b)
            | Node::Strong(b)
            | Node::Strikethrough(b)
            | Node::ThematicBreak(b) => {
                slugify_headings(&mut b.children, seen);
            }
            Node::Link(l) => slugify_headings(&mut l.children, seen),
            Node::Image(i) => slugify_headings(&mut i.children, seen),
            Node::Component(c) => slugify_headings(&mut c.children, seen),
            Node::FootnoteDefinition(f) => slugify_headings(&mut f.children, seen),
            _ => {}
        }
    }
}

fn to_slug(text: &str) -> String {
    let mut slug = String::with_capacity(text.len());
    let mut prev_dash = true; // prevent leading dash
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            slug.push('-');
            prev_dash = true;
        }
    }
    // Trim trailing dash
    if slug.ends_with('-') {
        slug.pop();
    }
    slug
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_generation() {
        assert_eq!(to_slug("Hello World"), "hello-world");
        assert_eq!(to_slug("API v2.0 Reference"), "api-v2-0-reference");
        assert_eq!(to_slug("  Leading Spaces  "), "leading-spaces");
        assert_eq!(to_slug("camelCase"), "camelcase");
        assert_eq!(to_slug("ALLCAPS"), "allcaps");
    }

    #[test]
    fn auto_slug_sets_id() {
        let mut root = rdx_parser::parse("# Hello\n\n## World\n");
        let slug = AutoSlug::new();
        slug.transform(&mut root, "");
        match &root.children[0] {
            Node::Heading(h) => assert_eq!(h.id.as_deref(), Some("hello")),
            other => panic!("Expected heading, got {:?}", other),
        }
    }

    #[test]
    fn preserves_existing_id() {
        let mut root = rdx_parser::parse("# Test\n");
        // Manually set an id
        if let Node::Heading(ref mut h) = root.children[0] {
            h.id = Some("custom-id".to_string());
        }
        AutoSlug::new().transform(&mut root, "");
        match &root.children[0] {
            Node::Heading(h) => assert_eq!(h.id.as_deref(), Some("custom-id")),
            other => panic!("Expected heading, got {:?}", other),
        }
    }
}
