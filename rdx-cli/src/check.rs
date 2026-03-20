use rdx_ast::{Node, Root};
use std::collections::HashMap;
use std::path::Path;

use crate::ast_util::{collect_labels, walk_nodes, LabelEntry, LabelKind};

// ---------------------------------------------------------------------------
// Diagnostic types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckSeverity {
    Error,
    Warning,
    Info,
}

impl std::fmt::Display for CheckSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckSeverity::Error => write!(f, "error"),
            CheckSeverity::Warning => write!(f, "warning"),
            CheckSeverity::Info => write!(f, "info"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CheckDiagnostic {
    pub line: usize,
    pub column: usize,
    pub severity: CheckSeverity,
    pub message: String,
}

impl std::fmt::Display for CheckDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}: {}: {}",
            self.line, self.column, self.severity, self.message
        )
    }
}

// ---------------------------------------------------------------------------
// Cross-reference checking
// ---------------------------------------------------------------------------

/// Check that every `{@target}` cross-reference in `root` resolves to a
/// defined label in `labels`. Returns a [`CheckSeverity::Error`] diagnostic
/// for each undefined reference.
pub fn check_cross_refs(
    root: &Root,
    labels: &HashMap<String, LabelEntry>,
) -> Vec<CheckDiagnostic> {
    let mut diagnostics = Vec::new();

    walk_nodes(&root.children, &mut |node| {
        if let Node::CrossRef(cr) = node {
            if !labels.contains_key(&cr.target) {
                diagnostics.push(CheckDiagnostic {
                    line: cr.position.start.line,
                    column: cr.position.start.column,
                    severity: CheckSeverity::Error,
                    message: format!("undefined cross-reference {{@{}}}", cr.target),
                });
            }
        }
    });

    diagnostics
}

// ---------------------------------------------------------------------------
// Citation checking
// ---------------------------------------------------------------------------

/// Check citation keys in `root` against the bibliography path specified in
/// frontmatter.
///
/// - If no `bibliography` frontmatter key is set and there are citation nodes,
///   emits an `Info` diagnostic for each citation key (nothing to check against).
/// - If a bibliography path is set but cannot be read, emits an `Error`.
/// - If a bibliography is loaded, emits a `Warning` for each key not found in it.
pub fn check_citations(root: &Root) -> Vec<CheckDiagnostic> {
    let mut diagnostics = Vec::new();

    // Extract bibliography path from frontmatter if present.
    let bib_path: Option<String> = root
        .frontmatter
        .as_ref()
        .and_then(|fm| fm.get("bibliography"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Collect citation keys from the AST.
    let mut citation_uses: Vec<(String, usize, usize)> = Vec::new();
    walk_nodes(&root.children, &mut |node| {
        if let Node::Citation(c) = node {
            for key in &c.keys {
                citation_uses.push((
                    key.id.clone(),
                    c.position.start.line,
                    c.position.start.column,
                ));
            }
        }
    });

    match bib_path {
        Some(bib) => {
            // Try to load and parse the .bib file for defined keys.
            match std::fs::read_to_string(&bib) {
                Ok(contents) => {
                    let defined_keys = parse_bib_keys(&contents);
                    for (key, line, col) in &citation_uses {
                        if !defined_keys.contains(key.as_str()) {
                            diagnostics.push(CheckDiagnostic {
                                line: *line,
                                column: *col,
                                severity: CheckSeverity::Warning,
                                message: format!(
                                    "undefined citation key \"{}\" (not found in {})",
                                    key, bib
                                ),
                            });
                        }
                    }
                }
                Err(e) => {
                    diagnostics.push(CheckDiagnostic {
                        line: 1,
                        column: 1,
                        severity: CheckSeverity::Error,
                        message: format!(
                            "bibliography file \"{}\" could not be read: {}",
                            bib, e
                        ),
                    });
                }
            }
        }
        None => {
            // No bibliography: report citation keys as info.
            for (key, line, col) in &citation_uses {
                diagnostics.push(CheckDiagnostic {
                    line: *line,
                    column: *col,
                    severity: CheckSeverity::Info,
                    message: format!(
                        "citation key \"{}\" found (no bibliography to check against)",
                        key
                    ),
                });
            }
        }
    }

    diagnostics
}

/// Minimal .bib key parser: extracts citable keys from `@type{key,` entries.
///
/// Handles:
/// - BibTeX line comments (`%` prefix)
/// - `@string` and `@preamble` entries (skipped — they are not citable keys)
/// - Keys that contain special characters (anything up to the first comma or `}`)
fn parse_bib_keys(content: &str) -> std::collections::HashSet<String> {
    let mut keys = std::collections::HashSet::new();
    for line in content.lines() {
        let trimmed = line.trim();
        // Skip BibTeX comment lines.
        if trimmed.starts_with('%') {
            continue;
        }
        if trimmed.starts_with('@') {
            // Find the opening brace.
            if let Some(brace_pos) = trimmed.find('{') {
                let entry_type = trimmed[1..brace_pos].trim().to_ascii_lowercase();
                // Skip non-citable meta-entries.
                if entry_type == "string" || entry_type == "preamble" || entry_type == "comment" {
                    continue;
                }
                let rest = &trimmed[brace_pos + 1..];
                // The key ends at the first comma or closing brace.
                let key_end = rest
                    .find(|c| c == ',' || c == '}')
                    .unwrap_or(rest.len());
                let key = rest[..key_end].trim();
                if !key.is_empty() {
                    keys.insert(key.to_string());
                }
            }
        }
    }
    keys
}

// ---------------------------------------------------------------------------
// Link checking
// ---------------------------------------------------------------------------

/// Check that all links in the document resolve correctly.
///
/// Internal anchor links (`#fragment`) are checked against the label map.
/// Relative file links (`./` or `../`) are checked for existence on disk.
/// Absolute URLs (`http://`, `https://`) are not checked.
pub fn check_links(
    root: &Root,
    labels: &HashMap<String, LabelEntry>,
    doc_path: &Path,
) -> Vec<CheckDiagnostic> {
    let mut diagnostics = Vec::new();
    let doc_dir = doc_path.parent().unwrap_or(Path::new("."));

    walk_nodes(&root.children, &mut |node| {
        if let Node::Link(l) = node {
            let url = &l.url;
            if let Some(fragment) = url.strip_prefix('#') {
                // Internal anchor link — check heading ID exists.
                if !labels.contains_key(fragment) {
                    diagnostics.push(CheckDiagnostic {
                        line: l.position.start.line,
                        column: l.position.start.column,
                        severity: CheckSeverity::Error,
                        message: format!(
                            "broken internal link: anchor \"#{}\" has no matching heading id",
                            fragment
                        ),
                    });
                }
            } else if url.starts_with("./") || url.starts_with("../") {
                // Relative file link — check file exists on disk.
                let target = doc_dir.join(url);
                if !target.exists() {
                    diagnostics.push(CheckDiagnostic {
                        line: l.position.start.line,
                        column: l.position.start.column,
                        severity: CheckSeverity::Error,
                        message: format!("broken relative link: file \"{}\" not found", url),
                    });
                }
            }
            // Absolute URLs (http/https) are not checked.
        }
    });

    diagnostics
}

// ---------------------------------------------------------------------------
// Image checking
// ---------------------------------------------------------------------------

/// Check that image references are valid.
///
/// Emits a [`CheckSeverity::Warning`] for images with empty or missing alt
/// text, and a [`CheckSeverity::Error`] for relative image paths that do not
/// exist on disk. Absolute URLs and `data:` URIs are not checked.
pub fn check_images(root: &Root, doc_path: &Path) -> Vec<CheckDiagnostic> {
    let mut diagnostics = Vec::new();
    let doc_dir = doc_path.parent().unwrap_or(Path::new("."));

    walk_nodes(&root.children, &mut |node| {
        if let Node::Image(img) = node {
            // Warn on empty alt text.
            let alt_empty = img.alt.as_deref().map(|a| a.is_empty()).unwrap_or(true);
            if alt_empty {
                diagnostics.push(CheckDiagnostic {
                    line: img.position.start.line,
                    column: img.position.start.column,
                    severity: CheckSeverity::Warning,
                    message: "image has empty or missing alt text".to_string(),
                });
            }

            // Check relative image paths exist on disk.
            let url = &img.url;
            if !url.starts_with("http://")
                && !url.starts_with("https://")
                && !url.starts_with("data:")
            {
                let target = doc_dir.join(url);
                if !target.exists() {
                    diagnostics.push(CheckDiagnostic {
                        line: img.position.start.line,
                        column: img.position.start.column,
                        severity: CheckSeverity::Error,
                        message: format!("missing image file: \"{}\" not found", url),
                    });
                }
            }
        }
    });

    diagnostics
}

// ---------------------------------------------------------------------------
// Heading level checking
// ---------------------------------------------------------------------------

/// Check heading hierarchy for common issues.
///
/// Emits a [`CheckSeverity::Warning`] when:
/// - The document contains more than one h1 heading.
/// - A heading skips one or more levels (e.g. h1 followed by h3).
pub fn check_heading_levels(root: &Root) -> Vec<CheckDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut h1_count = 0usize;
    let mut prev_depth: Option<u8> = None;

    // Only walk top-level children for heading-level checks — we look at
    // document-order headings regardless of nesting depth.
    let mut headings: Vec<(u8, usize, usize)> = Vec::new();
    walk_nodes(&root.children, &mut |node| {
        if let Node::Heading(b) = node {
            if let Some(depth) = b.depth {
                headings.push((depth, b.position.start.line, b.position.start.column));
            }
        }
    });

    for (depth, line, col) in headings {
        if depth == 1 {
            h1_count += 1;
            if h1_count > 1 {
                diagnostics.push(CheckDiagnostic {
                    line,
                    column: col,
                    severity: CheckSeverity::Warning,
                    message: "multiple h1 headings in document".to_string(),
                });
            }
        }

        if let Some(prev) = prev_depth {
            // A skip is when the new depth is more than one level deeper.
            if depth > prev + 1 {
                diagnostics.push(CheckDiagnostic {
                    line,
                    column: col,
                    severity: CheckSeverity::Warning,
                    message: format!(
                        "skipped heading level (h{} -> h{})",
                        prev, depth
                    ),
                });
            }
        }

        prev_depth = Some(depth);
    }

    diagnostics
}

// ---------------------------------------------------------------------------
// Top-level check runner
// ---------------------------------------------------------------------------

/// Configuration for a document check run.
///
/// Both flags default to `false`. Use [`Default::default()`] or the
/// `#[derive(Default)]` implementation to create an options value with all
/// optional checks disabled.
#[derive(Debug, Clone, Default)]
pub struct CheckOptions {
    /// When true, check that relative links and internal anchors resolve.
    pub check_links: bool,
    /// When true, check that referenced image files exist and have alt text.
    pub check_images: bool,
}

/// The results of running all enabled check passes on a document.
pub struct CheckResults {
    /// All diagnostics produced, sorted by (line, column).
    pub diagnostics: Vec<CheckDiagnostic>,
    /// True if any diagnostic has severity [`CheckSeverity::Error`].
    pub has_errors: bool,
}

/// Run all enabled document check passes and return the combined results.
///
/// The following checks always run: cross-reference validation, citation
/// validation, and heading-level ordering. Link and image checks are
/// controlled by `opts`.
pub fn run_check(
    root: &Root,
    doc_path: &Path,
    opts: &CheckOptions,
) -> CheckResults {
    let mut diagnostics: Vec<CheckDiagnostic> = Vec::new();

    // Collect labels once; reused by cross-ref and link checks.
    let labels = collect_labels(root);

    // Cross-reference checking (always).
    diagnostics.extend(check_cross_refs(root, &labels));

    // Citation checking (always).
    diagnostics.extend(check_citations(root));

    // Link checking (optional).
    if opts.check_links {
        diagnostics.extend(check_links(root, &labels, doc_path));
    }

    // Image checking (optional).
    if opts.check_images {
        diagnostics.extend(check_images(root, doc_path));
    }

    // Heading level checking (always).
    diagnostics.extend(check_heading_levels(root));

    // Sort diagnostics by line then column for predictable output.
    diagnostics.sort_by_key(|d| (d.line, d.column));

    let has_errors = diagnostics
        .iter()
        .any(|d| d.severity == CheckSeverity::Error);

    CheckResults {
        diagnostics,
        has_errors,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rdx_ast::*;
    use crate::test_helpers::{span, make_root};

    fn heading(depth: u8, id: Option<&str>, line: usize) -> Node {
        Node::Heading(StandardBlockNode {
            depth: Some(depth),
            ordered: None,
            checked: None,
            id: id.map(|s| s.to_string()),
            children: vec![],
            position: span(line, 1, 0, line, 20, 19),
        })
    }

    fn cross_ref(target: &str, line: usize) -> Node {
        Node::CrossRef(CrossRefNode {
            target: target.to_string(),
            position: span(line, 1, 0, line, 15, 14),
        })
    }

    // --- cross-ref tests ---

    #[test]
    fn cross_ref_defined_label_no_error() {
        let root = make_root(vec![
            heading(1, Some("sec:intro"), 1),
            cross_ref("sec:intro", 5),
        ]);
        let labels = collect_labels(&root);
        let diags = check_cross_refs(&root, &labels);
        assert!(diags.is_empty(), "expected no diagnostics, got: {:?}", diags);
    }

    #[test]
    fn cross_ref_undefined_label_is_error() {
        let root = make_root(vec![cross_ref("fig:missing", 10)]);
        let labels = collect_labels(&root);
        let diags = check_cross_refs(&root, &labels);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, CheckSeverity::Error);
        assert!(diags[0].message.contains("fig:missing"));
    }

    #[test]
    fn cross_ref_multiple_undefined() {
        let root = make_root(vec![
            cross_ref("tbl:missing", 3),
            cross_ref("eq:missing", 7),
        ]);
        let labels = collect_labels(&root);
        let diags = check_cross_refs(&root, &labels);
        assert_eq!(diags.len(), 2);
        assert!(diags.iter().all(|d| d.severity == CheckSeverity::Error));
    }

    #[test]
    fn math_display_label_is_collected() {
        let root = make_root(vec![Node::MathDisplay(MathDisplayNode {
            raw: "E = mc^2".into(),
            tree: MathExpr::Ident { value: "E".into() },
            label: Some("eq:einstein".into()),
            position: span(5, 1, 0, 5, 20, 19),
        })]);
        let labels = collect_labels(&root);
        assert!(labels.contains_key("eq:einstein"));
        assert_eq!(labels["eq:einstein"].kind, LabelKind::Math);
    }

    // --- heading level tests ---

    #[test]
    fn heading_levels_sequential_no_warning() {
        let root = make_root(vec![
            heading(1, None, 1),
            heading(2, None, 5),
            heading(3, None, 10),
        ]);
        let diags = check_heading_levels(&root);
        assert!(diags.is_empty(), "expected no diagnostics, got: {:?}", diags);
    }

    #[test]
    fn heading_level_skip_produces_warning() {
        let root = make_root(vec![
            heading(1, None, 1),
            heading(3, None, 5), // skip h2
        ]);
        let diags = check_heading_levels(&root);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, CheckSeverity::Warning);
        assert!(diags[0].message.contains("h1") && diags[0].message.contains("h3"));
    }

    #[test]
    fn multiple_h1_produces_warning() {
        let root = make_root(vec![
            heading(1, None, 1),
            heading(1, None, 10),
        ]);
        let diags = check_heading_levels(&root);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, CheckSeverity::Warning);
        assert!(diags[0].message.contains("multiple h1"));
    }

    #[test]
    fn heading_level_jump_back_is_fine() {
        // h1 -> h2 -> h3 -> h2 is valid (going back up is ok)
        let root = make_root(vec![
            heading(1, None, 1),
            heading(2, None, 2),
            heading(3, None, 3),
            heading(2, None, 4),
        ]);
        let diags = check_heading_levels(&root);
        assert!(diags.is_empty(), "going back up should not warn, got: {:?}", diags);
    }

    // --- collect_labels ---

    #[test]
    fn collect_labels_from_heading_id() {
        let root = make_root(vec![heading(2, Some("sec:methods"), 3)]);
        let labels = collect_labels(&root);
        assert!(labels.contains_key("sec:methods"));
        assert_eq!(labels["sec:methods"].kind, LabelKind::Heading);
    }

    #[test]
    fn collect_labels_from_component_id_attr() {
        let root = make_root(vec![Node::Component(ComponentNode {
            name: "Figure".into(),
            is_inline: false,
            attributes: vec![AttributeNode {
                name: "id".into(),
                value: AttributeValue::String("fig:arch".into()),
                position: span(10, 9, 0, 10, 20, 11),
            }],
            children: vec![],
            raw_content: String::new(),
            position: span(10, 1, 0, 12, 1, 50),
        })]);
        let labels = collect_labels(&root);
        assert!(labels.contains_key("fig:arch"));
        assert_eq!(labels["fig:arch"].kind, LabelKind::Component);
    }

    // --- parse_bib_keys ---

    #[test]
    fn parse_bib_keys_basic() {
        let bib = "@article{smith2024,\n  title = {Title},\n}\n";
        let keys = parse_bib_keys(bib);
        assert!(keys.contains("smith2024"));
        assert_eq!(keys.len(), 1);
    }

    #[test]
    fn parse_bib_keys_skips_line_comments() {
        let bib = "% This is a comment\n@article{jones2023,\n  title = {T},\n}\n";
        let keys = parse_bib_keys(bib);
        assert!(keys.contains("jones2023"));
        assert_eq!(keys.len(), 1);
    }

    #[test]
    fn parse_bib_keys_skips_string_and_preamble() {
        let bib = "@string{conf = {Conference}}\n@preamble{\"Some preamble\"}\n@article{citable,\n}\n";
        let keys = parse_bib_keys(bib);
        assert!(keys.contains("citable"), "expected citable key, got: {:?}", keys);
        assert!(!keys.contains("conf"), "@string entries should not be citable keys");
        assert_eq!(keys.len(), 1);
    }

    #[test]
    fn parse_bib_keys_skips_comment_entry_type() {
        // @comment{...} is a BibTeX comment block, not a citable entry.
        let bib = "@comment{This is a comment block}\n@book{realentry,}\n";
        let keys = parse_bib_keys(bib);
        assert!(keys.contains("realentry"));
        assert!(!keys.contains("This is a comment block"));
        assert_eq!(keys.len(), 1);
    }
}
