use rdx_ast::*;

/// Format an RDX AST back into a normalized RDX document string.
///
/// The formatter produces canonical output:
/// - Consistent blank lines between block-level nodes
/// - Normalized attribute quoting (strings always double-quoted)
/// - Consistent component tag formatting
/// - Preserved frontmatter, code blocks, and math blocks
pub fn format_root(root: &Root) -> String {
    let mut out = String::new();

    // Frontmatter
    if let Some(ref fm) = root.frontmatter {
        out.push_str("---\n");
        // Serialize frontmatter as YAML-like key: value pairs
        if let serde_json::Value::Object(map) = fm {
            format_yaml_object(&mut out, map, 0);
        }
        out.push_str("---\n");
        if !root.children.is_empty() {
            out.push('\n');
        }
    }

    format_block_children(&mut out, &root.children, 0);

    // Ensure trailing newline
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }

    out
}

fn format_yaml_object(
    out: &mut String,
    map: &serde_json::Map<String, serde_json::Value>,
    indent: usize,
) {
    let prefix = "  ".repeat(indent);
    for (key, value) in map {
        out.push_str(&prefix);
        out.push_str(key);
        out.push_str(": ");
        format_yaml_value(out, value, indent);
        out.push('\n');
    }
}

fn format_yaml_value(out: &mut String, value: &serde_json::Value, indent: usize) {
    match value {
        serde_json::Value::Null => out.push_str("null"),
        serde_json::Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        serde_json::Value::Number(n) => out.push_str(&n.to_string()),
        serde_json::Value::String(s) => {
            // Use bare string if safe, otherwise quote
            if s.contains(':')
                || s.contains('#')
                || s.contains('\n')
                || s.contains('"')
                || s.contains('\'')
                || s.starts_with(' ')
                || s.ends_with(' ')
                || s.is_empty()
                || s == "true"
                || s == "false"
                || s == "null"
            {
                out.push('"');
                out.push_str(&s.replace('\\', "\\\\").replace('"', "\\\""));
                out.push('"');
            } else {
                out.push_str(s);
            }
        }
        serde_json::Value::Array(arr) => {
            out.push('\n');
            let prefix = "  ".repeat(indent + 1);
            for item in arr {
                out.push_str(&prefix);
                out.push_str("- ");
                format_yaml_value(out, item, indent + 1);
                out.push('\n');
            }
        }
        serde_json::Value::Object(map) => {
            out.push('\n');
            format_yaml_object(out, map, indent + 1);
        }
    }
}

fn format_block_children(out: &mut String, children: &[Node], depth: usize) {
    for (i, node) in children.iter().enumerate() {
        if i > 0 && needs_blank_line_before(node, &children[i - 1]) {
            out.push('\n');
        }
        format_node(out, node, depth);
    }
}

fn needs_blank_line_before(node: &Node, prev: &Node) -> bool {
    // Block-level nodes get blank lines between them
    match (prev, node) {
        // No blank line between list items
        (Node::ListItem(_), Node::ListItem(_)) => false,
        (Node::TableRow(_), Node::TableRow(_)) => false,
        // Block elements separated by blank lines
        _ if is_block(prev) && is_block(node) => true,
        _ => false,
    }
}

fn is_block(node: &Node) -> bool {
    matches!(
        node,
        Node::Paragraph(_)
            | Node::Heading(_)
            | Node::List(_)
            | Node::Blockquote(_)
            | Node::CodeBlock(_)
            | Node::ThematicBreak(_)
            | Node::Component(_)
            | Node::MathDisplay(_)
            | Node::Table(_)
            | Node::Html(_)
            | Node::FootnoteDefinition(_)
    )
}

fn format_node(out: &mut String, node: &Node, depth: usize) {
    match node {
        Node::Text(t) => out.push_str(&t.value),
        Node::CodeInline(t) => {
            let delimiter = if t.value.contains('`') { "``" } else { "`" };
            out.push_str(delimiter);
            if t.value.starts_with('`') || t.value.ends_with('`') {
                out.push(' ');
                out.push_str(&t.value);
                out.push(' ');
            } else {
                out.push_str(&t.value);
            }
            out.push_str(delimiter);
        }
        Node::CodeBlock(cb) => {
            let fence = if cb.value.contains("```") {
                "~~~"
            } else {
                "```"
            };
            out.push_str(fence);
            if let Some(ref lang) = cb.lang {
                out.push_str(lang);
                if let Some(ref meta) = cb.meta {
                    out.push(' ');
                    out.push_str(meta);
                }
            }
            out.push('\n');
            out.push_str(&cb.value);
            if !cb.value.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(fence);
            out.push('\n');
        }
        Node::Paragraph(b) => {
            format_inline_children(out, &b.children);
            out.push('\n');
        }
        Node::Heading(b) => {
            let d = b.depth.unwrap_or(1);
            for _ in 0..d {
                out.push('#');
            }
            out.push(' ');
            format_inline_children(out, &b.children);
            if let Some(ref id) = b.id {
                out.push_str(" {#");
                out.push_str(id);
                out.push('}');
            }
            out.push('\n');
        }
        Node::List(b) => {
            let ordered = b.ordered.unwrap_or(false);
            for (i, child) in b.children.iter().enumerate() {
                if let Node::ListItem(li) = child {
                    if ordered {
                        out.push_str(&format!("{}. ", i + 1));
                    } else {
                        out.push_str("- ");
                    }
                    if let Some(checked) = li.checked {
                        out.push_str(if checked { "[x] " } else { "[ ] " });
                    }
                    format_inline_children(out, &li.children);
                    out.push('\n');
                }
            }
        }
        Node::ListItem(_) => {
            // Handled by List
        }
        Node::Blockquote(b) => {
            for child in &b.children {
                let mut child_out = String::new();
                format_node(&mut child_out, child, depth);
                for line in child_out.lines() {
                    out.push_str("> ");
                    out.push_str(line);
                    out.push('\n');
                }
            }
        }
        Node::ThematicBreak(_) => {
            out.push_str("---\n");
        }
        Node::Html(b) => {
            format_inline_children(out, &b.children);
            out.push('\n');
        }
        Node::Table(b) => {
            format_table(out, b);
        }
        Node::TableRow(_) | Node::TableCell(_) => {
            // Handled by Table
        }
        Node::Link(l) => {
            out.push('[');
            format_inline_children(out, &l.children);
            out.push_str("](");
            out.push_str(&l.url);
            if let Some(ref title) = l.title {
                out.push_str(" \"");
                out.push_str(title);
                out.push('"');
            }
            out.push(')');
        }
        Node::Image(img) => {
            out.push_str("![");
            if let Some(ref alt) = img.alt {
                out.push_str(alt);
            } else {
                format_inline_children(out, &img.children);
            }
            out.push_str("](");
            out.push_str(&img.url);
            if let Some(ref title) = img.title {
                out.push_str(" \"");
                out.push_str(title);
                out.push('"');
            }
            out.push(')');
        }
        Node::Emphasis(b) => {
            out.push('*');
            format_inline_children(out, &b.children);
            out.push('*');
        }
        Node::Strong(b) => {
            out.push_str("**");
            format_inline_children(out, &b.children);
            out.push_str("**");
        }
        Node::Strikethrough(b) => {
            out.push_str("~~");
            format_inline_children(out, &b.children);
            out.push_str("~~");
        }
        Node::FootnoteDefinition(f) => {
            out.push_str(&format!("[^{}]: ", f.label));
            format_inline_children(out, &f.children);
            out.push('\n');
        }
        Node::FootnoteReference(f) => {
            out.push_str(&format!("[^{}]", f.label));
        }
        Node::MathInline(t) => {
            out.push('$');
            out.push_str(&t.value);
            out.push('$');
        }
        Node::MathDisplay(t) => {
            out.push_str("$$\n");
            out.push_str(&t.value);
            if !t.value.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("$$\n");
        }
        Node::Component(c) => {
            format_component(out, c, depth);
        }
        Node::Variable(v) => {
            out.push_str("{$");
            out.push_str(&v.path);
            out.push('}');
        }
        Node::Error(e) => {
            // Preserve raw content for error nodes
            out.push_str(&e.raw_content);
        }
    }
}

fn format_inline_children(out: &mut String, children: &[Node]) {
    for child in children {
        format_node(out, child, 0);
    }
}

fn format_component(out: &mut String, c: &ComponentNode, depth: usize) {
    out.push('<');
    out.push_str(&c.name);

    for attr in &c.attributes {
        out.push(' ');
        format_attribute(out, attr);
    }

    if c.children.is_empty() {
        out.push_str(" />\n");
    } else {
        out.push_str(">\n");
        format_block_children(out, &c.children, depth + 1);
        out.push_str(&format!("</{}>\n", c.name));
    }
}

fn format_attribute(out: &mut String, attr: &AttributeNode) {
    out.push_str(&attr.name);
    match &attr.value {
        AttributeValue::Bool(true) => {
            // Boolean shorthand: just the name
        }
        AttributeValue::Bool(false) => {
            out.push_str("={false}");
        }
        AttributeValue::Null => {
            out.push_str("={null}");
        }
        AttributeValue::String(s) => {
            out.push_str("=\"");
            out.push_str(&s.replace('\\', "\\\\").replace('"', "\\\""));
            out.push('"');
        }
        AttributeValue::Number(n) => {
            out.push_str(&format!("={{{}}}", n));
        }
        AttributeValue::Variable(v) => {
            out.push_str("={$");
            out.push_str(&v.path);
            out.push('}');
        }
        AttributeValue::Array(arr) => {
            let json = serde_json::to_string(arr).unwrap_or_default();
            out.push_str("={{");
            out.push_str(&json);
            out.push_str("}}");
        }
        AttributeValue::Object(map) => {
            let val = serde_json::Value::Object(map.clone());
            let json = serde_json::to_string(&val).unwrap_or_default();
            out.push_str("={{");
            out.push_str(&json);
            out.push_str("}}");
        }
    }
}

fn format_table(out: &mut String, table: &StandardBlockNode) {
    // Collect rows
    let rows: Vec<&StandardBlockNode> = table
        .children
        .iter()
        .filter_map(|n| {
            if let Node::TableRow(r) = n {
                Some(r)
            } else {
                None
            }
        })
        .collect();

    if rows.is_empty() {
        return;
    }

    // Calculate column widths
    let col_count = rows.first().map(|r| r.children.len()).unwrap_or(0);
    let mut widths = vec![3usize; col_count];

    let cell_texts: Vec<Vec<String>> = rows
        .iter()
        .map(|row| {
            row.children
                .iter()
                .map(|cell| {
                    let mut s = String::new();
                    if let Node::TableCell(c) = cell {
                        format_inline_children(&mut s, &c.children);
                    }
                    s
                })
                .collect()
        })
        .collect();

    for row_texts in &cell_texts {
        for (j, text) in row_texts.iter().enumerate() {
            if j < widths.len() {
                widths[j] = widths[j].max(text.len());
            }
        }
    }

    // Header row
    if let Some(header) = cell_texts.first() {
        out.push('|');
        for (j, text) in header.iter().enumerate() {
            let w = widths.get(j).copied().unwrap_or(3);
            out.push(' ');
            out.push_str(text);
            for _ in text.len()..w {
                out.push(' ');
            }
            out.push_str(" |");
        }
        out.push('\n');

        // Separator
        out.push('|');
        for &w in &widths {
            out.push(' ');
            for _ in 0..w {
                out.push('-');
            }
            out.push_str(" |");
        }
        out.push('\n');
    }

    // Data rows
    for row_texts in cell_texts.iter().skip(1) {
        out.push('|');
        for (j, text) in row_texts.iter().enumerate() {
            let w = widths.get(j).copied().unwrap_or(3);
            out.push(' ');
            out.push_str(text);
            for _ in text.len()..w {
                out.push(' ');
            }
            out.push_str(" |");
        }
        out.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rdx_parser::parse;

    fn roundtrip(input: &str) -> String {
        let root = parse(input);
        format_root(&root)
    }

    #[test]
    fn format_heading() {
        let out = roundtrip("# Hello World\n");
        assert_eq!(out, "# Hello World\n");
    }

    #[test]
    fn format_paragraph() {
        let out = roundtrip("Hello world.\n");
        assert_eq!(out, "Hello world.\n");
    }

    #[test]
    fn format_emphasis_and_strong() {
        let out = roundtrip("This is **bold** and *italic*.\n");
        assert!(out.contains("**bold**"));
        assert!(out.contains("*italic*"));
    }

    #[test]
    fn format_code_block() {
        let out = roundtrip("```rust\nlet x = 1;\n```\n");
        assert!(out.contains("```rust\n"));
        assert!(out.contains("let x = 1;\n"));
    }

    #[test]
    fn format_component_self_closing() {
        let out = roundtrip("<Badge status=\"beta\" />\n");
        assert!(out.contains("<Badge"));
        assert!(out.contains("status=\"beta\""));
        assert!(out.contains("/>\n"));
    }

    #[test]
    fn format_component_with_children() {
        let out = roundtrip("<Notice type=\"warning\">\nBe careful.\n</Notice>\n");
        assert!(out.contains("<Notice"));
        assert!(out.contains("type=\"warning\""));
        assert!(out.contains("</Notice>"));
    }

    #[test]
    fn format_variable() {
        let out = roundtrip("Hello {$name}!\n");
        assert!(out.contains("{$name}"));
    }

    #[test]
    fn format_frontmatter() {
        let out = roundtrip("---\ntitle: Hello\n---\n\n# Title\n");
        assert!(out.starts_with("---\n"));
        assert!(out.contains("title: Hello\n"));
        assert!(out.contains("---\n"));
    }

    #[test]
    fn format_list() {
        let out = roundtrip("- item 1\n- item 2\n");
        assert!(out.contains("- item 1\n"));
        assert!(out.contains("- item 2\n"));
    }

    #[test]
    fn format_math_inline() {
        let out = roundtrip("The equation $x^2$ is here.\n");
        assert!(out.contains("$x^2$"));
    }

    #[test]
    fn format_math_display() {
        let out = roundtrip("$$\nE = mc^2\n$$\n");
        assert!(out.contains("$$\n"));
        assert!(out.contains("E = mc^2"));
    }

    #[test]
    fn format_link() {
        let out = roundtrip("[click](https://example.com)\n");
        assert!(out.contains("[click](https://example.com)"));
    }

    #[test]
    fn format_blockquote() {
        let out = roundtrip("> quoted text\n");
        assert!(out.contains("> "));
    }

    #[test]
    fn format_thematic_break() {
        let out = roundtrip("Hello\n\n---\n\nWorld\n");
        assert!(out.contains("---\n"));
    }

    #[test]
    fn format_strikethrough() {
        let out = roundtrip("~~deleted~~\n");
        assert!(out.contains("~~deleted~~"));
    }

    #[test]
    fn format_task_list() {
        let out = roundtrip("- [x] done\n- [ ] todo\n");
        assert!(out.contains("[x] done"));
        assert!(out.contains("[ ] todo"));
    }
}
