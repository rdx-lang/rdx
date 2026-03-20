use rdx_ast::{Node, Root};
use std::collections::HashSet;

use crate::ast_util::walk_nodes;

// ---------------------------------------------------------------------------
// Statistics data
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct HeadingCounts {
    pub h1: u32,
    pub h2: u32,
    pub h3: u32,
    pub h4: u32,
    pub h5: u32,
    pub h6: u32,
}

impl HeadingCounts {
    pub fn total(&self) -> u32 {
        self.h1 + self.h2 + self.h3 + self.h4 + self.h5 + self.h6
    }
}

#[derive(Debug, Clone, Default)]
pub struct DocumentStats {
    pub words: u64,
    pub characters: u64,
    pub paragraphs: u64,
    pub headings: HeadingCounts,
    pub images: u64,
    pub tables: u64,
    pub code_blocks: u64,
    pub math_inline: u64,
    pub math_display: u64,
    pub citations: u64,
    pub unique_citation_keys: u64,
    pub cross_refs: u64,
    pub components: u64,
    pub footnote_definitions: u64,
}

// ---------------------------------------------------------------------------
// Stats collection
// ---------------------------------------------------------------------------

/// Walk the entire AST and accumulate document statistics into a
/// [`DocumentStats`] value.
pub fn collect_stats(root: &Root) -> DocumentStats {
    let mut stats = DocumentStats::default();
    let mut seen_citation_keys: HashSet<String> = HashSet::new();

    walk_nodes(&root.children, &mut |node| match node {
        Node::Text(t) => {
            // Count words by splitting on whitespace.
            stats.words += t.value.split_whitespace().count() as u64;
            // Count all characters (not bytes).
            stats.characters += t.value.chars().count() as u64;
        }
        Node::Paragraph(_) => {
            stats.paragraphs += 1;
        }
        Node::Heading(b) => {
            match b.depth {
                Some(1) => stats.headings.h1 += 1,
                Some(2) => stats.headings.h2 += 1,
                Some(3) => stats.headings.h3 += 1,
                Some(4) => stats.headings.h4 += 1,
                Some(5) => stats.headings.h5 += 1,
                Some(6) => stats.headings.h6 += 1,
                _ => {}
            }
        }
        Node::Image(_) => {
            stats.images += 1;
        }
        Node::Table(_) => {
            stats.tables += 1;
        }
        Node::CodeBlock(_) => {
            stats.code_blocks += 1;
        }
        Node::MathInline(_) => {
            stats.math_inline += 1;
        }
        Node::MathDisplay(_) => {
            stats.math_display += 1;
        }
        Node::Citation(c) => {
            stats.citations += c.keys.len() as u64;
            for key in &c.keys {
                seen_citation_keys.insert(key.id.clone());
            }
        }
        Node::CrossRef(_) => {
            stats.cross_refs += 1;
        }
        Node::Component(_) => {
            stats.components += 1;
        }
        Node::FootnoteDefinition(_) => {
            stats.footnote_definitions += 1;
        }
        _ => {}
    });

    stats.unique_citation_keys = seen_citation_keys.len() as u64;

    stats
}

// ---------------------------------------------------------------------------
// Formatted output
// ---------------------------------------------------------------------------

/// Format a u64 with thousands separators (e.g. 1234 -> "1,234").
fn fmt_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*c);
    }
    result
}

/// Format [`DocumentStats`] as a human-readable, aligned table string.
pub fn format_stats(stats: &DocumentStats) -> String {
    let mut out = String::new();

    // Only include non-zero heading levels in the detail string.
    let heading_parts: Vec<String> = [
        ("h1", stats.headings.h1),
        ("h2", stats.headings.h2),
        ("h3", stats.headings.h3),
        ("h4", stats.headings.h4),
        ("h5", stats.headings.h5),
        ("h6", stats.headings.h6),
    ]
    .iter()
    .filter(|(_, count)| *count > 0)
    .map(|(level, count)| format!("{}: {}", level, count))
    .collect();

    let heading_detail = if heading_parts.is_empty() {
        String::new()
    } else {
        format!(" ({})", heading_parts.join(", "))
    };

    let citation_detail = if stats.unique_citation_keys > 0 {
        format!(" ({} unique keys)", stats.unique_citation_keys)
    } else {
        String::new()
    };

    let label_w = 16usize;

    out.push_str(&format!(
        "{:<label_w$}  {}\n",
        "Words:",
        fmt_number(stats.words),
        label_w = label_w,
    ));
    out.push_str(&format!(
        "{:<label_w$}  {}\n",
        "Characters:",
        fmt_number(stats.characters),
        label_w = label_w,
    ));
    out.push_str(&format!(
        "{:<label_w$}  {}\n",
        "Paragraphs:",
        fmt_number(stats.paragraphs),
        label_w = label_w,
    ));
    out.push_str(&format!(
        "{:<label_w$}  {}{}\n",
        "Headings:",
        fmt_number(stats.headings.total() as u64),
        heading_detail,
        label_w = label_w,
    ));
    out.push_str(&format!(
        "{:<label_w$}  {}\n",
        "Images:",
        fmt_number(stats.images),
        label_w = label_w,
    ));
    out.push_str(&format!(
        "{:<label_w$}  {}\n",
        "Tables:",
        fmt_number(stats.tables),
        label_w = label_w,
    ));
    out.push_str(&format!(
        "{:<label_w$}  {}\n",
        "Code blocks:",
        fmt_number(stats.code_blocks),
        label_w = label_w,
    ));
    out.push_str(&format!(
        "{:<label_w$}  {}\n",
        "Math (inline):",
        fmt_number(stats.math_inline),
        label_w = label_w,
    ));
    out.push_str(&format!(
        "{:<label_w$}  {}\n",
        "Math (display):",
        fmt_number(stats.math_display),
        label_w = label_w,
    ));
    out.push_str(&format!(
        "{:<label_w$}  {}{}\n",
        "Citations:",
        fmt_number(stats.citations),
        citation_detail,
        label_w = label_w,
    ));
    out.push_str(&format!(
        "{:<label_w$}  {}\n",
        "Cross-refs:",
        fmt_number(stats.cross_refs),
        label_w = label_w,
    ));
    out.push_str(&format!(
        "{:<label_w$}  {}\n",
        "Components:",
        fmt_number(stats.components),
        label_w = label_w,
    ));
    out.push_str(&format!(
        "{:<label_w$}  {}\n",
        "Footnotes:",
        fmt_number(stats.footnote_definitions),
        label_w = label_w,
    ));

    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rdx_ast::*;
    use crate::test_helpers::{span, make_root};

    fn text_node(value: &str, line: usize) -> Node {
        Node::Text(TextNode {
            value: value.to_string(),
            position: span(line, 1, 0, line, value.len() + 1, value.len()),
        })
    }

    fn heading_node(depth: u8, line: usize) -> Node {
        Node::Heading(StandardBlockNode {
            depth: Some(depth),
            ordered: None,
            checked: None,
            id: None,
            children: vec![],
            position: span(line, 1, 0, line, 20, 19),
        })
    }

    // --- word count ---

    #[test]
    fn word_count_single_text_node() {
        let root = make_root(vec![text_node("hello world foo", 1)]);
        let stats = collect_stats(&root);
        assert_eq!(stats.words, 3);
    }

    #[test]
    fn word_count_multiple_text_nodes() {
        let root = make_root(vec![
            Node::Paragraph(StandardBlockNode {
                depth: None,
                ordered: None,
                checked: None,
                id: None,
                children: vec![
                    text_node("one two", 1),
                    text_node("three four five", 1),
                ],
                position: span(1, 1, 0, 1, 30, 29),
            }),
        ]);
        let stats = collect_stats(&root);
        assert_eq!(stats.words, 5);
    }

    #[test]
    fn word_count_empty_text() {
        let root = make_root(vec![text_node("", 1)]);
        let stats = collect_stats(&root);
        assert_eq!(stats.words, 0);
    }

    // --- character count ---

    #[test]
    fn character_count() {
        let root = make_root(vec![text_node("hello", 1)]);
        let stats = collect_stats(&root);
        assert_eq!(stats.characters, 5);
    }

    // --- heading counts ---

    #[test]
    fn heading_counts_by_level() {
        let root = make_root(vec![
            heading_node(1, 1),
            heading_node(2, 5),
            heading_node(2, 10),
            heading_node(3, 15),
        ]);
        let stats = collect_stats(&root);
        assert_eq!(stats.headings.h1, 1);
        assert_eq!(stats.headings.h2, 2);
        assert_eq!(stats.headings.h3, 1);
        assert_eq!(stats.headings.total(), 4);
    }

    // --- citation unique keys ---

    #[test]
    fn citation_unique_keys_deduplicated() {
        let root = make_root(vec![
            Node::Citation(CitationNode {
                keys: vec![
                    CitationKey { id: "smith2024".into(), prefix: None, locator: None },
                    CitationKey { id: "jones2023".into(), prefix: None, locator: None },
                ],
                position: span(8, 1, 0, 8, 30, 29),
            }),
            Node::Citation(CitationNode {
                keys: vec![
                    CitationKey { id: "smith2024".into(), prefix: None, locator: None },
                ],
                position: span(15, 1, 0, 15, 15, 14),
            }),
        ]);
        let stats = collect_stats(&root);
        // 3 total citation references, 2 unique keys.
        assert_eq!(stats.citations, 3);
        assert_eq!(stats.unique_citation_keys, 2);
    }

    // --- component count ---

    #[test]
    fn component_count() {
        let root = make_root(vec![
            Node::Component(ComponentNode {
                name: "Figure".into(),
                is_inline: false,
                attributes: vec![],
                children: vec![],
                raw_content: String::new(),
                position: span(1, 1, 0, 3, 1, 50),
            }),
            Node::Component(ComponentNode {
                name: "Table".into(),
                is_inline: false,
                attributes: vec![],
                children: vec![],
                raw_content: String::new(),
                position: span(5, 1, 0, 7, 1, 100),
            }),
        ]);
        let stats = collect_stats(&root);
        assert_eq!(stats.components, 2);
    }

    // --- fmt_number ---

    #[test]
    fn fmt_number_small() {
        assert_eq!(fmt_number(0), "0");
        assert_eq!(fmt_number(999), "999");
    }

    #[test]
    fn fmt_number_thousands() {
        assert_eq!(fmt_number(1000), "1,000");
        assert_eq!(fmt_number(1234), "1,234");
        assert_eq!(fmt_number(1234567), "1,234,567");
    }

    // --- format_stats output ---

    #[test]
    fn format_stats_contains_key_fields() {
        let root = make_root(vec![
            text_node("hello world", 1),
            heading_node(1, 2),
            heading_node(2, 3),
        ]);
        let stats = collect_stats(&root);
        let output = format_stats(&stats);

        assert!(output.contains("Words:"));
        assert!(output.contains("Headings:"));
        assert!(output.contains("h1: 1"));
        assert!(output.contains("h2: 1"));
    }
}
