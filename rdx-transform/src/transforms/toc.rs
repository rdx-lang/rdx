use rdx_ast::*;

use crate::{Transform, collect_text, synthetic_pos, walk};

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
/// - `numbered`: if true, prefix entries with hierarchical numbers (1, 1.1, 1.1.2)
pub struct TableOfContents {
    pub min_depth: u8,
    pub max_depth: u8,
    pub auto_insert: bool,
    pub numbered: bool,
}

impl Default for TableOfContents {
    fn default() -> Self {
        TableOfContents {
            min_depth: 2,
            max_depth: 3,
            auto_insert: false,
            numbered: false,
        }
    }
}

/// A single entry in the generated table of contents.
#[derive(Debug, Clone)]
struct TocEntry {
    depth: u8,
    text: String,
    id: Option<String>,
    /// Hierarchical number like "1.2.3", set when `numbered` is true.
    number: Option<String>,
}

impl Transform for TableOfContents {
    fn name(&self) -> &str {
        "table-of-contents"
    }

    fn transform(&self, root: &mut Root, _source: &str) {
        let mut entries = collect_headings(&root.children, self.min_depth, self.max_depth);
        if entries.is_empty() {
            return;
        }

        if self.numbered {
            assign_numbers(&mut entries);
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
            && depth >= min
            && depth <= max
        {
            entries.push(TocEntry {
                depth,
                text: collect_text(&h.children),
                id: h.id.clone(),
                number: None,
            });
        }
    });
    entries
}

/// Assign hierarchical numbers (1, 1.1, 1.1.2, 2, 2.1, etc.) to TOC entries.
fn assign_numbers(entries: &mut [TocEntry]) {
    if entries.is_empty() {
        return;
    }
    let base_depth = entries.iter().map(|e| e.depth).min().unwrap();
    // Counters indexed by relative depth (0 = shallowest heading in range)
    let mut counters = vec![0u32; 7]; // supports up to 6 nesting levels

    for entry in entries.iter_mut() {
        let rel = (entry.depth - base_depth) as usize;
        counters[rel] += 1;
        // Reset all deeper counters
        for c in counters.iter_mut().skip(rel + 1) {
            *c = 0;
        }
        // Build number string from counters[0..=rel]
        let parts: Vec<String> = counters[..=rel].iter().map(|n| n.to_string()).collect();
        entry.number = Some(parts.join("."));
    }
}

fn build_toc_list(entries: &[TocEntry]) -> Node {
    // Build a flat list of links, using depth for nested structure
    let mut items = Vec::new();
    // Synthetic position for generated nodes (line 0 / col 0 / offset 0
    // distinguishes them from parser-produced positions, which are 1-based).
    let pos = synthetic_pos();

    for entry in entries {
        let href = entry
            .id
            .as_ref()
            .map(|id| format!("#{}", id))
            .unwrap_or_default();

        let display = match &entry.number {
            Some(num) => format!("{} {}", num, entry.text),
            None => entry.text.clone(),
        };

        let link = Node::Link(LinkNode {
            url: href,
            title: None,
            children: vec![Node::Text(TextNode {
                value: display,
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
        raw_content: String::new(),
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
                numbered: false,
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
    fn numbered_toc() {
        let root = Pipeline::new()
            .add(AutoSlug::new())
            .add(TableOfContents {
                min_depth: 1,
                max_depth: 3,
                auto_insert: true,
                numbered: true,
            })
            .run("# Intro\n\n## Setup\n\n### Details\n\n## Usage\n\n# Advanced\n\n## Config\n");

        // First child should be the TOC
        if let Node::Component(c) = &root.children[0] {
            if let Node::List(l) = &c.children[0] {
                // Extract link text from each list item
                let texts: Vec<String> = l
                    .children
                    .iter()
                    .filter_map(|item| {
                        if let Node::ListItem(li) = item {
                            if let Node::Link(link) = &li.children[0] {
                                if let Node::Text(t) = &link.children[0] {
                                    return Some(t.value.clone());
                                }
                            }
                        }
                        None
                    })
                    .collect();
                assert_eq!(texts[0], "1 Intro");
                assert_eq!(texts[1], "1.1 Setup");
                assert_eq!(texts[2], "1.1.1 Details");
                assert_eq!(texts[3], "1.2 Usage");
                assert_eq!(texts[4], "2 Advanced");
                assert_eq!(texts[5], "2.1 Config");
            }
        }
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
