pub use rdx_ast::*;

mod attributes;
mod frontmatter;
mod markdown;
mod scanner;
mod source_map;
mod tags;
mod text;

use scanner::Segment;
use source_map::SourceMap;

/// Parse an RDX document string into a compliant AST.
///
/// This is the primary entry point for the parser. It handles:
/// - YAML frontmatter extraction (spec 2.1)
/// - CommonMark block structure via pulldown-cmark
/// - RDX component tags with typed attributes (spec 2.2, 2.3)
/// - Variable interpolation in text (spec 2.4)
/// - Escape sequences (spec 2.5)
/// - HTML pass-through for lowercase tags (spec 2.6)
/// - Error nodes for malformed constructs (spec 3)
pub fn parse(input: &str) -> Root {
    let sm = SourceMap::new(input);
    let (frontmatter, body_start) = frontmatter::extract_frontmatter(input);
    let body = &input[body_start..];
    let children = parse_body(body, body_start, &sm, input);

    Root {
        node_type: RootType::Root,
        frontmatter,
        children,
        position: sm.position(0, input.len()),
    }
}

/// Validate a variable path against spec 2.4.1 grammar:
/// `[a-zA-Z_][a-zA-Z0-9_]*(\.[a-zA-Z_][a-zA-Z0-9_]*)*`
pub(crate) fn is_valid_variable_path(path: &str) -> bool {
    if path.is_empty() {
        return false;
    }
    for segment in path.split('.') {
        if segment.is_empty() {
            return false;
        }
        let bytes = segment.as_bytes();
        if !bytes[0].is_ascii_alphabetic() && bytes[0] != b'_' {
            return false;
        }
        for &b in &bytes[1..] {
            if !b.is_ascii_alphanumeric() && b != b'_' {
                return false;
            }
        }
    }
    true
}

/// Recursively parse a body region into AST nodes.
/// Splits into markdown vs block-component segments, processes each accordingly.
fn parse_body(body: &str, base_offset: usize, sm: &SourceMap, full_input: &str) -> Vec<Node> {
    let segments = scanner::scan_segments(body, base_offset, sm);
    let mut nodes = Vec::new();

    for seg in segments {
        match seg {
            Segment::Markdown { start, end } => {
                let text = &full_input[start..end];
                nodes.extend(markdown::parse_markdown_region(text, start, sm, full_input));
            }
            Segment::BlockComponent {
                tag,
                body_start,
                body_end,
                close_end,
            } => {
                let inner = if body_start <= body_end {
                    &full_input[body_start..body_end]
                } else {
                    ""
                };
                let children = parse_body(inner, body_start, sm, full_input);
                nodes.push(Node::Component(ComponentNode {
                    name: tag.name,
                    is_inline: false,
                    attributes: tag.attributes,
                    children,
                    position: sm.position(tag.start, close_end),
                }));
            }
            Segment::BlockSelfClosing { tag } => {
                nodes.push(Node::Component(ComponentNode {
                    name: tag.name,
                    is_inline: false,
                    attributes: tag.attributes,
                    children: vec![],
                    position: sm.position(tag.start, tag.end),
                }));
            }
            Segment::MathBlock {
                value_start,
                value_end,
                block_end,
            } => {
                let value = if value_start <= value_end {
                    full_input[value_start..value_end].to_string()
                } else {
                    String::new()
                };
                nodes.push(Node::MathDisplay(TextNode {
                    value,
                    position: sm.position(value_start.saturating_sub(3), block_end), // include $$
                }));
            }
            Segment::Error {
                message,
                raw,
                start,
                end,
            } => {
                nodes.push(Node::Error(ErrorNode {
                    message,
                    raw_content: raw,
                    position: sm.position(start, end),
                }));
            }
        }
    }

    nodes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_document() {
        let root = parse("");
        assert_eq!(root.node_type, RootType::Root);
        assert_eq!(root.frontmatter, None);
        assert!(root.children.is_empty());
    }

    #[test]
    fn frontmatter_only() {
        let root = parse("---\ntitle: Hello\nversion: 2\n---\n");
        assert!(root.frontmatter.is_some());
        let fm = root.frontmatter.unwrap();
        assert_eq!(fm["title"], "Hello");
        assert_eq!(fm["version"], 2);
    }

    #[test]
    fn frontmatter_no_trailing_content() {
        let root = parse("---\nfoo: bar\n---");
        assert!(root.frontmatter.is_some());
        assert_eq!(root.frontmatter.unwrap()["foo"], "bar");
    }

    #[test]
    fn no_frontmatter_when_not_at_line1() {
        let root = parse("\n---\ntitle: Hello\n---\n");
        assert_eq!(root.frontmatter, None);
    }

    #[test]
    fn frontmatter_plus_content() {
        let input = "---\ntitle: Test\n---\n# Hello\n";
        let root = parse(input);
        assert!(root.frontmatter.is_some());
        assert_eq!(root.frontmatter.unwrap()["title"], "Test");
        assert!(matches!(&root.children[0], Node::Heading(_)));
    }

    #[test]
    fn pure_markdown_heading() {
        let root = parse("# Hello World\n");
        match &root.children[0] {
            Node::Heading(block) => {
                assert_eq!(block.depth, Some(1));
                match &block.children[0] {
                    Node::Text(t) => assert_eq!(t.value, "Hello World"),
                    other => panic!("Expected text, got {:?}", other),
                }
            }
            other => panic!("Expected heading, got {:?}", other),
        }
    }

    #[test]
    fn paragraph_with_emphasis() {
        let root = parse("This is **bold** text.\n");
        match &root.children[0] {
            Node::Paragraph(block) => {
                assert!(block.children.len() >= 3);
                assert!(matches!(&block.children[1], Node::Strong(_)));
            }
            other => panic!("Expected paragraph, got {:?}", other),
        }
    }

    #[test]
    fn self_closing_block_component() {
        let root = parse("<Badge status=\"beta\" />\n");
        match &root.children[0] {
            Node::Component(c) => {
                assert_eq!(c.name, "Badge");
                assert!(!c.is_inline);
                assert!(c.children.is_empty());
                assert_eq!(c.attributes[0].name, "status");
                assert_eq!(c.attributes[0].value, AttributeValue::String("beta".into()));
            }
            other => panic!("Expected component, got {:?}", other),
        }
    }

    #[test]
    fn block_component_with_children() {
        let root = parse("<Notice type=\"warning\">\nThis is a warning.\n</Notice>\n");
        match &root.children[0] {
            Node::Component(c) => {
                assert_eq!(c.name, "Notice");
                assert!(!c.is_inline);
                assert!(!c.children.is_empty());
            }
            other => panic!("Expected component, got {:?}", other),
        }
    }

    #[test]
    fn nested_components() {
        let root = parse("<Outer>\n<Inner>\nText\n</Inner>\n</Outer>\n");
        match &root.children[0] {
            Node::Component(outer) => {
                assert_eq!(outer.name, "Outer");
                match &outer.children[0] {
                    Node::Component(inner) => assert_eq!(inner.name, "Inner"),
                    other => panic!("Expected inner, got {:?}", other),
                }
            }
            other => panic!("Expected outer, got {:?}", other),
        }
    }

    #[test]
    fn inline_self_closing_component() {
        let root = parse("Text with <Badge /> inline.\n");
        match &root.children[0] {
            Node::Paragraph(p) => {
                let has_badge = p
                    .children
                    .iter()
                    .any(|n| matches!(n, Node::Component(c) if c.name == "Badge" && c.is_inline));
                assert!(has_badge, "Should contain inline Badge: {:?}", p.children);
            }
            other => panic!("Expected paragraph, got {:?}", other),
        }
    }

    #[test]
    fn unclosed_block_component() {
        let root = parse("<Notice>\nContent\n");
        let has_error = root.children.iter().any(|n| matches!(n, Node::Error(_)));
        assert!(
            has_error,
            "Should have error for unclosed tag: {:?}",
            root.children
        );
    }

    #[test]
    fn stray_close_tag() {
        let root = parse("</Notice>\n");
        let has_error = root.children.iter().any(|n| matches!(n, Node::Error(_)));
        assert!(
            has_error,
            "Should have error for stray close tag: {:?}",
            root.children
        );
    }

    #[test]
    fn html_passthrough() {
        let root = parse("<div>hello</div>\n");
        let has_component = root
            .children
            .iter()
            .any(|n| matches!(n, Node::Component(_)));
        assert!(
            !has_component,
            "Lowercase HTML should not be component: {:?}",
            root.children
        );
    }

    #[test]
    fn thematic_break_not_frontmatter() {
        let root = parse("Hello\n\n---\n\nWorld\n");
        assert_eq!(root.frontmatter, None);
        let has_break = root
            .children
            .iter()
            .any(|n| matches!(n, Node::ThematicBreak(_)));
        assert!(has_break);
    }

    #[test]
    fn position_tracking() {
        let root = parse("# Hi\n");
        assert_eq!(root.position.start.line, 1);
        assert_eq!(root.position.start.column, 1);
        assert_eq!(root.position.start.offset, 0);
    }

    #[test]
    fn mixed_markdown_and_components() {
        let input =
            "# Title\n\n<Notice type=\"info\">\nSome **bold** content.\n</Notice>\n\nMore text.\n";
        let root = parse(input);
        assert!(root.children.len() >= 3);
        assert!(matches!(&root.children[0], Node::Heading(_)));
        assert!(matches!(&root.children[1], Node::Component(_)));
        assert!(matches!(&root.children[2], Node::Paragraph(_)));
    }

    #[test]
    fn component_with_markdown_children() {
        let root = parse("<Notice>\n**Bold** and *italic*.\n</Notice>\n");
        match &root.children[0] {
            Node::Component(c) => {
                assert_eq!(c.name, "Notice");
                assert!(!c.children.is_empty());
            }
            other => panic!("Expected component, got {:?}", other),
        }
    }

    #[test]
    fn list_parsing() {
        let root = parse("- item 1\n- item 2\n");
        match &root.children[0] {
            Node::List(l) => {
                assert_eq!(l.ordered, Some(false));
                assert_eq!(l.children.len(), 2);
            }
            other => panic!("Expected list, got {:?}", other),
        }
    }

    #[test]
    fn ordered_list() {
        let root = parse("1. first\n2. second\n");
        match &root.children[0] {
            Node::List(l) => assert_eq!(l.ordered, Some(true)),
            other => panic!("Expected list, got {:?}", other),
        }
    }

    #[test]
    fn blockquote() {
        let root = parse("> quoted text\n");
        assert!(matches!(&root.children[0], Node::Blockquote(_)));
    }

    #[test]
    fn strikethrough() {
        let root = parse("~~deleted~~\n");
        match &root.children[0] {
            Node::Paragraph(p) => {
                assert!(
                    p.children
                        .iter()
                        .any(|n| matches!(n, Node::Strikethrough(_)))
                );
            }
            other => panic!("Expected paragraph, got {:?}", other),
        }
    }

    #[test]
    fn task_list() {
        let root = parse("- [x] done\n- [ ] todo\n");
        match &root.children[0] {
            Node::List(l) => {
                assert_eq!(l.children.len(), 2);
                match &l.children[0] {
                    Node::ListItem(li) => assert_eq!(li.checked, Some(true)),
                    other => panic!("Expected list item, got {:?}", other),
                }
                match &l.children[1] {
                    Node::ListItem(li) => assert_eq!(li.checked, Some(false)),
                    other => panic!("Expected list item, got {:?}", other),
                }
            }
            other => panic!("Expected list, got {:?}", other),
        }
    }

    #[test]
    fn link_with_url_and_title() {
        let root = parse("[click](https://example.com \"My Title\")\n");
        match &root.children[0] {
            Node::Paragraph(p) => match &p.children[0] {
                Node::Link(l) => {
                    assert_eq!(l.url, "https://example.com");
                    assert_eq!(l.title.as_deref(), Some("My Title"));
                    assert!(!l.children.is_empty());
                }
                other => panic!("Expected link, got {:?}", other),
            },
            other => panic!("Expected paragraph, got {:?}", other),
        }
    }

    #[test]
    fn image_with_url() {
        let root = parse("![alt text](image.png)\n");
        match &root.children[0] {
            Node::Paragraph(p) => match &p.children[0] {
                Node::Image(i) => {
                    assert_eq!(i.url, "image.png");
                }
                other => panic!("Expected image, got {:?}", other),
            },
            other => panic!("Expected paragraph, got {:?}", other),
        }
    }

    #[test]
    fn code_block_with_language() {
        let root = parse("```rust\nlet x = 1;\n```\n");
        match &root.children[0] {
            Node::CodeBlock(cb) => {
                assert_eq!(cb.lang.as_deref(), Some("rust"));
                assert_eq!(cb.value, "let x = 1;\n");
            }
            other => panic!("Expected code block, got {:?}", other),
        }
    }

    #[test]
    fn footnote() {
        let root = parse("Text[^1].\n\n[^1]: Footnote content.\n");
        let has_ref = root.children.iter().any(|n| {
            if let Node::Paragraph(p) = n {
                p.children
                    .iter()
                    .any(|c| matches!(c, Node::FootnoteReference(_)))
            } else {
                false
            }
        });
        let has_def = root
            .children
            .iter()
            .any(|n| matches!(n, Node::FootnoteDefinition(_)));
        assert!(
            has_ref,
            "Should have footnote reference: {:?}",
            root.children
        );
        assert!(
            has_def,
            "Should have footnote definition: {:?}",
            root.children
        );
    }

    #[test]
    fn inline_math() {
        let root = parse("The equation $x^2 + y^2 = z^2$ is famous.\n");
        match &root.children[0] {
            Node::Paragraph(p) => {
                let has_math = p
                    .children
                    .iter()
                    .any(|n| matches!(n, Node::MathInline(t) if t.value == "x^2 + y^2 = z^2"));
                assert!(has_math, "Should contain inline math: {:?}", p.children);
            }
            other => panic!("Expected paragraph, got {:?}", other),
        }
    }

    #[test]
    fn display_math_block() {
        let root = parse("$$\nE = mc^2\n$$\n");
        let has_math = root
            .children
            .iter()
            .any(|n| matches!(n, Node::MathDisplay(t) if t.value.contains("E = mc^2")));
        assert!(has_math, "Should contain display math: {:?}", root.children);
    }

    #[test]
    fn math_does_not_conflict_with_variables() {
        let root = parse("Price is {$amount} dollars.\n");
        match &root.children[0] {
            Node::Paragraph(p) => {
                let has_var = p
                    .children
                    .iter()
                    .any(|n| matches!(n, Node::Variable(v) if v.path == "amount"));
                let has_math = p.children.iter().any(|n| matches!(n, Node::MathInline(_)));
                assert!(has_var, "Should contain variable: {:?}", p.children);
                assert!(!has_math, "Should NOT contain math: {:?}", p.children);
            }
            other => panic!("Expected paragraph, got {:?}", other),
        }
    }

    #[test]
    fn variable_path_validation() {
        assert!(is_valid_variable_path("title"));
        assert!(is_valid_variable_path("frontmatter.title"));
        assert!(is_valid_variable_path("config.theme_name"));
        assert!(is_valid_variable_path("_private"));
        assert!(!is_valid_variable_path(""));
        assert!(!is_valid_variable_path("123abc"));
        assert!(!is_valid_variable_path("foo..bar"));
        assert!(!is_valid_variable_path(".foo"));
        assert!(!is_valid_variable_path("foo."));
    }

    #[test]
    fn mixed_fence_chars_not_cross_closed() {
        // A ~~~ fence should NOT be closed by ```
        let root = parse("~~~\nstill fenced\n```\nstill fenced\n~~~\n\nAfter fence.\n");
        // The ``` inside should be treated as content, not close the fence
        // "After fence." should be a separate paragraph, not part of code block content
        match &root.children[0] {
            Node::CodeBlock(cb) => {
                assert!(
                    cb.value.contains("```"),
                    "``` should be content inside ~~~ fence: {:?}",
                    cb.value
                );
                assert!(
                    cb.value.contains("still fenced"),
                    "Content should be preserved: {:?}",
                    cb.value
                );
            }
            other => panic!("Expected code block, got {:?}", other),
        }
    }
}
