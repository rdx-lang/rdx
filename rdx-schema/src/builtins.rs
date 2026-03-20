use crate::{ComponentSchema, PropSchema, PropType, Schema};

// ---------------------------------------------------------------------------
// Helper: base schema shared by all theorem-like environments
// ---------------------------------------------------------------------------

fn theorem_base() -> ComponentSchema {
    ComponentSchema::new()
        .prop("id", PropSchema::optional(PropType::String))
        .prop("title", PropSchema::optional(PropType::String))
}

// ---------------------------------------------------------------------------
// Helper: shared schema for admonition components
// ---------------------------------------------------------------------------

fn admonition_schema() -> ComponentSchema {
    ComponentSchema::new()
        .prop("title", PropSchema::optional(PropType::String))
        .prop("collapsible", PropSchema::optional(PropType::Boolean))
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Returns a [`Schema`] pre-loaded with all standard RDX components and global
/// attributes as specified in Section 7 of the RDX specification.
///
/// The returned schema is in non-strict mode by default so that user-defined
/// components are still accepted alongside the standard ones. Call
/// `.strict(true)` on the result if you want to reject unknown components.
pub fn standard_schema() -> Schema {
    let schema = Schema::new()
        // ------------------------------------------------------------------
        // Global attributes — valid on every component
        // ------------------------------------------------------------------
        .global_prop(
            "target",
            PropSchema::enum_optional(vec!["web", "print", "all"])
                .with_default(serde_json::Value::String("all".into()))
                .with_description("Output target filter"),
        )
        .global_prop(
            "id",
            PropSchema::optional(PropType::String)
                .with_description("Cross-reference anchor identifier"),
        )
        .global_prop(
            "class",
            PropSchema::optional(PropType::String)
                .with_description("CSS class (web) or style mapping (print)"),
        )
        // ------------------------------------------------------------------
        // 7.1 Admonitions
        // ------------------------------------------------------------------
        .component("Note", admonition_schema().description("Supplementary information the reader should be aware of"))
        .component("Tip", admonition_schema().description("Helpful suggestion or best practice"))
        .component("Important", admonition_schema().description("Key information the reader must not overlook"))
        .component("Warning", admonition_schema().description("Potential issue that could cause problems"))
        .component("Caution", admonition_schema().description("Action that could lead to data loss or irreversible consequences"))
        // ------------------------------------------------------------------
        // 7.2 Figures, Tables & Listings
        // ------------------------------------------------------------------
        .component(
            "Figure",
            ComponentSchema::new()
                .prop("id", PropSchema::optional(PropType::String).with_description("Cross-reference label"))
                .prop("caption", PropSchema::required(PropType::String).with_description("Descriptive caption"))
                .description("Wraps images and illustrations with a numbered caption"),
        )
        .component(
            "TableFigure",
            ComponentSchema::new()
                .prop("id", PropSchema::optional(PropType::String).with_description("Cross-reference label"))
                .prop("caption", PropSchema::required(PropType::String).with_description("Descriptive caption"))
                .description("Wraps markdown tables with a numbered caption"),
        )
        .component(
            "Listing",
            ComponentSchema::new()
                .prop("id", PropSchema::optional(PropType::String).with_description("Cross-reference label"))
                .prop("caption", PropSchema::required(PropType::String).with_description("Descriptive caption"))
                .description("Wraps code blocks with a numbered caption"),
        )
        // ------------------------------------------------------------------
        // 7.3 Interactive Components
        // ------------------------------------------------------------------
        .component(
            "Tabs",
            ComponentSchema::new()
                .allowed_children(vec!["Tab"])
                .description("Tabbed interface container; must contain only <Tab> children"),
        )
        .component(
            "Tab",
            ComponentSchema::new()
                .prop("label", PropSchema::required(PropType::String).with_description("Tab label text"))
                .description("A single tab pane inside <Tabs>"),
        )
        .component(
            "Accordion",
            ComponentSchema::new()
                .allowed_children(vec!["AccordionItem"])
                .description("Collapsible accordion container; must contain only <AccordionItem> children"),
        )
        .component(
            "AccordionItem",
            ComponentSchema::new()
                .prop("title", PropSchema::required(PropType::String).with_description("Section heading"))
                .prop("defaultOpen", PropSchema::optional(PropType::Boolean).with_description("Whether the item starts open"))
                .description("A single collapsible item inside <Accordion>"),
        )
        .component(
            "Steps",
            ComponentSchema::new()
                .description("Numbered steps; each child heading becomes a step boundary"),
        )
        .component(
            "CodeGroup",
            ComponentSchema::new()
                .description("Tabbed code variant group; each child code block's title becomes the tab label"),
        )
        // ------------------------------------------------------------------
        // 7.4 Conditional Rendering
        // ------------------------------------------------------------------
        .component(
            "WebOnly",
            ComponentSchema::new()
                .description("Content rendered only in web output; stripped in print"),
        )
        .component(
            "PrintOnly",
            ComponentSchema::new()
                .description("Content rendered only in print output; stripped on web"),
        )
        // ------------------------------------------------------------------
        // 7.5 Page & Layout Control (Print)
        // ------------------------------------------------------------------
        .component(
            "PageBreak",
            ComponentSchema::new()
                .self_closing(true)
                .description("Force a new page (print only)"),
        )
        .component(
            "ColumnBreak",
            ComponentSchema::new()
                .self_closing(true)
                .description("Break to next column in multi-column layout (print only)"),
        )
        .component(
            "Spread",
            ComponentSchema::new()
                .description("Content spans both pages of an open book spread (print only)"),
        )
        // ------------------------------------------------------------------
        // 7.6 Book Front & Back Matter
        // ------------------------------------------------------------------
        .component(
            "Abstract",
            ComponentSchema::new()
                .description("Indented abstract block with 'Abstract' heading (print)"),
        )
        .component(
            "Dedication",
            ComponentSchema::new()
                .description("Centered dedication text on its own page (print)"),
        )
        .component(
            "Epigraph",
            ComponentSchema::new()
                .prop(
                    "attribution",
                    PropSchema::optional(PropType::String).with_description("Quote source attribution"),
                )
                .description("Right-aligned epigraph quote with optional attribution"),
        )
        .component(
            "Colophon",
            ComponentSchema::new()
                .description("Production notes, typically on the last page (print)"),
        )
        // ------------------------------------------------------------------
        // 7.7 Academic Environments
        // ------------------------------------------------------------------
        .component("Theorem", theorem_base().description("Theorem environment with italic body"))
        .component("Lemma", theorem_base().description("Lemma environment with italic body"))
        .component("Corollary", theorem_base().description("Corollary environment with italic body"))
        .component("Proposition", theorem_base().description("Proposition environment with italic body"))
        .component("Conjecture", theorem_base().description("Conjecture environment with italic body"))
        .component("Definition", theorem_base().description("Definition environment with upright body"))
        .component("Example", theorem_base().description("Example environment with upright body"))
        .component("Remark", theorem_base().description("Remark environment with upright body"))
        .component("Proof", theorem_base().description("Proof environment with QED symbol appended"))
        // ------------------------------------------------------------------
        // 7.8 Bibliography
        // ------------------------------------------------------------------
        .component(
            "Bibliography",
            ComponentSchema::new()
                .self_closing(true)
                .prop(
                    "style",
                    PropSchema::optional(PropType::String)
                        .with_description("Citation style, e.g. \"apa\", \"ieee\", \"chicago\""),
                )
                .description("Marks the bibliography insertion point"),
        )
        // ------------------------------------------------------------------
        // 7.9 Content Reuse
        // ------------------------------------------------------------------
        .component(
            "Include",
            ComponentSchema::new()
                .self_closing(true)
                .prop("src", PropSchema::required(PropType::String).with_description("Path to the .rdx file to splice in"))
                .description("Splice the AST of another .rdx file at this location"),
        )
        .component(
            "Partial",
            ComponentSchema::new()
                .self_closing(true)
                .prop("src", PropSchema::required(PropType::String).with_description("Path to the source .rdx file"))
                .prop("fragment", PropSchema::required(PropType::String).with_description("Fragment identifier to extract"))
                .description("Splice a labeled subtree from another file"),
        )
        // ------------------------------------------------------------------
        // 7.10 Diagrams
        // ------------------------------------------------------------------
        .component(
            "Diagram",
            ComponentSchema::new()
                .prop(
                    "type",
                    PropSchema::enum_required(vec!["mermaid", "d2", "plantuml", "graphviz"])
                        .with_description("Diagram rendering engine"),
                )
                .prop("id", PropSchema::optional(PropType::String).with_description("Cross-reference label"))
                .prop("caption", PropSchema::optional(PropType::String).with_description("Figure caption"))
                .description("Renders embedded diagram source using the specified engine"),
        )
        // ------------------------------------------------------------------
        // 7.11 API Documentation
        // ------------------------------------------------------------------
        .component(
            "ApiEndpoint",
            ComponentSchema::new()
                .prop("method", PropSchema::required(PropType::String).with_description("HTTP method, e.g. GET, POST"))
                .prop("path", PropSchema::required(PropType::String).with_description("URL path pattern"))
                .description("Documents an API endpoint"),
        )
        // ------------------------------------------------------------------
        // 7.12 Index & Glossary
        // ------------------------------------------------------------------
        .component(
            "Index",
            ComponentSchema::new()
                .prop("term", PropSchema::required(PropType::String).with_description("Primary index term"))
                .prop("sub", PropSchema::optional(PropType::String).with_description("Sub-entry under the primary term"))
                .description("Marks inline text for back-of-book index generation"),
        )
        .component(
            "IndexList",
            ComponentSchema::new()
                .self_closing(true)
                .description("Marks where the generated alphabetical index should appear"),
        )
        .component(
            "Term",
            ComponentSchema::new()
                .prop("id", PropSchema::required(PropType::String).with_description("Unique glossary term identifier"))
                .description("Defines a glossary term inline; collected by <Glossary />"),
        )
        .component(
            "Glossary",
            ComponentSchema::new()
                .self_closing(true)
                .description("Renders an alphabetized list of all <Term> definitions in the document"),
        )
        // ------------------------------------------------------------------
        // 7.13 Translation
        // ------------------------------------------------------------------
        .component(
            "Trans",
            ComponentSchema::new()
                .prop("id", PropSchema::required(PropType::String).with_description("Translation key identifier"))
                .description("Marks translatable content for i18n workflows"),
        )
        .component(
            "ApiParam",
            ComponentSchema::new()
                .prop("name", PropSchema::required(PropType::String).with_description("Parameter name"))
                .prop("type", PropSchema::required(PropType::String).with_description("Parameter type"))
                .prop(
                    "required",
                    PropSchema::optional(PropType::Boolean)
                        .with_default(serde_json::Value::Bool(false))
                        .with_description("Whether the parameter is required"),
                )
                .description("Documents an API parameter"),
        );

    schema
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rdx_parser::parse;

    use crate::validate;

    // Helper: parse and validate, returning diagnostics.
    fn check(src: &str) -> Vec<crate::Diagnostic> {
        let root = parse(src);
        validate(&root, &standard_schema())
    }

    // Helper: parse and validate in strict mode.
    fn check_strict(src: &str) -> Vec<crate::Diagnostic> {
        let root = parse(src);
        let schema = standard_schema().strict(true);
        validate(&root, &schema)
    }

    // ---------------------------------------------------------------------------
    // Basic smoke test: known components produce no errors
    // ---------------------------------------------------------------------------

    #[test]
    fn note_with_no_props_is_valid() {
        let diags = check("<Note>\nSome text.\n</Note>\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    }

    #[test]
    fn note_with_title_and_collapsible_is_valid() {
        let diags = check("<Note title=\"Extra\" collapsible={true}>\nText.\n</Note>\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    }

    #[test]
    fn tip_important_warning_caution_are_valid() {
        for name in ["Tip", "Important", "Warning", "Caution"] {
            let src = format!("<{name}>\nContent.\n</{name}>\n");
            let diags = check(&src);
            assert!(diags.is_empty(), "unexpected diagnostics for <{name}>: {diags:?}");
        }
    }

    #[test]
    fn figure_with_caption_is_valid() {
        let diags = check("<Figure caption=\"My figure\">\n![alt](img.png)\n</Figure>\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    }

    #[test]
    fn figure_with_id_and_caption_is_valid() {
        let diags = check("<Figure id=\"fig:arch\" caption=\"Architecture\">\n![alt](img.png)\n</Figure>\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    }

    #[test]
    fn table_figure_with_caption_is_valid() {
        let diags = check("<TableFigure caption=\"Results\">\n| a | b |\n|---|---|\n| 1 | 2 |\n</TableFigure>\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    }

    #[test]
    fn listing_with_caption_is_valid() {
        let diags = check("<Listing caption=\"Code listing\">\n```rust\nfn main() {}\n```\n</Listing>\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    }

    #[test]
    fn theorem_environments_are_valid() {
        for name in ["Theorem", "Lemma", "Corollary", "Proposition", "Conjecture",
                     "Definition", "Example", "Remark", "Proof"]
        {
            let src = format!("<{name}>\nContent.\n</{name}>\n");
            let diags = check(&src);
            assert!(diags.is_empty(), "unexpected diagnostics for <{name}>: {diags:?}");
        }
    }

    #[test]
    fn bibliography_self_closing_is_valid() {
        let diags = check("<Bibliography />\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    }

    #[test]
    fn bibliography_with_style_is_valid() {
        let diags = check("<Bibliography style=\"apa\" />\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    }

    #[test]
    fn include_with_src_is_valid() {
        let diags = check("<Include src=\"other.rdx\" />\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    }

    #[test]
    fn partial_with_src_and_fragment_is_valid() {
        let diags = check("<Partial src=\"other.rdx\" fragment=\"intro\" />\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    }

    #[test]
    fn diagram_with_type_is_valid() {
        let diags = check("<Diagram type=\"mermaid\">\ngraph LR\n  A --> B\n</Diagram>\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    }

    #[test]
    fn api_endpoint_is_valid() {
        let diags = check("<ApiEndpoint method=\"GET\" path=\"/users\">\nDescription.\n</ApiEndpoint>\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    }

    #[test]
    fn api_param_is_valid() {
        let diags = check("<ApiParam name=\"id\" type=\"string\">\nThe user ID.\n</ApiParam>\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    }

    #[test]
    fn web_only_and_print_only_are_valid() {
        let diags = check("<WebOnly>\nSome web content.\n</WebOnly>\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        let diags = check("<PrintOnly>\nPrint content.\n</PrintOnly>\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    }

    #[test]
    fn page_break_and_column_break_are_valid() {
        let diags = check("<PageBreak />\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        let diags = check("<ColumnBreak />\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    }

    #[test]
    fn spread_abstract_dedication_epigraph_colophon_are_valid() {
        for name in ["Spread", "Abstract", "Dedication", "Colophon"] {
            let src = format!("<{name}>\nContent.\n</{name}>\n");
            let diags = check(&src);
            assert!(diags.is_empty(), "unexpected diagnostics for <{name}>: {diags:?}");
        }
        let diags = check("<Epigraph attribution=\"Author Name\">\nQuote text.\n</Epigraph>\n");
        assert!(diags.is_empty(), "unexpected diagnostics for <Epigraph>: {diags:?}");
    }

    #[test]
    fn steps_and_code_group_are_valid() {
        let diags = check("<Steps>\n## Step 1\nDo this.\n## Step 2\nDo that.\n</Steps>\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        let diags = check("<CodeGroup>\n```rust\nfn main() {}\n```\n</CodeGroup>\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    }

    // ---------------------------------------------------------------------------
    // Strict mode: unknown components are flagged
    // ---------------------------------------------------------------------------

    #[test]
    fn strict_mode_rejects_unknown_component() {
        let diags = check_strict("<MyCustomWidget />\n");
        assert!(
            !diags.is_empty(),
            "expected an error for unknown component in strict mode"
        );
        assert!(
            diags.iter().any(|d| d.message.contains("MyCustomWidget")),
            "error message should mention the component name"
        );
    }

    #[test]
    fn strict_mode_accepts_standard_components() {
        let diags = check_strict("<Note>\nText.\n</Note>\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    }

    // ---------------------------------------------------------------------------
    // Global attributes accepted on any component
    // ---------------------------------------------------------------------------

    #[test]
    fn global_target_attr_on_note_is_valid() {
        let diags = check("<Note target=\"web\">\nContent.\n</Note>\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    }

    #[test]
    fn global_id_attr_on_theorem_is_valid() {
        let diags = check("<Theorem id=\"thm:main\">\nProof content.\n</Theorem>\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    }

    #[test]
    fn global_class_attr_on_figure_is_valid() {
        let diags = check("<Figure caption=\"fig\" class=\"wide\">\n![alt](x.png)\n</Figure>\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    }

    #[test]
    fn global_attrs_on_all_standard_components_are_accepted() {
        for name in ["Note", "Tip", "Warning", "Steps", "Spread", "WebOnly"] {
            let src = format!("<{name} target=\"print\" id=\"ref-1\" class=\"highlight\">\nText.\n</{name}>\n");
            let diags = check(&src);
            assert!(
                diags.is_empty(),
                "unexpected diagnostics for global attrs on <{name}>: {diags:?}"
            );
        }
    }

    // ---------------------------------------------------------------------------
    // Schema inheritance: theorem components accept `id` and `title`
    // ---------------------------------------------------------------------------

    #[test]
    fn theorem_accepts_id_and_title() {
        let diags = check("<Theorem id=\"thm:1\" title=\"Main Result\">\nBody.\n</Theorem>\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    }

    #[test]
    fn lemma_accepts_id_and_title() {
        let diags = check("<Lemma id=\"lem:aux\" title=\"Auxiliary Lemma\">\nBody.\n</Lemma>\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    }

    #[test]
    fn proof_accepts_id_and_title() {
        let diags = check("<Proof id=\"prf:main\" title=\"Proof of Theorem 1\">\nBody.\n</Proof>\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    }

    // ---------------------------------------------------------------------------
    // Required props: missing required props produce errors
    // ---------------------------------------------------------------------------

    #[test]
    fn figure_without_caption_produces_error() {
        let diags = check("<Figure>\n![alt](img.png)\n</Figure>\n");
        let has_caption_error = diags.iter().any(|d| {
            d.message.contains("caption") && d.message.contains("required")
        });
        assert!(has_caption_error, "expected missing caption error, got: {diags:?}");
    }

    #[test]
    fn table_figure_without_caption_produces_error() {
        let diags = check("<TableFigure>\n| a | b |\n</TableFigure>\n");
        let has_caption_error = diags.iter().any(|d| {
            d.message.contains("caption") && d.message.contains("required")
        });
        assert!(has_caption_error, "expected missing caption error, got: {diags:?}");
    }

    #[test]
    fn listing_without_caption_produces_error() {
        let diags = check("<Listing>\n```rust\nfn x() {}\n```\n</Listing>\n");
        let has_caption_error = diags.iter().any(|d| {
            d.message.contains("caption") && d.message.contains("required")
        });
        assert!(has_caption_error, "expected missing caption error, got: {diags:?}");
    }

    #[test]
    fn tab_without_label_produces_error() {
        // Tabs without allowed_children validation context, just check the Tab itself
        let diags = check("<Tab>\nContent.\n</Tab>\n");
        let has_label_error = diags.iter().any(|d| {
            d.message.contains("label") && d.message.contains("required")
        });
        assert!(has_label_error, "expected missing label error, got: {diags:?}");
    }

    #[test]
    fn accordion_item_without_title_produces_error() {
        let diags = check("<AccordionItem>\nContent.\n</AccordionItem>\n");
        let has_title_error = diags.iter().any(|d| {
            d.message.contains("title") && d.message.contains("required")
        });
        assert!(has_title_error, "expected missing title error, got: {diags:?}");
    }

    #[test]
    fn include_without_src_produces_error() {
        let diags = check("<Include />\n");
        let has_src_error = diags.iter().any(|d| {
            d.message.contains("src") && d.message.contains("required")
        });
        assert!(has_src_error, "expected missing src error, got: {diags:?}");
    }

    #[test]
    fn partial_without_fragment_produces_error() {
        let diags = check("<Partial src=\"file.rdx\" />\n");
        let has_fragment_error = diags.iter().any(|d| {
            d.message.contains("fragment") && d.message.contains("required")
        });
        assert!(has_fragment_error, "expected missing fragment error, got: {diags:?}");
    }

    #[test]
    fn diagram_without_type_produces_error() {
        let diags = check("<Diagram>\ngraph LR\n  A --> B\n</Diagram>\n");
        let has_type_error = diags.iter().any(|d| {
            d.message.contains("type") && d.message.contains("required")
        });
        assert!(has_type_error, "expected missing type error, got: {diags:?}");
    }

    #[test]
    fn api_endpoint_without_method_produces_error() {
        let diags = check("<ApiEndpoint path=\"/users\">\nDesc.\n</ApiEndpoint>\n");
        let has_method_error = diags.iter().any(|d| {
            d.message.contains("method") && d.message.contains("required")
        });
        assert!(has_method_error, "expected missing method error, got: {diags:?}");
    }

    // ---------------------------------------------------------------------------
    // Enum validation: invalid enum values produce errors
    // ---------------------------------------------------------------------------

    #[test]
    fn target_invalid_value_produces_error() {
        let diags = check("<Note target=\"invalid\">\nText.\n</Note>\n");
        let has_enum_error = diags.iter().any(|d| {
            d.message.contains("target") && d.message.contains("invalid")
        });
        assert!(has_enum_error, "expected enum error for target=\"invalid\", got: {diags:?}");
    }

    #[test]
    fn target_valid_values_are_accepted() {
        for val in ["web", "print", "all"] {
            let src = format!("<Note target=\"{val}\">\nText.\n</Note>\n");
            let diags = check(&src);
            assert!(
                diags.is_empty(),
                "expected no errors for target=\"{val}\", got: {diags:?}"
            );
        }
    }

    #[test]
    fn diagram_invalid_type_produces_error() {
        let diags = check("<Diagram type=\"tikz\">\nsource\n</Diagram>\n");
        let has_enum_error = diags.iter().any(|d| {
            d.message.contains("type") && d.message.contains("tikz")
        });
        assert!(has_enum_error, "expected enum error for type=\"tikz\", got: {diags:?}");
    }

    #[test]
    fn diagram_valid_types_are_accepted() {
        for val in ["mermaid", "d2", "plantuml", "graphviz"] {
            let src = format!("<Diagram type=\"{val}\">\nsource\n</Diagram>\n");
            let diags = check(&src);
            assert!(
                diags.is_empty(),
                "expected no errors for diagram type=\"{val}\", got: {diags:?}"
            );
        }
    }

    // ---------------------------------------------------------------------------
    // Schema inheritance via ComponentSchema::extends
    // ---------------------------------------------------------------------------

    #[test]
    fn extends_merges_base_props() {
        let base = ComponentSchema::new()
            .prop("id", PropSchema::optional(PropType::String))
            .prop("title", PropSchema::optional(PropType::String));

        // Child overrides title to be required, keeps base id
        let child = ComponentSchema::new()
            .prop("title", PropSchema::required(PropType::String))
            .extends(&base);

        // id comes from base
        assert!(child.props.contains_key("id"), "id should be inherited from base");
        // title is the child's version (required), not base's
        let title_schema = child.props.get("title").expect("title should exist");
        assert!(title_schema.required, "child's required title should take precedence over base's optional");
    }

    #[test]
    fn extends_does_not_override_child_props() {
        let base = ComponentSchema::new()
            .prop("shared", PropSchema::required(PropType::String));

        let child = ComponentSchema::new()
            .prop("shared", PropSchema::optional(PropType::Boolean))
            .extends(&base);

        let shared = child.props.get("shared").expect("shared should exist");
        // Child's optional boolean should win over base's required string
        assert!(!shared.required, "child's prop should take precedence");
        assert_eq!(shared.prop_type, PropType::Boolean, "child's type should be kept");
    }

    // ---------------------------------------------------------------------------
    // Self-closing constraint respected
    // ---------------------------------------------------------------------------

    #[test]
    fn index_with_term_is_valid() {
        let diags = check("<Index term=\"parser\">\nrecursive descent parser\n</Index>\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    }

    #[test]
    fn index_with_sub_is_valid() {
        let diags = check("<Index term=\"parser\" sub=\"recursive descent\">\nparser\n</Index>\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    }

    #[test]
    fn index_without_term_produces_error() {
        let diags = check("<Index>\ntext\n</Index>\n");
        assert!(diags.iter().any(|d| d.message.contains("term") && d.message.contains("required")),
            "expected missing term error, got: {diags:?}");
    }

    #[test]
    fn index_list_self_closing_is_valid() {
        let diags = check("<IndexList />\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    }

    #[test]
    fn glossary_self_closing_is_valid() {
        let diags = check("<Glossary />\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    }

    #[test]
    fn term_with_id_is_valid() {
        let diags = check("<Term id=\"api\">\nApplication Programming Interface\n</Term>\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    }

    #[test]
    fn term_without_id_produces_error() {
        let diags = check("<Term>\nSome term\n</Term>\n");
        assert!(diags.iter().any(|d| d.message.contains("id") && d.message.contains("required")),
            "expected missing id error, got: {diags:?}");
    }

    #[test]
    fn trans_with_id_is_valid() {
        let diags = check("<Trans id=\"greeting\">\nHello, world!\n</Trans>\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    }

    #[test]
    fn page_break_with_children_produces_error() {
        // A PageBreak with children should produce a self-closing error.
        // We need to construct this programmatically since the parser won't
        // produce children inside a self-closing tag from source text.
        use rdx_ast::{ComponentNode, Node, Point, Position, Root, RootType, TextNode};

        let pos = Position {
            start: Point { line: 1, column: 1, offset: 0 },
            end: Point { line: 1, column: 15, offset: 14 },
        };

        let root = Root {
            node_type: RootType::Root,
            frontmatter: None,
            children: vec![Node::Component(ComponentNode {
                name: "PageBreak".into(),
                is_inline: false,
                attributes: vec![],
                children: vec![Node::Text(TextNode {
                    value: "illegal".into(),
                    position: pos.clone(),
                })],
                raw_content: String::new(),
                position: pos,
            })],
            position: Position {
                start: Point { line: 1, column: 1, offset: 0 },
                end: Point { line: 2, column: 1, offset: 15 },
            },
        };

        let schema = standard_schema();
        let diags = crate::validate(&root, &schema);
        let has_self_closing_error = diags.iter().any(|d| {
            d.message.contains("self-closing")
        });
        assert!(has_self_closing_error, "expected self-closing error for PageBreak with children, got: {diags:?}");
    }
}
