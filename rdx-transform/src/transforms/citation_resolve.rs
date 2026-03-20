use std::collections::HashMap;
use std::fs;

use rdx_ast::*;

use crate::{synthetic_pos, Transform};

/// Resolve citation keys against a bibliography and inject formatted text.
///
/// Given a `.bib` file path (from frontmatter `bibliography` field), this
/// transform:
/// 1. Parses BibTeX entries to extract keys, authors, titles, years.
/// 2. Replaces each `Node::Citation` with formatted inline text
///    (e.g., "(Smith, 2024)" or "[1]" depending on `style`).
/// 3. Appends a formatted bibliography section to the document
///    at a `<Bibliography />` placeholder, or at the end if none exists.
pub struct CitationResolve {
    /// Parsed bibliography entries, keyed by citation key.
    pub entries: HashMap<String, BibEntry>,
    /// Citation style: "author-year" or "numeric".
    pub style: CitationStyle,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CitationStyle {
    /// (Smith, 2024) / (Smith & Jones, 2024) / (Smith et al., 2024)
    AuthorYear,
    /// [1], [2], [3]
    Numeric,
}

/// A parsed bibliography entry with just the fields we need for formatting.
#[derive(Debug, Clone)]
pub struct BibEntry {
    pub key: String,
    pub entry_type: String,
    pub authors: Vec<String>,
    pub title: String,
    pub year: String,
    pub journal: Option<String>,
    pub publisher: Option<String>,
    pub volume: Option<String>,
    pub pages: Option<String>,
    pub url: Option<String>,
    pub doi: Option<String>,
}

impl CitationResolve {
    /// Create from a `.bib` file path. Returns `Err` if the file can't be read or parsed.
    pub fn from_bib_file(path: &str, style: CitationStyle) -> Result<Self, String> {
        let content =
            fs::read_to_string(path).map_err(|e| format!("cannot read {}: {}", path, e))?;
        let entries = parse_bib(&content);
        Ok(CitationResolve { entries, style })
    }

    /// Create directly from pre-parsed entries.
    pub fn new(entries: HashMap<String, BibEntry>, style: CitationStyle) -> Self {
        CitationResolve { entries, style }
    }

    fn format_inline(&self, keys: &[CitationKey], key_order: &[String]) -> String {
        match self.style {
            CitationStyle::AuthorYear => {
                let parts: Vec<String> = keys
                    .iter()
                    .map(|k| {
                        if let Some(entry) = self.entries.get(&k.id) {
                            let author = if entry.authors.is_empty() {
                                k.id.clone()
                            } else if entry.authors.len() == 1 {
                                surname(&entry.authors[0])
                            } else if entry.authors.len() == 2 {
                                format!(
                                    "{} & {}",
                                    surname(&entry.authors[0]),
                                    surname(&entry.authors[1])
                                )
                            } else {
                                format!("{} et al.", surname(&entry.authors[0]))
                            };
                            let mut s = format!("{}, {}", author, entry.year);
                            if let Some(ref loc) = k.locator {
                                s.push_str(&format!(", {}", loc));
                            }
                            s
                        } else {
                            format!("{}?", k.id)
                        }
                    })
                    .collect();
                format!("({})", parts.join("; "))
            }
            CitationStyle::Numeric => {
                let nums: Vec<String> = keys
                    .iter()
                    .map(|k| {
                        let idx = key_order.iter().position(|x| x == &k.id);
                        match idx {
                            Some(i) => format!("{}", i + 1),
                            None => "?".to_string(),
                        }
                    })
                    .collect();
                format!("[{}]", nums.join(", "))
            }
        }
    }

    fn format_bib_entry(&self, entry: &BibEntry, number: Option<usize>) -> String {
        let mut s = String::new();
        if let Some(n) = number {
            s.push_str(&format!("[{}] ", n));
        }
        if !entry.authors.is_empty() {
            s.push_str(&entry.authors.join(", "));
            s.push_str(". ");
        }
        if !entry.title.is_empty() {
            s.push_str(&format!("\"{}\"", entry.title));
            s.push_str(". ");
        }
        if let Some(ref j) = entry.journal {
            s.push_str(&format!("*{}*", j));
            if let Some(ref v) = entry.volume {
                s.push_str(&format!(", {}", v));
            }
            if let Some(ref p) = entry.pages {
                s.push_str(&format!(", pp. {}", p));
            }
            s.push_str(". ");
        } else if let Some(ref p) = entry.publisher {
            s.push_str(&format!("{}, ", p));
        }
        s.push_str(&format!("{}.", entry.year));
        if let Some(ref doi) = entry.doi {
            s.push_str(&format!(" doi:{}", doi));
        }
        s
    }
}

impl Transform for CitationResolve {
    fn name(&self) -> &str {
        "citation-resolve"
    }

    fn transform(&self, root: &mut Root, _source: &str) {
        // First pass: collect citation key order (for numeric style)
        let mut key_order: Vec<String> = Vec::new();
        collect_citation_keys(&root.children, &mut key_order);

        // Second pass: replace Citation nodes with formatted text
        resolve_citations(&mut root.children, self, &key_order);

        // Third pass: build bibliography section and inject it
        if key_order.is_empty() {
            return;
        }
        let bib_node = self.build_bibliography(&key_order);

        // Look for <Bibliography /> placeholder
        let placeholder = root.children.iter().position(|n| {
            matches!(n, Node::Component(c) if c.name == "Bibliography" && c.children.is_empty())
        });
        if let Some(idx) = placeholder {
            root.children[idx] = bib_node;
        } else {
            root.children.push(bib_node);
        }
    }
}

impl CitationResolve {
    fn build_bibliography(&self, key_order: &[String]) -> Node {
        let pos = synthetic_pos();
        let mut items = Vec::new();

        for (i, key) in key_order.iter().enumerate() {
            if let Some(entry) = self.entries.get(key) {
                let number = match self.style {
                    CitationStyle::Numeric => Some(i + 1),
                    CitationStyle::AuthorYear => None,
                };
                let text = self.format_bib_entry(entry, number);
                items.push(Node::ListItem(StandardBlockNode {
                    depth: None,
                    ordered: Some(true),
                    checked: None,
                    id: Some(format!("bib:{}", key)),
                    children: vec![Node::Paragraph(StandardBlockNode {
                        depth: None,
                        ordered: None,
                        checked: None,
                        id: None,
                        children: vec![Node::Text(TextNode {
                            value: text,
                            position: pos.clone(),
                        })],
                        position: pos.clone(),
                    })],
                    position: pos.clone(),
                }));
            }
        }

        Node::Component(ComponentNode {
            name: "Bibliography".to_string(),
            is_inline: false,
            attributes: vec![],
            children: vec![
                Node::Heading(StandardBlockNode {
                    depth: Some(2),
                    ordered: None,
                    checked: None,
                    id: Some("references".to_string()),
                    children: vec![Node::Text(TextNode {
                        value: "References".to_string(),
                        position: pos.clone(),
                    })],
                    position: pos.clone(),
                }),
                Node::List(StandardBlockNode {
                    depth: None,
                    ordered: Some(true),
                    checked: None,
                    id: None,
                    children: items,
                    position: pos.clone(),
                }),
            ],
            raw_content: String::new(),
            position: pos,
        })
    }
}

fn collect_citation_keys(nodes: &[Node], order: &mut Vec<String>) {
    for node in nodes {
        if let Node::Citation(c) = node {
            for key in &c.keys {
                if !order.contains(&key.id) {
                    order.push(key.id.clone());
                }
            }
        }
        if let Some(children) = node.children() {
            collect_citation_keys(children, order);
        }
    }
}

fn resolve_citations(nodes: &mut Vec<Node>, resolver: &CitationResolve, key_order: &[String]) {
    for node in nodes.iter_mut() {
        if let Node::Citation(c) = node {
            let text = resolver.format_inline(&c.keys, key_order);
            *node = Node::Text(TextNode {
                value: text,
                position: c.position.clone(),
            });
            continue;
        }
        if let Some(children) = node.children_mut() {
            resolve_citations(children, resolver, key_order);
        }
    }
}

/// Extract the surname from a "First Last" or "Last, First" name.
fn surname(name: &str) -> String {
    if let Some(comma_pos) = name.find(',') {
        name[..comma_pos].trim().to_string()
    } else {
        name.rsplit_once(' ')
            .map(|(_, last)| last.to_string())
            .unwrap_or_else(|| name.to_string())
    }
}

// ── BibTeX parser ────────────────────────────────────────────────────────────

/// Parse a BibTeX string into a map of key → BibEntry.
pub fn parse_bib(input: &str) -> HashMap<String, BibEntry> {
    let mut entries = HashMap::new();
    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        if ch == '%' {
            // Skip comment line
            while chars.peek().is_some_and(|&c| c != '\n') {
                chars.next();
            }
            continue;
        }
        if ch == '@' {
            chars.next(); // consume @
            let mut entry_type = String::new();
            while chars.peek().is_some_and(|c| c.is_ascii_alphanumeric()) {
                entry_type.push(chars.next().unwrap());
            }
            let entry_type_lower = entry_type.to_ascii_lowercase();

            // Skip non-entry types
            if matches!(
                entry_type_lower.as_str(),
                "string" | "preamble" | "comment"
            ) {
                skip_braced_block(&mut chars);
                continue;
            }

            // Skip whitespace and opening brace/paren
            skip_ws(&mut chars);
            let opener = chars.next();
            if opener != Some('{') && opener != Some('(') {
                continue;
            }

            // Read citation key
            let mut key = String::new();
            while chars
                .peek()
                .is_some_and(|c| *c != ',' && *c != '}' && *c != ')')
            {
                key.push(chars.next().unwrap());
            }
            // Consume the comma after key
            if chars.peek() == Some(&',') {
                chars.next();
            }
            let key = key.trim().to_string();
            if key.is_empty() {
                skip_braced_block_remaining(&mut chars);
                continue;
            }

            // Parse fields
            let mut fields: HashMap<String, String> = HashMap::new();
            loop {
                skip_ws(&mut chars);
                match chars.peek() {
                    None | Some('}') | Some(')') => {
                        chars.next();
                        break;
                    }
                    _ => {}
                }

                // Field name
                let mut field_name = String::new();
                while chars
                    .peek()
                    .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                {
                    field_name.push(chars.next().unwrap());
                }
                let field_name = field_name.trim().to_ascii_lowercase();
                skip_ws(&mut chars);
                if chars.peek() == Some(&'=') {
                    chars.next();
                }
                skip_ws(&mut chars);
                let value = read_bib_value(&mut chars);
                if !field_name.is_empty() {
                    fields.insert(field_name, value);
                }

                // Skip comma between fields
                skip_ws(&mut chars);
                if chars.peek() == Some(&',') {
                    chars.next();
                }
            }

            let authors = fields
                .get("author")
                .map(|a| parse_bib_authors(a))
                .unwrap_or_default();
            let title = fields.get("title").cloned().unwrap_or_default();
            let year = fields.get("year").cloned().unwrap_or_default();

            entries.insert(
                key.clone(),
                BibEntry {
                    key,
                    entry_type: entry_type_lower,
                    authors,
                    title,
                    year,
                    journal: fields.get("journal").cloned(),
                    publisher: fields.get("publisher").cloned(),
                    volume: fields.get("volume").cloned(),
                    pages: fields.get("pages").cloned(),
                    url: fields.get("url").cloned(),
                    doi: fields.get("doi").cloned(),
                },
            );
        } else {
            chars.next();
        }
    }
    entries
}

fn skip_ws(chars: &mut std::iter::Peekable<std::str::Chars>) {
    while chars.peek().is_some_and(|c| c.is_ascii_whitespace()) {
        chars.next();
    }
}

fn skip_braced_block(chars: &mut std::iter::Peekable<std::str::Chars>) {
    skip_ws(chars);
    if chars.peek() == Some(&'{') || chars.peek() == Some(&'(') {
        chars.next();
        skip_braced_block_remaining(chars);
    }
}

fn skip_braced_block_remaining(chars: &mut std::iter::Peekable<std::str::Chars>) {
    let mut depth = 1;
    for ch in chars.by_ref() {
        match ch {
            '{' | '(' => depth += 1,
            '}' | ')' => {
                depth -= 1;
                if depth == 0 {
                    return;
                }
            }
            _ => {}
        }
    }
}

fn read_bib_value(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    skip_ws(chars);
    match chars.peek() {
        Some(&'{') => {
            chars.next();
            let mut val = String::new();
            let mut depth = 1;
            for ch in chars.by_ref() {
                match ch {
                    '{' => {
                        depth += 1;
                        val.push(ch);
                    }
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                        val.push(ch);
                    }
                    _ => val.push(ch),
                }
            }
            val
        }
        Some(&'"') => {
            chars.next();
            let mut val = String::new();
            for ch in chars.by_ref() {
                if ch == '"' {
                    break;
                }
                val.push(ch);
            }
            val
        }
        _ => {
            // Bare number or string reference
            let mut val = String::new();
            while let Some(&ch) = chars.peek() {
                if ch == ',' || ch == '}' || ch == ')' || ch.is_ascii_whitespace() {
                    break;
                }
                val.push(ch);
                chars.next();
            }
            val
        }
    }
}

fn parse_bib_authors(raw: &str) -> Vec<String> {
    raw.split(" and ")
        .map(|a| a.trim().to_string())
        .filter(|a| !a.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_citation(keys: &[(&str, Option<&str>)]) -> Node {
        Node::Citation(CitationNode {
            keys: keys
                .iter()
                .map(|(id, loc)| CitationKey {
                    id: id.to_string(),
                    prefix: None,
                    locator: loc.map(|s| s.to_string()),
                })
                .collect(),
            position: synthetic_pos(),
        })
    }

    fn make_root_with(nodes: Vec<Node>) -> Root {
        Root {
            node_type: RootType::Root,
            frontmatter: None,
            children: nodes,
            position: synthetic_pos(),
        }
    }

    #[test]
    fn parse_bib_basic() {
        let bib = r#"
@article{smith2024,
  author = {John Smith and Jane Doe},
  title = {A Great Paper},
  journal = {Nature},
  year = {2024},
  volume = {42},
  pages = {100--110},
}
        "#;
        let entries = parse_bib(bib);
        assert_eq!(entries.len(), 1);
        let e = &entries["smith2024"];
        assert_eq!(e.authors, vec!["John Smith", "Jane Doe"]);
        assert_eq!(e.title, "A Great Paper");
        assert_eq!(e.year, "2024");
        assert_eq!(e.journal.as_deref(), Some("Nature"));
        assert_eq!(e.volume.as_deref(), Some("42"));
    }

    #[test]
    fn parse_bib_skips_comments_and_preamble() {
        let bib = r#"
% This is a comment
@preamble{"\newcommand{\noopsort}[1]{}"}
@string{mit = {MIT Press}}
@book{knuth1984,
  author = {Donald Knuth},
  title = {The TeXbook},
  year = {1984},
  publisher = {Addison-Wesley},
}
        "#;
        let entries = parse_bib(bib);
        assert_eq!(entries.len(), 1);
        assert!(entries.contains_key("knuth1984"));
    }

    #[test]
    fn parse_bib_double_quoted_values() {
        let bib = r#"
@inproceedings{jones2023,
  author = "Alice Jones",
  title = "Machine Learning",
  year = "2023",
}
        "#;
        let entries = parse_bib(bib);
        assert_eq!(entries["jones2023"].authors, vec!["Alice Jones"]);
        assert_eq!(entries["jones2023"].title, "Machine Learning");
    }

    #[test]
    fn author_year_single_author() {
        let mut entries = HashMap::new();
        entries.insert(
            "smith2024".to_string(),
            BibEntry {
                key: "smith2024".to_string(),
                entry_type: "article".to_string(),
                authors: vec!["John Smith".to_string()],
                title: "A Paper".to_string(),
                year: "2024".to_string(),
                journal: None,
                publisher: None,
                volume: None,
                pages: None,
                url: None,
                doi: None,
            },
        );
        let resolver = CitationResolve::new(entries, CitationStyle::AuthorYear);

        let keys = vec![CitationKey {
            id: "smith2024".to_string(),
            prefix: None,
            locator: None,
        }];
        let text = resolver.format_inline(&keys, &["smith2024".to_string()]);
        assert_eq!(text, "(Smith, 2024)");
    }

    #[test]
    fn author_year_two_authors() {
        let mut entries = HashMap::new();
        entries.insert(
            "k1".to_string(),
            BibEntry {
                key: "k1".to_string(),
                entry_type: "article".to_string(),
                authors: vec!["Alice Jones".to_string(), "Bob Smith".to_string()],
                title: String::new(),
                year: "2023".to_string(),
                journal: None,
                publisher: None,
                volume: None,
                pages: None,
                url: None,
                doi: None,
            },
        );
        let resolver = CitationResolve::new(entries, CitationStyle::AuthorYear);
        let keys = vec![CitationKey {
            id: "k1".to_string(),
            prefix: None,
            locator: None,
        }];
        assert_eq!(
            resolver.format_inline(&keys, &["k1".to_string()]),
            "(Jones & Smith, 2023)"
        );
    }

    #[test]
    fn author_year_three_plus_authors() {
        let mut entries = HashMap::new();
        entries.insert(
            "k1".to_string(),
            BibEntry {
                key: "k1".to_string(),
                entry_type: "article".to_string(),
                authors: vec!["A".to_string(), "B".to_string(), "C".to_string()],
                title: String::new(),
                year: "2020".to_string(),
                journal: None,
                publisher: None,
                volume: None,
                pages: None,
                url: None,
                doi: None,
            },
        );
        let resolver = CitationResolve::new(entries, CitationStyle::AuthorYear);
        let keys = vec![CitationKey {
            id: "k1".to_string(),
            prefix: None,
            locator: None,
        }];
        assert_eq!(
            resolver.format_inline(&keys, &["k1".to_string()]),
            "(A et al., 2020)"
        );
    }

    #[test]
    fn author_year_with_locator() {
        let mut entries = HashMap::new();
        entries.insert(
            "s".to_string(),
            BibEntry {
                key: "s".to_string(),
                entry_type: "book".to_string(),
                authors: vec!["Smith".to_string()],
                title: String::new(),
                year: "2024".to_string(),
                journal: None,
                publisher: None,
                volume: None,
                pages: None,
                url: None,
                doi: None,
            },
        );
        let resolver = CitationResolve::new(entries, CitationStyle::AuthorYear);
        let keys = vec![CitationKey {
            id: "s".to_string(),
            prefix: None,
            locator: Some("p. 42".to_string()),
        }];
        assert_eq!(
            resolver.format_inline(&keys, &["s".to_string()]),
            "(Smith, 2024, p. 42)"
        );
    }

    #[test]
    fn numeric_style() {
        let entries = HashMap::new(); // entries not needed for numeric — just order
        let resolver = CitationResolve::new(entries, CitationStyle::Numeric);
        let keys = vec![
            CitationKey {
                id: "a".to_string(),
                prefix: None,
                locator: None,
            },
            CitationKey {
                id: "c".to_string(),
                prefix: None,
                locator: None,
            },
        ];
        let order = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(resolver.format_inline(&keys, &order), "[1, 3]");
    }

    #[test]
    fn unknown_key_shows_question_mark() {
        let entries = HashMap::new();
        let resolver = CitationResolve::new(entries, CitationStyle::AuthorYear);
        let keys = vec![CitationKey {
            id: "missing".to_string(),
            prefix: None,
            locator: None,
        }];
        assert_eq!(
            resolver.format_inline(&keys, &["missing".to_string()]),
            "(missing?)"
        );
    }

    #[test]
    fn transform_replaces_citations_and_appends_bib() {
        let mut entries = HashMap::new();
        entries.insert(
            "smith2024".to_string(),
            BibEntry {
                key: "smith2024".to_string(),
                entry_type: "article".to_string(),
                authors: vec!["John Smith".to_string()],
                title: "Paper".to_string(),
                year: "2024".to_string(),
                journal: Some("Nature".to_string()),
                publisher: None,
                volume: None,
                pages: None,
                url: None,
                doi: None,
            },
        );
        let resolver = CitationResolve::new(entries, CitationStyle::AuthorYear);

        let mut root = make_root_with(vec![Node::Paragraph(StandardBlockNode {
            depth: None,
            ordered: None,
            checked: None,
            id: None,
            children: vec![
                Node::Text(TextNode {
                    value: "See ".to_string(),
                    position: synthetic_pos(),
                }),
                make_citation(&[("smith2024", None)]),
            ],
            position: synthetic_pos(),
        })]);

        resolver.transform(&mut root, "");

        // Citation should be replaced with text
        if let Node::Paragraph(p) = &root.children[0] {
            assert_eq!(p.children.len(), 2);
            if let Node::Text(t) = &p.children[1] {
                assert_eq!(t.value, "(Smith, 2024)");
            } else {
                panic!("Expected text node, got {:?}", p.children[1]);
            }
        }

        // Bibliography should be appended at end
        let last = root.children.last().unwrap();
        assert!(
            matches!(last, Node::Component(c) if c.name == "Bibliography"),
            "Expected Bibliography component at end, got {:?}",
            last
        );
    }

    #[test]
    fn transform_replaces_placeholder() {
        let entries = HashMap::new();
        let resolver = CitationResolve::new(entries, CitationStyle::Numeric);

        let mut root = make_root_with(vec![
            Node::Paragraph(StandardBlockNode {
                depth: None,
                ordered: None,
                checked: None,
                id: None,
                children: vec![make_citation(&[("k1", None)])],
                position: synthetic_pos(),
            }),
            Node::Component(ComponentNode {
                name: "Bibliography".to_string(),
                is_inline: false,
                attributes: vec![],
                children: vec![], // placeholder — empty
                raw_content: String::new(),
                position: synthetic_pos(),
            }),
            Node::Paragraph(StandardBlockNode {
                depth: None,
                ordered: None,
                checked: None,
                id: None,
                children: vec![Node::Text(TextNode {
                    value: "After bib".to_string(),
                    position: synthetic_pos(),
                })],
                position: synthetic_pos(),
            }),
        ]);

        resolver.transform(&mut root, "");

        // Bibliography should replace placeholder (index 1), not be appended
        assert_eq!(root.children.len(), 3);
        assert!(
            matches!(&root.children[1], Node::Component(c) if c.name == "Bibliography" && !c.children.is_empty()),
        );
        // "After bib" should still be at index 2
        assert!(matches!(&root.children[2], Node::Paragraph(_)));
    }

    #[test]
    fn no_citations_no_bibliography() {
        let entries = HashMap::new();
        let resolver = CitationResolve::new(entries, CitationStyle::AuthorYear);
        let mut root = make_root_with(vec![Node::Text(TextNode {
            value: "No citations here".to_string(),
            position: synthetic_pos(),
        })]);
        resolver.transform(&mut root, "");
        // No bibliography should be added
        assert_eq!(root.children.len(), 1);
    }

    #[test]
    fn surname_extraction() {
        assert_eq!(surname("John Smith"), "Smith");
        assert_eq!(surname("Smith, John"), "Smith");
        assert_eq!(surname("Madonna"), "Madonna");
        assert_eq!(surname("van der Berg, Jan"), "van der Berg");
    }

    #[test]
    fn bib_entry_formatting() {
        let entry = BibEntry {
            key: "smith2024".to_string(),
            entry_type: "article".to_string(),
            authors: vec!["John Smith".to_string(), "Jane Doe".to_string()],
            title: "A Great Paper".to_string(),
            year: "2024".to_string(),
            journal: Some("Nature".to_string()),
            publisher: None,
            volume: Some("42".to_string()),
            pages: Some("100--110".to_string()),
            url: None,
            doi: Some("10.1234/test".to_string()),
        };
        let resolver = CitationResolve::new(HashMap::new(), CitationStyle::Numeric);
        let formatted = resolver.format_bib_entry(&entry, Some(1));
        assert!(formatted.contains("[1]"));
        assert!(formatted.contains("John Smith, Jane Doe"));
        assert!(formatted.contains("\"A Great Paper\""));
        assert!(formatted.contains("*Nature*"));
        assert!(formatted.contains("42"));
        assert!(formatted.contains("100--110"));
        assert!(formatted.contains("doi:10.1234/test"));
    }
}
