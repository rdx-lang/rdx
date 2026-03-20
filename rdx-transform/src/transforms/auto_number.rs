use std::cell::RefCell;
use std::collections::HashMap;

use rdx_ast::*;

use crate::{Transform, synthetic_pos};

// ---------------------------------------------------------------------------
// Public registry types
// ---------------------------------------------------------------------------

/// A single numbered element's metadata.
#[derive(Debug, Clone)]
pub struct NumberEntry {
    /// Conceptual kind: "Figure", "Table", "Listing", "Theorem", "Lemma",
    /// "Corollary", "Proposition", "Conjecture", "Definition", "Example",
    /// "Remark", "Equation", "Section".
    pub kind: String,
    /// Display number string: "1", "2", "1.1", etc.
    pub number: String,
    /// Optional caption or theorem title extracted from the element.
    pub title: Option<String>,
}

/// Maps label strings to their numbered entries.
///
/// Built as a side-effect of running [`AutoNumber`] and accessible via
/// [`AutoNumber::registry`] after the transform has been applied.
#[derive(Debug, Default)]
pub struct NumberRegistry {
    /// label -> entry
    pub entries: HashMap<String, NumberEntry>,
}

// ---------------------------------------------------------------------------
// Internal counter state
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
struct Counters {
    figure: u32,
    table: u32,
    listing: u32,
    /// Shared counter for: Theorem, Lemma, Corollary, Proposition, Conjecture.
    theorem_group: u32,
    /// Shared counter for: Definition, Example, Remark.
    definition_group: u32,
    equation: u32,
    /// Section hierarchy stack: [h1_count, h2_count, h3_count, ...]
    sections: Vec<u32>,
}

impl Counters {
    /// Increment and return the section number for the given 1-based `depth`,
    /// resetting all deeper counters. Returns a dot-separated string like "1",
    /// "1.2", or "1.2.3".
    fn next_section(&mut self, depth: u8) -> String {
        let idx = (depth as usize).saturating_sub(1);
        // Extend the stack if needed.
        if self.sections.len() <= idx {
            self.sections.resize(idx + 1, 0);
        }
        // Increment this level and zero out deeper levels.
        self.sections[idx] += 1;
        self.sections.truncate(idx + 1);

        self.sections
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(".")
    }
}

// ---------------------------------------------------------------------------
// Transform
// ---------------------------------------------------------------------------

/// Walks the AST and assigns sequential numbers to figures, tables, listings,
/// theorem-group components, definition-group components, display math
/// equations, and headings.
///
/// Numbers are injected as a `number` string attribute on `Component` nodes.
/// For `MathDisplay` nodes (which are not components), the numbers are only
/// stored in the registry.
///
/// After calling [`Transform::transform`], retrieve the full registry via
/// [`AutoNumber::registry`].
///
/// # Example
///
/// ```rust
/// use rdx_transform::{AutoNumber, Transform, parse};
///
/// let mut root = parse("<Figure id=\"fig:arch\">\n</Figure>\n");
/// let numberer = AutoNumber::new();
/// numberer.transform(&mut root, "");
/// let reg = numberer.registry();
/// assert_eq!(reg.entries["fig:arch"].number, "1");
/// ```
pub struct AutoNumber {
    /// If true, number headings hierarchically (1, 1.1, 1.1.2).
    pub number_headings: bool,
    /// If true, prefix figure/table numbers with chapter (Figure 2.3 vs Figure 7).
    /// When enabled, the chapter counter is the h1 section counter.
    pub per_chapter: bool,
    /// Interior mutability via `RefCell` is required because the [`Transform`]
    /// trait takes `&self` (not `&mut self`), yet we need to mutate the registry
    /// during `transform()`. Transforms run single-threaded, so `RefCell` is
    /// the appropriate lightweight primitive here (no locking overhead).
    registry: RefCell<NumberRegistry>,
}

impl AutoNumber {
    pub fn new() -> Self {
        AutoNumber {
            number_headings: true,
            per_chapter: false,
            registry: RefCell::new(NumberRegistry::default()),
        }
    }

    /// Return a shared borrow of the registry built during the last call to
    /// [`Transform::transform`].
    ///
    /// # Panics
    ///
    /// Panics if the registry is already mutably borrowed elsewhere. This
    /// should never happen because transforms run single-threaded.
    pub fn registry(&self) -> std::cell::Ref<'_, NumberRegistry> {
        self.registry.borrow()
    }
}

impl Default for AutoNumber {
    fn default() -> Self {
        Self::new()
    }
}

impl Transform for AutoNumber {
    fn name(&self) -> &str {
        "auto-number"
    }

    fn transform(&self, root: &mut Root, _source: &str) {
        // Clear any previous run.
        self.registry.borrow_mut().entries.clear();

        let mut counters = Counters::default();
        process_nodes(
            &mut root.children,
            &mut counters,
            self.number_headings,
            self.per_chapter,
            &self.registry,
        );
    }
}

// ---------------------------------------------------------------------------
// Core recursive walker
// ---------------------------------------------------------------------------

/// Helper: look up the string value of an attribute by name.
fn attr_str<'a>(comp: &'a ComponentNode, name: &str) -> Option<&'a str> {
    comp.attributes.iter().find_map(|a| {
        if a.name == name {
            if let AttributeValue::String(s) = &a.value {
                Some(s.as_str())
            } else {
                None
            }
        } else {
            None
        }
    })
}

/// Helper: extract plain text from a node's children (best-effort caption).
fn children_text(nodes: &[Node]) -> String {
    let mut out = String::new();
    crate::walk(nodes, &mut |n| {
        if let Node::Text(t) = n {
            out.push_str(&t.value);
        }
    });
    out
}

/// Inject or replace a `number` attribute on a component.
fn inject_number(comp: &mut ComponentNode, number: &str) {
    // Remove any existing `number` attribute first.
    comp.attributes.retain(|a| a.name != "number");
    comp.attributes.push(AttributeNode {
        name: "number".to_string(),
        value: AttributeValue::String(number.to_string()),
        position: synthetic_pos(),
    });
}

/// Format a plain counter as a chapter-prefixed string when `per_chapter` is
/// true. The chapter is the first element of `sections` (h1 counter).
fn format_number(counter: u32, sections: &[u32], per_chapter: bool) -> String {
    if per_chapter {
        let chapter = sections.first().copied().unwrap_or(0);
        format!("{}.{}", chapter, counter)
    } else {
        counter.to_string()
    }
}

fn process_nodes(
    nodes: &mut Vec<Node>,
    counters: &mut Counters,
    number_headings: bool,
    per_chapter: bool,
    registry_mutex: &RefCell<NumberRegistry>,
) {
    for node in nodes.iter_mut() {
        match node {
            // ---------------------------------------------------------------
            // Headings
            // ---------------------------------------------------------------
            Node::Heading(h) => {
                if number_headings {
                    if let Some(depth) = h.depth {
                        let num = counters.next_section(depth);
                        if let Some(ref id) = h.id.clone() {
                            let title = children_text(&h.children);
                            let mut reg =
                                registry_mutex.borrow_mut();
                            reg.entries.insert(
                                id.clone(),
                                NumberEntry {
                                    kind: "Section".to_string(),
                                    number: num.clone(),
                                    title: if title.is_empty() { None } else { Some(title) },
                                },
                            );
                        }
                        // Recurse into heading children.
                        process_nodes(
                            &mut h.children,
                            counters,
                            number_headings,
                            per_chapter,
                            registry_mutex,
                        );
                    }
                } else {
                    process_nodes(
                        &mut h.children,
                        counters,
                        number_headings,
                        per_chapter,
                        registry_mutex,
                    );
                }
            }

            // ---------------------------------------------------------------
            // MathDisplay
            // ---------------------------------------------------------------
            Node::MathDisplay(m) => {
                if let Some(ref label) = m.label.clone() {
                    counters.equation += 1;
                    let num = counters.equation.to_string();
                    let mut reg = registry_mutex.borrow_mut();
                    reg.entries.insert(
                        label.clone(),
                        NumberEntry {
                            kind: "Equation".to_string(),
                            number: num,
                            title: None,
                        },
                    );
                }
            }

            // ---------------------------------------------------------------
            // Components
            // ---------------------------------------------------------------
            Node::Component(comp) => {
                let name = comp.name.clone();
                match name.as_str() {
                    "Figure" => {
                        counters.figure += 1;
                        let num =
                            format_number(counters.figure, &counters.sections, per_chapter);
                        inject_number(comp, &num);
                        if let Some(id) = attr_str(comp, "id").map(str::to_string) {
                            let title = children_text(&comp.children);
                            let mut reg = registry_mutex.borrow_mut();
                            reg.entries.insert(
                                id,
                                NumberEntry {
                                    kind: "Figure".to_string(),
                                    number: num,
                                    title: if title.is_empty() { None } else { Some(title) },
                                },
                            );
                        }
                    }
                    "TableFigure" => {
                        counters.table += 1;
                        let num =
                            format_number(counters.table, &counters.sections, per_chapter);
                        inject_number(comp, &num);
                        if let Some(id) = attr_str(comp, "id").map(str::to_string) {
                            let title = children_text(&comp.children);
                            let mut reg = registry_mutex.borrow_mut();
                            reg.entries.insert(
                                id,
                                NumberEntry {
                                    kind: "Table".to_string(),
                                    number: num,
                                    title: if title.is_empty() { None } else { Some(title) },
                                },
                            );
                        }
                    }
                    "Listing" => {
                        counters.listing += 1;
                        let num =
                            format_number(counters.listing, &counters.sections, per_chapter);
                        inject_number(comp, &num);
                        if let Some(id) = attr_str(comp, "id").map(str::to_string) {
                            let title = children_text(&comp.children);
                            let mut reg = registry_mutex.borrow_mut();
                            reg.entries.insert(
                                id,
                                NumberEntry {
                                    kind: "Listing".to_string(),
                                    number: num,
                                    title: if title.is_empty() { None } else { Some(title) },
                                },
                            );
                        }
                    }
                    "Theorem" | "Lemma" | "Corollary" | "Proposition" | "Conjecture" => {
                        counters.theorem_group += 1;
                        let num = counters.theorem_group.to_string();
                        inject_number(comp, &num);
                        if let Some(id) = attr_str(comp, "id").map(str::to_string) {
                            let title = attr_str(comp, "title").map(str::to_string);
                            let mut reg = registry_mutex.borrow_mut();
                            reg.entries.insert(
                                id,
                                NumberEntry {
                                    kind: name.clone(),
                                    number: num,
                                    title,
                                },
                            );
                        }
                    }
                    "Definition" | "Example" | "Remark" => {
                        counters.definition_group += 1;
                        let num = counters.definition_group.to_string();
                        inject_number(comp, &num);
                        if let Some(id) = attr_str(comp, "id").map(str::to_string) {
                            let title = attr_str(comp, "title").map(str::to_string);
                            let mut reg = registry_mutex.borrow_mut();
                            reg.entries.insert(
                                id,
                                NumberEntry {
                                    kind: name.clone(),
                                    number: num,
                                    title,
                                },
                            );
                        }
                    }
                    _ => {}
                }
                // Always recurse into component children.
                process_nodes(
                    &mut comp.children,
                    counters,
                    number_headings,
                    per_chapter,
                    registry_mutex,
                );
            }

            // ---------------------------------------------------------------
            // All other container nodes — recurse
            // ---------------------------------------------------------------
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
            | Node::ThematicBreak(b)
            | Node::DefinitionList(b)
            | Node::DefinitionTerm(b)
            | Node::DefinitionDescription(b) => {
                process_nodes(
                    &mut b.children,
                    counters,
                    number_headings,
                    per_chapter,
                    registry_mutex,
                );
            }
            Node::Link(l) => {
                process_nodes(
                    &mut l.children,
                    counters,
                    number_headings,
                    per_chapter,
                    registry_mutex,
                );
            }
            Node::Image(i) => {
                process_nodes(
                    &mut i.children,
                    counters,
                    number_headings,
                    per_chapter,
                    registry_mutex,
                );
            }
            Node::FootnoteDefinition(f) => {
                process_nodes(
                    &mut f.children,
                    counters,
                    number_headings,
                    per_chapter,
                    registry_mutex,
                );
            }
            _ => {}
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

    #[test]
    fn figures_numbered_sequentially() {
        let mut root = parse(
            "<Figure id=\"fig:a\">\n</Figure>\n\
             <Figure id=\"fig:b\">\n</Figure>\n\
             <Figure id=\"fig:c\">\n</Figure>\n",
        );
        let numberer = AutoNumber::new();
        numberer.transform(&mut root, "");
        let reg = numberer.registry();
        assert_eq!(reg.entries["fig:a"].number, "1");
        assert_eq!(reg.entries["fig:b"].number, "2");
        assert_eq!(reg.entries["fig:c"].number, "3");
        assert_eq!(reg.entries["fig:a"].kind, "Figure");
    }

    #[test]
    fn tables_numbered_sequentially() {
        let mut root = parse(
            "<TableFigure id=\"tbl:a\">\n</TableFigure>\n\
             <TableFigure id=\"tbl:b\">\n</TableFigure>\n",
        );
        let numberer = AutoNumber::new();
        numberer.transform(&mut root, "");
        let reg = numberer.registry();
        assert_eq!(reg.entries["tbl:a"].number, "1");
        assert_eq!(reg.entries["tbl:b"].number, "2");
        assert_eq!(reg.entries["tbl:a"].kind, "Table");
    }

    #[test]
    fn theorem_group_shared_counter() {
        // Theorem, Lemma share the same counter.
        let mut root = parse(
            "<Theorem id=\"thm:one\">\n</Theorem>\n\
             <Lemma id=\"lem:two\">\n</Lemma>\n\
             <Corollary id=\"cor:three\">\n</Corollary>\n",
        );
        let numberer = AutoNumber::new();
        numberer.transform(&mut root, "");
        let reg = numberer.registry();
        assert_eq!(reg.entries["thm:one"].number, "1");
        assert_eq!(reg.entries["lem:two"].number, "2");
        assert_eq!(reg.entries["cor:three"].number, "3");
    }

    #[test]
    fn definition_group_shared_counter() {
        let mut root = parse(
            "<Definition id=\"def:one\">\n</Definition>\n\
             <Example id=\"ex:two\">\n</Example>\n\
             <Remark id=\"rem:three\">\n</Remark>\n",
        );
        let numberer = AutoNumber::new();
        numberer.transform(&mut root, "");
        let reg = numberer.registry();
        assert_eq!(reg.entries["def:one"].number, "1");
        assert_eq!(reg.entries["ex:two"].number, "2");
        assert_eq!(reg.entries["rem:three"].number, "3");
    }

    #[test]
    fn equations_numbered_from_label() {
        let mut root = parse(
            "$$ {#eq:first}\nE = mc^2\n$$\n\
             $$ {#eq:second}\na^2 + b^2 = c^2\n$$\n",
        );
        let numberer = AutoNumber::new();
        numberer.transform(&mut root, "");
        let reg = numberer.registry();
        assert_eq!(reg.entries["eq:first"].number, "1");
        assert_eq!(reg.entries["eq:second"].number, "2");
        assert_eq!(reg.entries["eq:first"].kind, "Equation");
    }

    #[test]
    fn number_attribute_injected_on_figure() {
        let mut root = parse("<Figure id=\"fig:x\">\n</Figure>\n");
        AutoNumber::new().transform(&mut root, "");
        match &root.children[0] {
            Node::Component(c) => {
                let num_attr = c.attributes.iter().find(|a| a.name == "number");
                assert!(num_attr.is_some(), "Expected 'number' attribute");
                assert_eq!(
                    num_attr.unwrap().value,
                    AttributeValue::String("1".to_string())
                );
            }
            other => panic!("Expected component, got {:?}", other),
        }
    }

    /// Build a Heading node with a preset `id` (since ENABLE_HEADING_ATTRIBUTES is
    /// not active in the parser, IDs must be injected programmatically in tests).
    fn make_heading(depth: u8, text: &str, id: &str) -> Node {
        let pos = Position {
            start: Point { line: 1, column: 1, offset: 0 },
            end: Point { line: 1, column: 1, offset: 0 },
        };
        Node::Heading(StandardBlockNode {
            depth: Some(depth),
            ordered: None,
            checked: None,
            id: Some(id.to_string()),
            children: vec![Node::Text(TextNode { value: text.to_string(), position: pos.clone() })],
            position: pos,
        })
    }

    #[test]
    fn headings_get_section_numbers() {
        // ENABLE_HEADING_ATTRIBUTES is not active in the parser so we must
        // construct the heading nodes with ids already set.
        let pos = Position {
            start: Point { line: 1, column: 1, offset: 0 },
            end: Point { line: 1, column: 1, offset: 0 },
        };
        let mut root = Root {
            node_type: RootType::Root,
            frontmatter: None,
            children: vec![
                make_heading(1, "Chapter One", "ch1"),
                make_heading(2, "Section A", "sec:a"),
                make_heading(2, "Section B", "sec:b"),
            ],
            position: pos,
        };
        let numberer = AutoNumber { number_headings: true, ..AutoNumber::new() };
        numberer.transform(&mut root, "");
        let reg = numberer.registry();
        assert_eq!(reg.entries["ch1"].number, "1");
        assert_eq!(reg.entries["sec:a"].number, "1.1");
        assert_eq!(reg.entries["sec:b"].number, "1.2");
    }

    #[test]
    fn per_chapter_prefixes_figure_numbers() {
        // ENABLE_HEADING_ATTRIBUTES is not active in the parser so we must
        // construct the heading node with an id already set.
        let numberer = AutoNumber {
            number_headings: true,
            per_chapter: true,
            ..AutoNumber::new()
        };
        // Parse the Figure component, then prepend a hand-crafted heading.
        let mut root = parse("<Figure id=\"fig:a\">\n</Figure>\n");
        root.children.insert(0, make_heading(1, "Chapter", "ch1"));
        numberer.transform(&mut root, "");
        let reg = numberer.registry();
        assert_eq!(reg.entries["fig:a"].number, "1.1");
    }

    #[test]
    fn re_run_clears_previous_registry() {
        let numberer = AutoNumber::new();
        let mut root = parse("<Figure id=\"fig:a\">\n</Figure>\n");
        numberer.transform(&mut root, "");
        numberer.transform(&mut root, "");
        // After second run the registry should still have just one entry.
        assert_eq!(numberer.registry().entries.len(), 1);
    }

    #[test]
    fn figures_and_tables_have_separate_counters() {
        let mut root = parse(
            "<Figure id=\"fig:a\">\n</Figure>\n\
             <TableFigure id=\"tbl:a\">\n</TableFigure>\n\
             <Figure id=\"fig:b\">\n</Figure>\n",
        );
        let numberer = AutoNumber::new();
        numberer.transform(&mut root, "");
        let reg = numberer.registry();
        assert_eq!(reg.entries["fig:a"].number, "1");
        assert_eq!(reg.entries["fig:b"].number, "2");
        assert_eq!(reg.entries["tbl:a"].number, "1");
    }
}
