use rdx_ast::{Node, Root};
use std::collections::HashMap;

use crate::ast_util::walk_nodes;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabelKind {
    Heading,
    Component,
    Math,
}

impl std::fmt::Display for LabelKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LabelKind::Heading => write!(f, "heading"),
            LabelKind::Component => write!(f, "component"),
            LabelKind::Math => write!(f, "math"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LabelInfo {
    pub key: String,
    pub kind: LabelKind,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct CrossRefInfo {
    pub target: String,
    pub line: usize,
    /// `Some` when the target is defined, `None` when undefined.
    pub defined_line: Option<usize>,
    /// Kind of the target, if defined.
    pub target_kind: Option<LabelKind>,
}

#[derive(Debug, Clone)]
pub struct CitationInfo {
    pub key: String,
    pub line: usize,
}

#[derive(Debug, Default)]
pub struct RefsReport {
    pub labels: Vec<LabelInfo>,
    pub cross_refs: Vec<CrossRefInfo>,
    pub citations: Vec<CitationInfo>,
}

// ---------------------------------------------------------------------------
// Collection
// ---------------------------------------------------------------------------

/// Collect all labels, cross-references, and citation keys from `root` in a
/// single pass. Returns a [`RefsReport`] suitable for display or further
/// analysis.
pub fn collect_refs(root: &Root) -> RefsReport {
    // --- Pass 1: collect labels ---
    let mut label_map: HashMap<String, LabelInfo> = HashMap::new();

    walk_nodes(&root.children, &mut |node| match node {
        Node::Heading(b) => {
            if let Some(ref id) = b.id {
                label_map.insert(
                    id.clone(),
                    LabelInfo {
                        key: id.clone(),
                        kind: LabelKind::Heading,
                        line: b.position.start.line,
                    },
                );
            }
        }
        Node::Component(c) => {
            for attr in &c.attributes {
                if attr.name == "id" {
                    if let rdx_ast::AttributeValue::String(ref id) = attr.value {
                        label_map.insert(
                            id.clone(),
                            LabelInfo {
                                key: id.clone(),
                                kind: LabelKind::Component,
                                line: c.position.start.line,
                            },
                        );
                    }
                }
            }
        }
        Node::MathDisplay(m) => {
            if let Some(ref label) = m.label {
                label_map.insert(
                    label.clone(),
                    LabelInfo {
                        key: label.clone(),
                        kind: LabelKind::Math,
                        line: m.position.start.line,
                    },
                );
            }
        }
        _ => {}
    });

    // Sort labels by line for predictable output.
    let mut labels: Vec<LabelInfo> = label_map.values().cloned().collect();
    labels.sort_by_key(|l| l.line);

    // --- Pass 2: collect cross-refs ---
    let mut cross_refs: Vec<CrossRefInfo> = Vec::new();
    walk_nodes(&root.children, &mut |node| {
        if let Node::CrossRef(cr) = node {
            let defined = label_map.get(&cr.target);
            cross_refs.push(CrossRefInfo {
                target: cr.target.clone(),
                line: cr.position.start.line,
                defined_line: defined.map(|l| l.line),
                target_kind: defined.map(|l| l.kind.clone()),
            });
        }
    });
    cross_refs.sort_by_key(|r| r.line);

    // --- Pass 3: collect citations ---
    let mut citations: Vec<CitationInfo> = Vec::new();
    walk_nodes(&root.children, &mut |node| {
        if let Node::Citation(c) = node {
            for key in &c.keys {
                citations.push(CitationInfo {
                    key: key.id.clone(),
                    line: c.position.start.line,
                });
            }
        }
    });
    citations.sort_by_key(|c| c.line);

    RefsReport {
        labels,
        cross_refs,
        citations,
    }
}

// ---------------------------------------------------------------------------
// Formatted output
// ---------------------------------------------------------------------------

/// Return the report as a formatted, human-readable string.
pub fn format_report(report: &RefsReport) -> String {
    let mut out = String::new();

    // Labels defined section.
    out.push_str("Labels defined:\n");
    if report.labels.is_empty() {
        out.push_str("  (none)\n");
    } else {
        // Column widths: key column is padded to the longest key.
        let max_key = report.labels.iter().map(|l| l.key.len()).max().unwrap_or(0);
        let max_kind = report
            .labels
            .iter()
            .map(|l| l.kind.to_string().len())
            .max()
            .unwrap_or(0);

        for label in &report.labels {
            out.push_str(&format!(
                "  {:<key_w$}  {:<kind_w$}  line {}\n",
                label.key,
                label.kind,
                label.line,
                key_w = max_key,
                kind_w = max_kind,
            ));
        }
    }

    out.push('\n');

    // Cross-references section.
    out.push_str("Cross-references:\n");
    if report.cross_refs.is_empty() {
        out.push_str("  (none)\n");
    } else {
        let max_target = report
            .cross_refs
            .iter()
            .map(|r| r.target.len() + 4) // {@target}
            .max()
            .unwrap_or(0);

        for cr in &report.cross_refs {
            let target_display = format!("{{@{}}}", cr.target);
            let resolution = match (&cr.target_kind, cr.defined_line) {
                (Some(kind), Some(def_line)) => {
                    // Capitalise kind for display.
                    let kind_str = match kind {
                        LabelKind::Heading => "Heading",
                        LabelKind::Component => "Component",
                        LabelKind::Math => "Equation",
                    };
                    format!("-> {} (defined line {})", kind_str, def_line)
                }
                _ => "-> UNDEFINED".to_string(),
            };
            out.push_str(&format!(
                "  {:<target_w$}  line {:<6}  {}\n",
                target_display,
                cr.line,
                resolution,
                target_w = max_target,
            ));
        }
    }

    out.push('\n');

    // Citations section.
    out.push_str("Citations:\n");
    if report.citations.is_empty() {
        out.push_str("  (none)\n");
    } else {
        let max_key = report
            .citations
            .iter()
            .map(|c| c.key.len() + 4) // [@key]
            .max()
            .unwrap_or(0);

        for cit in &report.citations {
            let key_display = format!("[@{}]", cit.key);
            out.push_str(&format!(
                "  {:<key_w$}  line {}\n",
                key_display,
                cit.line,
                key_w = max_key,
            ));
        }
    }

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

    fn heading_with_id(depth: u8, id: &str, line: usize) -> Node {
        Node::Heading(StandardBlockNode {
            depth: Some(depth),
            ordered: None,
            checked: None,
            id: Some(id.to_string()),
            children: vec![],
            position: span(line, 1, 0, line, 20, 19),
        })
    }

    fn cross_ref_node(target: &str, line: usize) -> Node {
        Node::CrossRef(CrossRefNode {
            target: target.to_string(),
            position: span(line, 1, 0, line, 15, 14),
        })
    }

    fn citation_node(key: &str, line: usize) -> Node {
        Node::Citation(CitationNode {
            keys: vec![CitationKey {
                id: key.to_string(),
                prefix: None,
                locator: None,
            }],
            position: span(line, 1, 0, line, 15, 14),
        })
    }

    #[test]
    fn collect_labels_from_heading() {
        let root = make_root(vec![heading_with_id(1, "sec:intro", 5)]);
        let report = collect_refs(&root);
        assert_eq!(report.labels.len(), 1);
        assert_eq!(report.labels[0].key, "sec:intro");
        assert_eq!(report.labels[0].kind, LabelKind::Heading);
        assert_eq!(report.labels[0].line, 5);
    }

    #[test]
    fn cross_ref_to_defined_label_resolved() {
        let root = make_root(vec![
            heading_with_id(1, "sec:intro", 5),
            cross_ref_node("sec:intro", 12),
        ]);
        let report = collect_refs(&root);
        assert_eq!(report.cross_refs.len(), 1);
        assert_eq!(report.cross_refs[0].target, "sec:intro");
        assert_eq!(report.cross_refs[0].defined_line, Some(5));
        assert_eq!(report.cross_refs[0].target_kind, Some(LabelKind::Heading));
    }

    #[test]
    fn cross_ref_to_undefined_label_is_none() {
        let root = make_root(vec![cross_ref_node("fig:missing", 10)]);
        let report = collect_refs(&root);
        assert_eq!(report.cross_refs.len(), 1);
        assert!(report.cross_refs[0].defined_line.is_none());
        assert!(report.cross_refs[0].target_kind.is_none());
    }

    #[test]
    fn citations_collected_correctly() {
        let root = make_root(vec![
            citation_node("smith2024", 8),
            citation_node("jones2023", 15),
        ]);
        let report = collect_refs(&root);
        assert_eq!(report.citations.len(), 2);
        assert_eq!(report.citations[0].key, "smith2024");
        assert_eq!(report.citations[0].line, 8);
        assert_eq!(report.citations[1].key, "jones2023");
        assert_eq!(report.citations[1].line, 15);
    }

    #[test]
    fn format_report_contains_labels_and_refs() {
        let root = make_root(vec![
            heading_with_id(1, "sec:intro", 5),
            cross_ref_node("sec:intro", 12),
            cross_ref_node("fig:missing", 20),
            citation_node("smith2024", 8),
        ]);
        let report = collect_refs(&root);
        let output = format_report(&report);

        assert!(output.contains("sec:intro"));
        assert!(output.contains("heading"));
        assert!(output.contains("line 5"));
        assert!(output.contains("{@sec:intro}"));
        assert!(output.contains("{@fig:missing}"));
        assert!(output.contains("UNDEFINED"));
        assert!(output.contains("[@smith2024]"));
    }

    #[test]
    fn format_report_shows_none_when_empty() {
        let root = make_root(vec![]);
        let report = collect_refs(&root);
        let output = format_report(&report);
        assert!(output.contains("(none)"));
    }

    #[test]
    fn math_display_label_collected() {
        let root = make_root(vec![Node::MathDisplay(MathDisplayNode {
            raw: "E=mc^2".into(),
            tree: MathExpr::Ident { value: "E".into() },
            label: Some("eq:einstein".into()),
            position: span(45, 1, 0, 47, 1, 50),
        })]);
        let report = collect_refs(&root);
        assert_eq!(report.labels.len(), 1);
        assert_eq!(report.labels[0].kind, LabelKind::Math);
        assert_eq!(report.labels[0].line, 45);
    }
}
