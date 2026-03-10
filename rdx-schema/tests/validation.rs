use rdx_parser::parse;
use rdx_schema::{ComponentSchema, Diagnostic, PropSchema, PropType, Schema, Severity, validate};

fn schema() -> Schema {
    Schema::new()
        .strict(true)
        .component(
            "Notice",
            ComponentSchema::new()
                .prop(
                    "type",
                    PropSchema::enum_required(vec!["info", "warning", "error"]),
                )
                .prop("title", PropSchema::optional(PropType::String)),
        )
        .component(
            "Badge",
            ComponentSchema::new()
                .self_closing(true)
                .prop("label", PropSchema::required(PropType::String))
                .prop("color", PropSchema::optional(PropType::String)),
        )
        .component(
            "Chart",
            ComponentSchema::new()
                .prop("config", PropSchema::required(PropType::Object))
                .prop("height", PropSchema::optional(PropType::Number)),
        )
        .component("Tabs", ComponentSchema::new().allowed_children(vec!["Tab"]))
        .component("Tab", ComponentSchema::new())
}

fn errors(diagnostics: &[Diagnostic]) -> Vec<&Diagnostic> {
    diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect()
}

fn warnings(diagnostics: &[Diagnostic]) -> Vec<&Diagnostic> {
    diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .collect()
}

#[test]
fn valid_document_produces_no_diagnostics() {
    let root = parse("<Notice type=\"info\">\nHello world.\n</Notice>\n");
    let diags = validate(&root, &schema());
    assert!(diags.is_empty(), "expected no diagnostics, got: {diags:?}");
}

#[test]
fn missing_required_prop() {
    let root = parse("<Notice>\nText.\n</Notice>\n");
    let diags = validate(&root, &schema());
    let errs = errors(&diags);
    assert_eq!(errs.len(), 1);
    assert!(errs[0].message.contains("missing required prop `type`"));
}

#[test]
fn wrong_type_prop() {
    let root = parse("<Chart config=\"not an object\" />\n");
    let diags = validate(&root, &schema());
    let errs = errors(&diags);
    assert_eq!(errs.len(), 1);
    assert!(errs[0].message.contains("expects object, got string"));
}

#[test]
fn invalid_enum_value() {
    let root = parse("<Notice type=\"danger\">\nText.\n</Notice>\n");
    let diags = validate(&root, &schema());
    let errs = errors(&diags);
    assert_eq!(errs.len(), 1);
    assert!(errs[0].message.contains("must be one of"));
    assert!(errs[0].message.contains("danger"));
}

#[test]
fn valid_enum_value() {
    let root = parse("<Notice type=\"warning\">\nText.\n</Notice>\n");
    let diags = validate(&root, &schema());
    assert!(diags.is_empty());
}

#[test]
fn unknown_component_strict() {
    let root = parse("<Foo />\n");
    let diags = validate(&root, &schema());
    let errs = errors(&diags);
    assert_eq!(errs.len(), 1);
    assert!(errs[0].message.contains("unknown component <Foo>"));
}

#[test]
fn unknown_component_non_strict() {
    let s = Schema::new().component("Notice", ComponentSchema::new());
    let root = parse("<Foo />\n");
    let diags = validate(&root, &s);
    assert!(diags.is_empty());
}

#[test]
fn unknown_prop_warns() {
    let root = parse("<Notice type=\"info\" size={42}>\nText.\n</Notice>\n");
    let diags = validate(&root, &schema());
    let warns = warnings(&diags);
    assert_eq!(warns.len(), 1);
    assert!(warns[0].message.contains("unknown prop `size`"));
}

#[test]
fn self_closing_with_children() {
    let root = parse("<Badge label=\"new\">\nChild text.\n</Badge>\n");
    let diags = validate(&root, &schema());
    let errs = errors(&diags);
    assert!(
        errs.iter()
            .any(|e| e.message.contains("must be self-closing")),
        "expected self-closing error, got: {errs:?}"
    );
}

#[test]
fn allowed_children_violation() {
    let root = parse("<Tabs>\n\n<Badge label=\"x\" />\n\n</Tabs>\n");
    let diags = validate(&root, &schema());
    let errs = errors(&diags);
    assert!(
        errs.iter()
            .any(|e| e.message.contains("not allowed as a child")),
        "expected child violation, got: {errs:?}"
    );
}

#[test]
fn allowed_children_valid() {
    let root = parse("<Tabs>\n\n<Tab>\nContent.\n</Tab>\n\n</Tabs>\n");
    let diags = validate(&root, &schema());
    let errs = errors(&diags);
    assert!(errs.is_empty(), "expected no errors, got: {errs:?}");
}

#[test]
fn variable_attribute_accepted_for_any_type() {
    let root = parse("<Chart config={$data.chart} />\n");
    let diags = validate(&root, &schema());
    let errs = errors(&diags);
    // Variable should be accepted — resolved at runtime
    assert!(errs.is_empty(), "expected no errors, got: {errs:?}");
}

#[test]
fn multiple_errors_collected() {
    // Missing required `label`, unknown component, wrong type
    let root = parse("<Badge />\n\n<Foo />\n\n<Chart config=\"bad\" />\n");
    let diags = validate(&root, &schema());
    let errs = errors(&diags);
    assert!(errs.len() >= 3, "expected at least 3 errors, got: {errs:?}");
}

#[test]
fn schema_serializes_to_json() {
    let s = schema();
    let json = serde_json::to_string_pretty(&s).unwrap();
    let deserialized: Schema = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.strict, true);
    assert!(deserialized.components.contains_key("Notice"));
    assert!(deserialized.components.contains_key("Badge"));
}

#[test]
fn nested_component_validation() {
    // Notice contains a Badge — Badge is validated even though nested
    let root = parse("<Notice type=\"info\">\n\n<Badge label=\"new\" />\n\n</Notice>\n");
    let diags = validate(&root, &schema());
    assert!(diags.is_empty(), "expected no diagnostics, got: {diags:?}");
}

#[test]
fn diagnostic_display() {
    let d = Diagnostic {
        severity: Severity::Error,
        message: "missing required prop `type`".to_string(),
        component: "Notice".to_string(),
        line: 1,
        column: 1,
    };
    let s = format!("{d}");
    assert_eq!(s, "Notice:1:1: error: missing required prop `type`");
}
