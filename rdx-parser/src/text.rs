use rdx_ast::*;

use crate::source_map::SourceMap;

/// Process text content for variable interpolation and escape sequences.
///
/// Per spec 2.4.2: no processing inside code spans or fenced code blocks.
/// Per spec 2.5: handles `\{$path}`, `\{{`, `\}}`, `\{`, `\\` escapes.
/// Backslash before any other character is passed through as-is.
pub(crate) fn process_text(
    text: &str,
    start_offset: usize,
    sm: &SourceMap,
    in_code: bool,
) -> Vec<Node> {
    if text.is_empty() {
        return vec![];
    }

    if in_code {
        return vec![Node::Text(TextNode {
            value: text.to_string(),
            position: sm.position(start_offset, start_offset + text.len()),
        })];
    }

    let bytes = text.as_bytes();
    let mut nodes = Vec::new();
    let mut buf = String::new();
    let mut buf_start = start_offset;
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            let next = bytes[i + 1];
            // Spec 2.5 escape sequences (checked in order of specificity)
            if next == b'{' && i + 2 < bytes.len() && bytes[i + 2] == b'$' {
                // \{$path} -> literal {$path}
                if let Some(close_rel) = text[i + 2..].find('}') {
                    let literal = &text[i + 1..i + 2 + close_rel + 1]; // {$...}
                    flush_text_buf(&mut nodes, &mut buf, buf_start, sm);
                    nodes.push(Node::Text(TextNode {
                        value: literal.to_string(),
                        position: sm
                            .position(start_offset + i, start_offset + i + 2 + close_rel + 1),
                    }));
                    i = i + 2 + close_rel + 1;
                    buf_start = start_offset + i;
                    continue;
                }
                // No closing brace — treat backslash as literal, fall through
                buf.push('\\');
                i += 1;
                continue;
            } else if next == b'{' && i + 2 < bytes.len() && bytes[i + 2] == b'{' {
                // \{{ -> literal {{
                flush_text_buf(&mut nodes, &mut buf, buf_start, sm);
                nodes.push(Node::Text(TextNode {
                    value: "{{".to_string(),
                    position: sm.position(start_offset + i, start_offset + i + 3),
                }));
                i += 3;
                buf_start = start_offset + i;
                continue;
            } else if next == b'}' && i + 2 < bytes.len() && bytes[i + 2] == b'}' {
                // \}} -> literal }}
                flush_text_buf(&mut nodes, &mut buf, buf_start, sm);
                nodes.push(Node::Text(TextNode {
                    value: "}}".to_string(),
                    position: sm.position(start_offset + i, start_offset + i + 3),
                }));
                i += 3;
                buf_start = start_offset + i;
                continue;
            } else if next == b'{' {
                // \{ -> literal {
                flush_text_buf(&mut nodes, &mut buf, buf_start, sm);
                nodes.push(Node::Text(TextNode {
                    value: "{".to_string(),
                    position: sm.position(start_offset + i, start_offset + i + 2),
                }));
                i += 2;
                buf_start = start_offset + i;
                continue;
            } else if next == b'\\' {
                // \\ -> literal \
                flush_text_buf(&mut nodes, &mut buf, buf_start, sm);
                nodes.push(Node::Text(TextNode {
                    value: "\\".to_string(),
                    position: sm.position(start_offset + i, start_offset + i + 2),
                }));
                i += 2;
                buf_start = start_offset + i;
                continue;
            }
            // Not a recognized escape — pass backslash through as-is (spec 2.5)
            buf.push('\\');
            i += 1;
            continue;
        }

        if bytes[i] == b'{' && i + 1 < bytes.len() && bytes[i + 1] == b'$' {
            // Variable interpolation {$path} (spec 2.4)
            if let Some(close_rel) = text[i..].find('}') {
                let path = &text[i + 2..i + close_rel]; // skip {$
                let var_start = start_offset + i;
                let var_end = start_offset + i + close_rel + 1;
                flush_text_buf(&mut nodes, &mut buf, buf_start, sm);
                if crate::is_valid_variable_path(path) {
                    nodes.push(Node::Variable(VariableNode {
                        path: path.to_string(),
                        position: sm.position(var_start, var_end),
                    }));
                } else {
                    // Invalid variable path -> error per spec 3.5
                    nodes.push(Node::Error(ErrorNode {
                        message: format!("Invalid variable path: {}", path),
                        raw_content: text[i..i + close_rel + 1].to_string(),
                        position: sm.position(var_start, var_end),
                    }));
                }
                i = i + close_rel + 1;
                buf_start = start_offset + i;
                continue;
            }
        }

        // Inline math: $...$ (but NOT {$ which is variable syntax)
        if bytes[i] == b'$' && (i == 0 || bytes[i - 1] != b'{') {
            // Find closing $ (not $$, not escaped)
            if let Some(close_rel) = text[i + 1..].find('$') {
                let math_content = &text[i + 1..i + 1 + close_rel];
                // Must have non-empty content and not start/end with space
                if !math_content.is_empty()
                    && !math_content.starts_with(' ')
                    && !math_content.ends_with(' ')
                {
                    let math_start = start_offset + i;
                    let math_end = start_offset + i + 1 + close_rel + 1;
                    flush_text_buf(&mut nodes, &mut buf, buf_start, sm);
                    nodes.push(Node::MathInline(TextNode {
                        value: math_content.to_string(),
                        position: sm.position(math_start, math_end),
                    }));
                    i = i + 1 + close_rel + 1;
                    buf_start = start_offset + i;
                    continue;
                }
            }
        }

        // Regular character — handle multi-byte UTF-8
        let ch = text[i..].chars().next().unwrap();
        buf.push(ch);
        i += ch.len_utf8();
    }

    flush_text_buf(&mut nodes, &mut buf, buf_start, sm);
    nodes
}

fn flush_text_buf(nodes: &mut Vec<Node>, buf: &mut String, buf_start: usize, sm: &SourceMap) {
    if !buf.is_empty() {
        let text = std::mem::take(buf);
        let len = text.len();
        nodes.push(Node::Text(TextNode {
            value: text,
            position: sm.position(buf_start, buf_start + len),
        }));
    }
}

#[cfg(test)]
mod tests {
    use crate::parse;
    use rdx_ast::*;

    #[test]
    fn variable_interpolation() {
        let root = parse("Hello {$name}!\n");
        match &root.children[0] {
            Node::Paragraph(p) => {
                let has_var = p
                    .children
                    .iter()
                    .any(|n| matches!(n, Node::Variable(v) if v.path == "name"));
                assert!(has_var, "Should contain variable node: {:?}", p.children);
            }
            other => panic!("Expected paragraph, got {:?}", other),
        }
    }

    #[test]
    fn variable_not_in_inline_code() {
        let root = parse("`{$name}`\n");
        match &root.children[0] {
            Node::Paragraph(p) => match &p.children[0] {
                Node::CodeInline(t) => assert_eq!(t.value, "{$name}"),
                other => panic!("Expected code_inline, got {:?}", other),
            },
            other => panic!("Expected paragraph, got {:?}", other),
        }
    }

    #[test]
    fn variable_not_in_code_block() {
        let root = parse("```\n{$variable}\n```\n");
        fn find_code_block(nodes: &[Node]) -> Option<&CodeBlockNode> {
            for node in nodes {
                if let Node::CodeBlock(t) = node {
                    return Some(t);
                }
                if let Node::Paragraph(b) | Node::Blockquote(b) = node {
                    if let Some(t) = find_code_block(&b.children) {
                        return Some(t);
                    }
                }
            }
            None
        }
        let code = find_code_block(&root.children).expect("Should have code block");
        assert_eq!(code.value, "{$variable}\n");
    }

    #[test]
    fn escape_variable() {
        let root = parse("\\{$path}\n");
        match &root.children[0] {
            Node::Paragraph(p) => {
                let has_var = p.children.iter().any(|n| matches!(n, Node::Variable(_)));
                assert!(!has_var, "Escaped variable should not be a Variable node");
                let text: String = p
                    .children
                    .iter()
                    .filter_map(|n| {
                        if let Node::Text(t) = n {
                            Some(t.value.as_str())
                        } else {
                            None
                        }
                    })
                    .collect();
                assert!(
                    text.contains("{$path}"),
                    "Should contain literal {{$path}}, got: {}",
                    text
                );
            }
            other => panic!("Expected paragraph, got {:?}", other),
        }
    }

    #[test]
    fn escape_double_braces() {
        let root = parse("\\{{ and \\}}\n");
        match &root.children[0] {
            Node::Paragraph(p) => {
                let text: String = p
                    .children
                    .iter()
                    .filter_map(|n| {
                        if let Node::Text(t) = n {
                            Some(t.value.as_str())
                        } else {
                            None
                        }
                    })
                    .collect();
                assert!(text.contains("{{"), "got: {}", text);
                assert!(text.contains("}}"), "got: {}", text);
            }
            other => panic!("Expected paragraph, got {:?}", other),
        }
    }

    #[test]
    fn escape_single_brace() {
        let root = parse("\\{not a var\n");
        match &root.children[0] {
            Node::Paragraph(p) => {
                let text: String = p
                    .children
                    .iter()
                    .filter_map(|n| {
                        if let Node::Text(t) = n {
                            Some(t.value.as_str())
                        } else {
                            None
                        }
                    })
                    .collect();
                assert!(text.contains("{"), "got: {}", text);
            }
            other => panic!("Expected paragraph, got {:?}", other),
        }
    }

    #[test]
    fn escape_backslash() {
        let root = parse("A \\\\ B\n");
        match &root.children[0] {
            Node::Paragraph(p) => {
                let text: String = p
                    .children
                    .iter()
                    .filter_map(|n| {
                        if let Node::Text(t) = n {
                            Some(t.value.as_str())
                        } else {
                            None
                        }
                    })
                    .collect();
                assert!(
                    text.contains("\\"),
                    "Should contain literal backslash, got: {}",
                    text
                );
            }
            other => panic!("Expected paragraph, got {:?}", other),
        }
    }

    #[test]
    fn backslash_before_non_special_passthrough() {
        let root = parse("\\a\n");
        match &root.children[0] {
            Node::Paragraph(p) => {
                let text: String = p
                    .children
                    .iter()
                    .filter_map(|n| {
                        if let Node::Text(t) = n {
                            Some(t.value.as_str())
                        } else {
                            None
                        }
                    })
                    .collect();
                assert!(
                    text.contains("\\a"),
                    "Should pass through \\a as-is, got: {}",
                    text
                );
            }
            other => panic!("Expected paragraph, got {:?}", other),
        }
    }

    #[test]
    fn invalid_variable_path_produces_error() {
        let root = parse("Hello {$123invalid}!\n");
        fn has_error(nodes: &[Node]) -> bool {
            nodes.iter().any(|n| match n {
                Node::Error(_) => true,
                Node::Paragraph(b)
                | Node::Heading(b)
                | Node::List(b)
                | Node::ListItem(b)
                | Node::Blockquote(b)
                | Node::Html(b)
                | Node::Table(b)
                | Node::TableRow(b)
                | Node::TableCell(b)
                | Node::Emphasis(b)
                | Node::Strong(b)
                | Node::Strikethrough(b) => has_error(&b.children),
                Node::Link(l) => has_error(&l.children),
                Node::Image(i) => has_error(&i.children),
                Node::Component(c) => has_error(&c.children),
                Node::FootnoteDefinition(f) => has_error(&f.children),
                _ => false,
            })
        }
        assert!(
            has_error(&root.children),
            "Should have error: {:?}",
            root.children
        );
    }
}
