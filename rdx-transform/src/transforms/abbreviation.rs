use rdx_ast::*;

use crate::{Transform, synthetic_pos};

// ---------------------------------------------------------------------------
// Transform
// ---------------------------------------------------------------------------

/// Reads `abbreviations` from the document frontmatter and wraps the **first**
/// occurrence of each abbreviation in the document text with an `<Abbr>`
/// inline component that carries a `title` attribute containing the expansion.
///
/// # Frontmatter format
///
/// ```yaml
/// ---
/// abbreviations:
///   HTML: HyperText Markup Language
///   CSS: Cascading Style Sheets
/// ---
/// ```
///
/// The above will wrap the first occurrence of the literal string `"HTML"` with
/// `<Abbr title="HyperText Markup Language">HTML</Abbr>` and similarly for
/// `"CSS"`.
///
/// Subsequent occurrences of the same abbreviation are left as plain text.
///
/// # Notes
///
/// - Only [`Node::Text`] nodes inside the document body are searched.
/// - If the frontmatter is absent, or has no `abbreviations` map, the
///   transform is a no-op.
/// - Abbreviation matching is **case-sensitive** and is done on exact
///   sub-string boundaries (the abbreviation must appear as-is in the text).
pub struct AbbreviationExpand;

impl Transform for AbbreviationExpand {
    fn name(&self) -> &str {
        "abbreviation-expand"
    }

    fn transform(&self, root: &mut Root, _source: &str) {
        // Extract the abbreviations map from frontmatter.
        let abbreviations = match root.frontmatter.as_ref() {
            Some(fm) => match fm.get("abbreviations") {
                Some(serde_json::Value::Object(map)) => map
                    .iter()
                    .filter_map(|(k, v)| {
                        if let serde_json::Value::String(expansion) = v {
                            Some((k.clone(), expansion.clone()))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>(),
                _ => return,
            },
            None => return,
        };

        if abbreviations.is_empty() {
            return;
        }

        // Track which abbreviations have already been wrapped (first-occurrence only).
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

        expand_nodes(&mut root.children, &abbreviations, &mut seen);
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Build an `<Abbr title="...">text</Abbr>` inline component node.
fn make_abbr_component(abbr: &str, expansion: &str) -> Node {
    Node::Component(ComponentNode {
        name: "Abbr".to_string(),
        is_inline: true,
        attributes: vec![AttributeNode {
            name: "title".to_string(),
            value: AttributeValue::String(expansion.to_string()),
            position: synthetic_pos(),
        }],
        children: vec![Node::Text(TextNode {
            value: abbr.to_string(),
            position: synthetic_pos(),
        })],
        raw_content: String::new(),
        position: synthetic_pos(),
    })
}

/// Try to expand the first unseen abbreviation found in `text`.
///
/// Returns `None` if no abbreviation from the list (that hasn't been seen yet)
/// appears in `text`.  Returns `Some(Vec<Node>)` of replacement nodes
/// otherwise — the split text fragments interspersed with the `<Abbr>`
/// component for the matched abbreviation.
///
/// We look for the *longest* matching abbreviation at the *earliest* position
/// to produce predictable output.
fn split_on_first_abbr(
    text: &str,
    abbreviations: &[(String, String)],
    seen: &std::collections::HashSet<String>,
) -> Option<(String, Vec<Node>)> {
    // Find the earliest (leftmost) occurrence of any unseen abbreviation.
    // If two abbreviations start at the same position, prefer the longer one.
    let mut best: Option<(usize, usize, &str, &str)> = None; // (start, end, abbr, expansion)

    for (abbr, expansion) in abbreviations {
        if seen.contains(abbr.as_str()) {
            continue;
        }
        if let Some(pos) = text.find(abbr.as_str()) {
            let end = pos + abbr.len();
            let is_better = match best {
                None => true,
                Some((best_start, best_end, _, _)) => {
                    pos < best_start || (pos == best_start && end > best_end)
                }
            };
            if is_better {
                best = Some((pos, end, abbr.as_str(), expansion.as_str()));
            }
        }
    }

    let (start, end, abbr, expansion) = best?;

    let mut nodes: Vec<Node> = Vec::new();
    if start > 0 {
        nodes.push(Node::Text(TextNode {
            value: text[..start].to_string(),
            position: synthetic_pos(),
        }));
    }
    nodes.push(make_abbr_component(abbr, expansion));
    if end < text.len() {
        nodes.push(Node::Text(TextNode {
            value: text[end..].to_string(),
            position: synthetic_pos(),
        }));
    }

    Some((abbr.to_string(), nodes))
}

/// Recursively process a children vector, expanding text nodes in place.
///
/// When a text node is expanded into multiple nodes, we replace that single
/// slot with the new nodes.  We then continue scanning the rest of the
/// children (the newly inserted `Text` suffix node may contain further
/// abbreviations).
fn expand_nodes(
    nodes: &mut Vec<Node>,
    abbreviations: &[(String, String)],
    seen: &mut std::collections::HashSet<String>,
) {
    let mut i = 0;
    while i < nodes.len() {
        let expansion_result = if let Node::Text(ref t) = nodes[i] {
            split_on_first_abbr(&t.value, abbreviations, seen)
        } else {
            None
        };

        if let Some((matched_abbr, replacement_nodes)) = expansion_result {
            seen.insert(matched_abbr);
            let how_many = replacement_nodes.len();
            // Replace the single text node with the replacement sequence.
            nodes.splice(i..=i, replacement_nodes);
            // Advance past all newly inserted nodes so we don't re-process them,
            // except for the trailing text node (if any) which may contain more
            // abbreviations.  We advance `i` to just before the last inserted
            // node and let the loop increment handle the rest.
            if how_many > 0 {
                i += how_many - 1;
            }
        } else {
            // Recurse into children of non-text or already-matched-text nodes.
            if let Some(children) = nodes[i].children_mut() {
                expand_nodes(children, abbreviations, seen);
            }
            i += 1;
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

    fn parse_with_abbrevs(input: &str) -> Root {
        parse(input)
    }

    #[test]
    fn no_frontmatter_is_noop() {
        let mut root = parse("HTML is great.\n");
        AbbreviationExpand.transform(&mut root, "");
        // No changes expected.
        match &root.children[0] {
            Node::Paragraph(p) => {
                assert!(
                    p.children.iter().all(|n| !matches!(n, Node::Component(_))),
                    "Should have no Abbr components without frontmatter"
                );
            }
            other => panic!("Expected paragraph, got {:?}", other),
        }
    }

    #[test]
    fn first_occurrence_wrapped() {
        let input = "---\nabbreviations:\n  HTML: HyperText Markup Language\n---\nHTML is a language. HTML again.\n";
        let mut root = parse_with_abbrevs(input);
        AbbreviationExpand.transform(&mut root, "");

        // Collect all Abbr components in the tree.
        let mut abbr_count = 0;
        crate::walk(&root.children, &mut |n| {
            if let Node::Component(c) = n {
                if c.name == "Abbr" {
                    abbr_count += 1;
                    let title = c.attributes.iter().find_map(|a| {
                        if a.name == "title" {
                            if let AttributeValue::String(s) = &a.value {
                                Some(s.as_str())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    });
                    assert_eq!(
                        title,
                        Some("HyperText Markup Language"),
                        "Abbr title should be the expansion"
                    );
                }
            }
        });
        assert_eq!(abbr_count, 1, "Only the first occurrence should be wrapped");
    }

    #[test]
    fn second_occurrence_left_as_text() {
        let input = "---\nabbreviations:\n  CSS: Cascading Style Sheets\n---\nCSS rules. CSS is cool.\n";
        let mut root = parse_with_abbrevs(input);
        AbbreviationExpand.transform(&mut root, "");

        let mut abbr_count = 0;
        crate::walk(&root.children, &mut |n| {
            if let Node::Component(c) = n {
                if c.name == "Abbr" {
                    abbr_count += 1;
                }
            }
        });
        assert_eq!(abbr_count, 1, "Should wrap only the first occurrence of CSS");
    }

    #[test]
    fn multiple_abbreviations_each_first_wrapped() {
        let input = "---\nabbreviations:\n  HTML: HyperText Markup Language\n  CSS: Cascading Style Sheets\n---\nHTML and CSS and HTML and CSS.\n";
        let mut root = parse_with_abbrevs(input);
        AbbreviationExpand.transform(&mut root, "");

        let mut abbr_count = 0;
        crate::walk(&root.children, &mut |n| {
            if let Node::Component(c) = n {
                if c.name == "Abbr" {
                    abbr_count += 1;
                }
            }
        });
        assert_eq!(abbr_count, 2, "First HTML and first CSS should each be wrapped once");
    }

    #[test]
    fn abbreviation_not_in_text_is_noop() {
        let input = "---\nabbreviations:\n  XML: Extensible Markup Language\n---\nNo abbreviations here.\n";
        let mut root = parse_with_abbrevs(input);
        AbbreviationExpand.transform(&mut root, "");

        let has_abbr = {
            let mut found = false;
            crate::walk(&root.children, &mut |n| {
                if let Node::Component(c) = n {
                    if c.name == "Abbr" {
                        found = true;
                    }
                }
            });
            found
        };
        assert!(!has_abbr, "No Abbr component should be created when abbreviation isn't in text");
    }

    #[test]
    fn abbr_component_has_correct_children() {
        let input = "---\nabbreviations:\n  API: Application Programming Interface\n---\nThe API endpoint.\n";
        let mut root = parse_with_abbrevs(input);
        AbbreviationExpand.transform(&mut root, "");

        let mut found_abbr = false;
        crate::walk(&root.children, &mut |n| {
            if let Node::Component(c) = n {
                if c.name == "Abbr" {
                    found_abbr = true;
                    // The Abbr component's child should be a Text node with value "API".
                    match c.children.first() {
                        Some(Node::Text(t)) => assert_eq!(t.value, "API"),
                        other => panic!("Expected Text child in Abbr, got {:?}", other),
                    }
                }
            }
        });
        assert!(found_abbr, "Should have found an Abbr component");
    }

    #[test]
    fn empty_abbreviations_map_is_noop() {
        let input = "---\nabbreviations: {}\n---\nSome text.\n";
        let mut root = parse_with_abbrevs(input);
        AbbreviationExpand.transform(&mut root, "");
        // Should not panic and should produce no Abbr components.
        let has_abbr = {
            let mut found = false;
            crate::walk(&root.children, &mut |n| {
                if matches!(n, Node::Component(c) if c.name == "Abbr") {
                    found = true;
                }
            });
            found
        };
        assert!(!has_abbr);
    }
}
