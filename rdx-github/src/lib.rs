use rdx_ast::*;
use rdx_transform::Transform;

/// Converts GitHub-style references in text to link nodes.
///
/// Supported patterns:
/// - `#123` → link to issue/PR
/// - `@username` → link to user profile
/// - 7+ hex chars → link to commit
///
/// # Configuration
///
/// Set `repo` directly, or add `github: owner/repo` to your document's frontmatter.
///
/// # Example
///
/// ```rust
/// use rdx_transform::Pipeline;
/// use rdx_github::GithubReferences;
///
/// let root = Pipeline::new()
///     .add(GithubReferences::new("rdx-lang/rdx"))
///     .run("Fixed #42 by @octocat.\n");
/// ```
pub struct GithubReferences {
    pub repo: String,
    pub base_url: String,
}

impl Default for GithubReferences {
    fn default() -> Self {
        GithubReferences {
            repo: String::new(),
            base_url: "https://github.com".to_string(),
        }
    }
}

impl GithubReferences {
    pub fn new(repo: &str) -> Self {
        GithubReferences {
            repo: repo.to_string(),
            base_url: "https://github.com".to_string(),
        }
    }

    pub fn with_base_url(mut self, url: &str) -> Self {
        self.base_url = url.to_string();
        self
    }
}

impl Transform for GithubReferences {
    fn name(&self) -> &str {
        "github-references"
    }

    fn transform(&self, root: &mut Root, _source: &str) {
        let repo = if self.repo.is_empty() {
            // Try frontmatter
            root.frontmatter
                .as_ref()
                .and_then(|fm| fm.get("github"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        } else {
            Some(self.repo.clone())
        };

        let Some(repo) = repo else { return };
        let cfg = ResolvedConfig {
            repo,
            base_url: &self.base_url,
        };
        transform_nodes(&mut root.children, &cfg);
    }
}

struct ResolvedConfig<'a> {
    repo: String,
    base_url: &'a str,
}

fn transform_nodes(nodes: &mut Vec<Node>, cfg: &ResolvedConfig) {
    let mut i = 0;
    while i < nodes.len() {
        // First recurse into children of non-text nodes
        match &mut nodes[i] {
            Node::Text(_) => {
                // Handle below
            }
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
            | Node::Strikethrough(b)
            | Node::ThematicBreak(b) => {
                transform_nodes(&mut b.children, cfg);
                i += 1;
                continue;
            }
            // Skip Link/Image children to avoid creating nested links
            Node::Link(_) | Node::Image(_) => {
                i += 1;
                continue;
            }
            Node::Component(c) => {
                transform_nodes(&mut c.children, cfg);
                i += 1;
                continue;
            }
            Node::FootnoteDefinition(f) => {
                transform_nodes(&mut f.children, cfg);
                i += 1;
                continue;
            }
            _ => {
                i += 1;
                continue;
            }
        }

        // Extract text node, try to expand
        let Node::Text(ref text_node) = nodes[i] else {
            i += 1;
            continue;
        };
        let refs = find_references(&text_node.value);
        if refs.is_empty() {
            i += 1;
            continue;
        }

        let old = nodes.remove(i);
        let text_node = match old {
            Node::Text(t) => t,
            _ => unreachable!(),
        };
        let expanded = expand_text(text_node, cfg);
        let count = expanded.len();
        for (j, node) in expanded.into_iter().enumerate() {
            nodes.insert(i + j, node);
        }
        i += count;
    }
}

struct Reference {
    kind: RefKind,
    start: usize,
    end: usize,
    value: String,
}

enum RefKind {
    Issue,
    User,
    Commit,
}

fn find_references(text: &str) -> Vec<Reference> {
    let mut refs = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'#' {
            let start = i;
            i += 1;
            let num_start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i > num_start && (start == 0 || !bytes[start - 1].is_ascii_alphanumeric()) {
                refs.push(Reference {
                    kind: RefKind::Issue,
                    start,
                    end: i,
                    value: text[num_start..i].to_string(),
                });
                continue;
            }
        } else if bytes[i] == b'@' {
            let start = i;
            i += 1;
            let name_start = i;
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_')
            {
                i += 1;
            }
            if i > name_start && (start == 0 || bytes[start - 1].is_ascii_whitespace()) {
                refs.push(Reference {
                    kind: RefKind::User,
                    start,
                    end: i,
                    value: text[name_start..i].to_string(),
                });
                continue;
            }
        } else if bytes[i].is_ascii_hexdigit() && (i == 0 || !bytes[i - 1].is_ascii_alphanumeric())
        {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_hexdigit() {
                i += 1;
            }
            let len = i - start;
            if (7..=40).contains(&len) && (i >= bytes.len() || !bytes[i].is_ascii_alphanumeric()) {
                let has_letter = text[start..i].bytes().any(|b| b.is_ascii_alphabetic());
                if has_letter {
                    refs.push(Reference {
                        kind: RefKind::Commit,
                        start,
                        end: i,
                        value: text[start..i].to_string(),
                    });
                    continue;
                }
            }
        }
        i += 1;
    }
    refs
}

/// Build a position for a sub-span of a text node, offsetting from the
/// original start position. Columns are approximate (assumes single-line text).
fn sub_position(base: &Position, byte_start: usize, byte_end: usize) -> Position {
    Position {
        start: Point {
            line: base.start.line,
            column: base.start.column + byte_start,
            offset: base.start.offset + byte_start,
        },
        end: Point {
            line: base.start.line,
            column: base.start.column + byte_end,
            offset: base.start.offset + byte_end,
        },
    }
}

fn expand_text(text_node: TextNode, cfg: &ResolvedConfig) -> Vec<Node> {
    let refs = find_references(&text_node.value);
    if refs.is_empty() {
        return vec![Node::Text(text_node)];
    }

    let mut result = Vec::new();
    let mut last_end = 0;
    let base = &text_node.position;

    for r in &refs {
        if r.start > last_end {
            result.push(Node::Text(TextNode {
                value: text_node.value[last_end..r.start].to_string(),
                position: sub_position(base, last_end, r.start),
            }));
        }

        let (url, display) = match r.kind {
            RefKind::Issue => (
                format!("{}/{}/issues/{}", cfg.base_url, cfg.repo, r.value),
                format!("#{}", r.value),
            ),
            RefKind::User => (
                format!("{}/{}", cfg.base_url, r.value),
                format!("@{}", r.value),
            ),
            RefKind::Commit => (
                format!("{}/{}/commit/{}", cfg.base_url, cfg.repo, r.value),
                r.value[..7.min(r.value.len())].to_string(),
            ),
        };

        let ref_pos = sub_position(base, r.start, r.end);
        result.push(Node::Link(LinkNode {
            url,
            title: None,
            children: vec![Node::Text(TextNode {
                value: display,
                position: ref_pos.clone(),
            })],
            position: ref_pos,
        }));

        last_end = r.end;
    }

    if last_end < text_node.value.len() {
        result.push(Node::Text(TextNode {
            value: text_node.value[last_end..].to_string(),
            position: sub_position(base, last_end, text_node.value.len()),
        }));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use rdx_transform::Pipeline;

    #[test]
    fn issue_reference() {
        let root = Pipeline::new()
            .add(GithubReferences::new("rdx-lang/rdx"))
            .run("See #42 for details.\n");

        match &root.children[0] {
            Node::Paragraph(p) => {
                let has_link = p
                    .children
                    .iter()
                    .any(|n| matches!(n, Node::Link(l) if l.url.contains("/issues/42")));
                assert!(has_link, "Should have issue link: {:?}", p.children);
            }
            other => panic!("Expected paragraph, got {:?}", other),
        }
    }

    #[test]
    fn user_reference() {
        let root = Pipeline::new()
            .add(GithubReferences::new("rdx-lang/rdx"))
            .run("Thanks @octocat for the fix.\n");

        match &root.children[0] {
            Node::Paragraph(p) => {
                let has_link = p
                    .children
                    .iter()
                    .any(|n| matches!(n, Node::Link(l) if l.url.contains("/octocat")));
                assert!(has_link, "Should have user link: {:?}", p.children);
            }
            other => panic!("Expected paragraph, got {:?}", other),
        }
    }

    #[test]
    fn commit_reference() {
        let root = Pipeline::new()
            .add(GithubReferences::new("rdx-lang/rdx"))
            .run("Fixed in abc1234def.\n");

        match &root.children[0] {
            Node::Paragraph(p) => {
                let has_link = p
                    .children
                    .iter()
                    .any(|n| matches!(n, Node::Link(l) if l.url.contains("/commit/")));
                assert!(has_link, "Should have commit link: {:?}", p.children);
            }
            other => panic!("Expected paragraph, got {:?}", other),
        }
    }

    #[test]
    fn no_transform_without_repo() {
        let root = Pipeline::new()
            .add(GithubReferences::default())
            .run("See #42.\n");

        match &root.children[0] {
            Node::Paragraph(p) => {
                let has_link = p.children.iter().any(|n| matches!(n, Node::Link(_)));
                assert!(!has_link, "Should not transform without repo");
            }
            other => panic!("Expected paragraph, got {:?}", other),
        }
    }

    #[test]
    fn no_nested_links() {
        // A markdown link whose text contains #123 should NOT become link-inside-link
        let root = Pipeline::new()
            .add(GithubReferences::new("rdx-lang/rdx"))
            .run("See [issue #123](https://example.com) for details.\n");

        match &root.children[0] {
            Node::Paragraph(p) => {
                for node in &p.children {
                    if let Node::Link(l) = node {
                        // No child of a link should itself be a link
                        let has_nested = l.children.iter().any(|c| matches!(c, Node::Link(_)));
                        assert!(
                            !has_nested,
                            "Should not create nested links: {:?}",
                            l.children
                        );
                    }
                }
            }
            other => panic!("Expected paragraph, got {:?}", other),
        }
    }

    #[test]
    fn repo_from_frontmatter() {
        let root = Pipeline::new()
            .add(GithubReferences::default())
            .run("---\ngithub: rdx-lang/rdx\n---\nSee #42.\n");

        let has_link = root.children.iter().any(|n| {
            if let Node::Paragraph(p) = n {
                p.children
                    .iter()
                    .any(|c| matches!(c, Node::Link(l) if l.url.contains("/issues/42")))
            } else {
                false
            }
        });
        assert!(
            has_link,
            "Should transform with repo from frontmatter: {:?}",
            root.children
        );
    }
}
