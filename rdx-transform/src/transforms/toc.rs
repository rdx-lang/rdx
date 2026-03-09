use rdx_ast::*;

use crate::{Transform, collect_text, walk};

/// Generates a table of contents from document headings and inserts it
/// as a `<TableOfContents>` component node.
///
/// The TOC is inserted at the position of an existing `<TableOfContents />`
/// placeholder component, or prepended after frontmatter if no placeholder exists
/// and `auto_insert` is true.
///
/// # Configuration
///
/// - `min_depth` / `max_depth`: heading levels to include (default: 2..=3)
/// - `auto_insert`: whether to insert TOC when no placeholder exists (default: false)
pub struct TableOfContents {
    pub min_depth: u8,
    pub max_depth: u8,
    pub auto_insert: bool,
}

impl Default for TableOfContents {
    fn default() -> Self {
        TableOfContents {
            min_depth: 2,
            max_depth: 3,
            auto_insert: false,
        }
    }
}

/// A single entry in the generated table of contents.
#[derive(Debug, Clone)]
struct TocEntry {
    depth: u8,
    text: String,
    id: Option<String>,
}

impl Transform for TableOfContents {
    fn name(&self) -> &str {
        "table-of-contents"
    }

    fn transform(&self, root: &mut Root, _source: &str) {
        let entries = collect_headings(&root.children, self.min_depth, self.max_depth);
        if entries.is_empty() {
            return;
        }

        let toc_node = build_toc_list(&entries);

        // Look for a <TableOfContents /> placeholder
        let placeholder_idx = root.children.iter().position(|n| {
            matches!(n, Node::Component(c) if c.name == "TableOfContents" && c.children.is_empty())
        });

        if let Some(idx) = placeholder_idx {
            root.children[idx] = toc_node;
        } else if self.auto_insert {
            root.children.insert(0, toc_node);
        }
    }
}

fn collect_headings(nodes: &[Node], min: u8, max: u8) -> Vec<TocEntry> {
    let mut entries = Vec::new();
    walk(nodes, &mut |node| {
        if let Node::Heading(h) = node
            && let Some(depth) = h.depth
                && depth >= min && depth <= max {
                    entries.push(TocEntry {
                        depth,
                        text: collect_text(&h.children),
                        id: h.id.clone(),
                    });
                }
    });
    entries
}

fn build_toc_list(entries: &[TocEntry]) -> Node {
    // Build a flat list of links, using depth for nested structure
    let mut items = Vec::new();
    // Synthetic position for generated nodes — use usize::MAX to clearly
    // distinguish from parser-produced positions (which are 1-based).
    let pos = Position {
        start: Point {
            line: usize::MAX,
            column: usize::MAX,
            offset: usize::MAX,
        },
        end: Point {
            line: usize::MAX,
            column: usize::MAX,
            offset: usize::MAX,
        },
    };

    for entry in entries {
        let href = entry
            .id
            .as_ref()
            .map(|id| format!("#{}", id))
            .unwrap_or_default();

        let link = Node::Link(LinkNode {
            url: href,
            title: None,
            children: vec![Node::Text(TextNode {
                value: entry.text.clone(),
                position: pos.clone(),
            })],
            position: pos.clone(),
        });

        let list_item = Node::ListItem(StandardBlockNode {
            depth: Some(entry.depth),
            ordered: None,
            checked: None,
            id: None,
            children: vec![link],
            position: pos.clone(),
        });

        items.push(list_item);
    }

    Node::Component(ComponentNode {
        name: "TableOfContents".to_string(),
        is_inline: false,
        attributes: vec![],
        children: vec![Node::List(StandardBlockNode {
            depth: None,
            ordered: None,
            checked: None,
            id: None,
            children: items,
            position: pos.clone(),
        })],
        position: pos,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AutoSlug, Pipeline};

    #[test]
    fn toc_from_headings() {
        let root = Pipeline::new()
            .add(AutoSlug::new())
            .add(TableOfContents {
                min_depth: 1,
                max_depth: 3,
                auto_insert: true,
            })
            .run("# Intro\n\n## Setup\n\n### Details\n\n## Usage\n");

        // First child should be the TOC component
        match &root.children[0] {
            Node::Component(c) => {
                assert_eq!(c.name, "TableOfContents");
                // Should have a list with 4 entries (h1 + h2 + h3 + h2)
                match &c.children[0] {
                    Node::List(l) => assert_eq!(l.children.len(), 4),
                    other => panic!("Expected list, got {:?}", other),
                }
            }
            other => panic!("Expected TOC component, got {:?}", other),
        }
    }

    #[test]
    fn toc_replaces_placeholder() {
        let root = Pipeline::new()
            .add(AutoSlug::new())
            .add(TableOfContents::default())
            .run("# Title\n\n<TableOfContents />\n\n## First\n\n## Second\n");

        // The placeholder should be replaced
        let toc = root.children.iter().find(|n| {
            matches!(n, Node::Component(c) if c.name == "TableOfContents" && !c.children.is_empty())
        });
        assert!(
            toc.is_some(),
            "TOC should replace placeholder: {:?}",
            root.children
        );
    }

    #[test]
    fn no_toc_without_placeholder_or_auto() {
        let root = Pipeline::new()
            .add(TableOfContents::default()) // auto_insert = false
            .run("## First\n\n## Second\n");

        let has_toc = root
            .children
            .iter()
            .any(|n| matches!(n, Node::Component(c) if c.name == "TableOfContents"));
        assert!(!has_toc, "Should not auto-insert TOC");
    }
}
