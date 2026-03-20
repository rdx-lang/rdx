use rkyv::Archive;
use serde::{Deserialize, Serialize};

/// rkyv wrapper that stores serde_json types as their JSON string representation.
mod rkyv_json {
    use rkyv::rancor::Fallible;
    use rkyv::string::ArchivedString;
    use rkyv::with::{ArchiveWith, DeserializeWith, SerializeWith};
    use rkyv::{Archive, Place};

    pub struct AsJsonString;

    impl<T: serde::Serialize> ArchiveWith<T> for AsJsonString {
        type Archived = ArchivedString;
        type Resolver = <String as Archive>::Resolver;

        fn resolve_with(field: &T, resolver: Self::Resolver, out: Place<Self::Archived>) {
            // Safety: serde_json::to_string cannot fail for types that already implement Serialize
            // (no IO, no map-key errors). An unwrap is appropriate here.
            let json = serde_json::to_string(field).expect("serde_json::to_string failed");
            ArchivedString::resolve_from_str(&json, resolver, out);
        }
    }

    impl<T: serde::Serialize, S: Fallible<Error: rkyv::rancor::Source> + rkyv::ser::Writer + ?Sized>
        SerializeWith<T, S> for AsJsonString
    {
        fn serialize_with(field: &T, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
            // Safety: see resolve_with — serialization of valid Serialize types cannot fail.
            let json = serde_json::to_string(field).expect("serde_json::to_string failed");
            ArchivedString::serialize_from_str(&json, serializer)
        }
    }

    impl<T: serde::de::DeserializeOwned, D: Fallible + ?Sized> DeserializeWith<ArchivedString, T, D>
        for AsJsonString
    {
        fn deserialize_with(archived: &ArchivedString, _: &mut D) -> Result<T, D::Error> {
            // Safety: the archived string was produced by to_string above, so from_str cannot fail.
            Ok(serde_json::from_str(archived.as_str()).expect("serde_json::from_str failed"))
        }
    }
}

/// Positional data mapping an AST node back to its source `.rdx` file.
/// Line and column numbers are 1-indexed. Offsets are 0-indexed byte offsets.
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct Position {
    pub start: Point,
    pub end: Point,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct Point {
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

/// The root of an RDX document.
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[rkyv(serialize_bounds(
    __S: rkyv::ser::Writer + rkyv::ser::Allocator + rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
))]
#[rkyv(deserialize_bounds(
    __D: rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
))]
#[rkyv(bytecheck(bounds(
    __C: rkyv::validation::ArchiveContext + rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
)))]
pub struct Root {
    #[serde(rename = "type")]
    pub node_type: RootType,
    #[rkyv(with = rkyv::with::Map<rkyv_json::AsJsonString>)]
    pub frontmatter: Option<serde_json::Value>,
    #[rkyv(omit_bounds)]
    pub children: Vec<Node>,
    pub position: Position,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub enum RootType {
    #[serde(rename = "root")]
    Root,
}

/// A union of all possible RDX nodes.
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[rkyv(serialize_bounds(
    __S: rkyv::ser::Writer + rkyv::ser::Allocator + rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
))]
#[rkyv(deserialize_bounds(
    __D: rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
))]
#[rkyv(bytecheck(bounds(
    __C: rkyv::validation::ArchiveContext + rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
)))]
#[serde(tag = "type")]
pub enum Node {
    #[serde(rename = "text")]
    Text(#[rkyv(omit_bounds)] TextNode),
    #[serde(rename = "code_inline")]
    CodeInline(#[rkyv(omit_bounds)] CodeInlineNode),
    #[serde(rename = "code_block")]
    CodeBlock(#[rkyv(omit_bounds)] CodeBlockNode),
    #[serde(rename = "paragraph")]
    Paragraph(#[rkyv(omit_bounds)] StandardBlockNode),
    #[serde(rename = "heading")]
    Heading(#[rkyv(omit_bounds)] StandardBlockNode),
    #[serde(rename = "list")]
    List(#[rkyv(omit_bounds)] StandardBlockNode),
    #[serde(rename = "list_item")]
    ListItem(#[rkyv(omit_bounds)] StandardBlockNode),
    #[serde(rename = "blockquote")]
    Blockquote(#[rkyv(omit_bounds)] StandardBlockNode),
    #[serde(rename = "thematic_break")]
    ThematicBreak(#[rkyv(omit_bounds)] StandardBlockNode),
    #[serde(rename = "html")]
    Html(#[rkyv(omit_bounds)] StandardBlockNode),
    #[serde(rename = "table")]
    Table(#[rkyv(omit_bounds)] StandardBlockNode),
    #[serde(rename = "table_row")]
    TableRow(#[rkyv(omit_bounds)] StandardBlockNode),
    #[serde(rename = "table_cell")]
    TableCell(#[rkyv(omit_bounds)] StandardBlockNode),
    #[serde(rename = "link")]
    Link(#[rkyv(omit_bounds)] LinkNode),
    #[serde(rename = "image")]
    Image(#[rkyv(omit_bounds)] ImageNode),
    #[serde(rename = "emphasis")]
    Emphasis(#[rkyv(omit_bounds)] StandardBlockNode),
    #[serde(rename = "strong")]
    Strong(#[rkyv(omit_bounds)] StandardBlockNode),
    #[serde(rename = "strikethrough")]
    Strikethrough(#[rkyv(omit_bounds)] StandardBlockNode),
    #[serde(rename = "definition_list")]
    DefinitionList(#[rkyv(omit_bounds)] StandardBlockNode),
    #[serde(rename = "definition_term")]
    DefinitionTerm(#[rkyv(omit_bounds)] StandardBlockNode),
    #[serde(rename = "definition_description")]
    DefinitionDescription(#[rkyv(omit_bounds)] StandardBlockNode),
    #[serde(rename = "footnote_definition")]
    FootnoteDefinition(#[rkyv(omit_bounds)] FootnoteNode),
    #[serde(rename = "footnote_reference")]
    FootnoteReference(#[rkyv(omit_bounds)] FootnoteNode),
    #[serde(rename = "math_inline")]
    MathInline(#[rkyv(omit_bounds)] MathNode),
    #[serde(rename = "math_display")]
    MathDisplay(#[rkyv(omit_bounds)] MathDisplayNode),
    #[serde(rename = "citation")]
    Citation(#[rkyv(omit_bounds)] CitationNode),
    #[serde(rename = "cross_ref")]
    CrossRef(#[rkyv(omit_bounds)] CrossRefNode),
    #[serde(rename = "component")]
    Component(#[rkyv(omit_bounds)] ComponentNode),
    #[serde(rename = "variable")]
    Variable(#[rkyv(omit_bounds)] VariableNode),
    #[serde(rename = "error")]
    Error(#[rkyv(omit_bounds)] ErrorNode),
}

/// A standard CommonMark block node.
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[rkyv(serialize_bounds(
    __S: rkyv::ser::Writer + rkyv::ser::Allocator + rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
))]
#[rkyv(deserialize_bounds(
    __D: rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
))]
#[rkyv(bytecheck(bounds(
    __C: rkyv::validation::ArchiveContext + rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
)))]
pub struct StandardBlockNode {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ordered: Option<bool>,
    /// For list items: whether a task list checkbox is checked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
    /// For headings: an explicit ID attribute (`# Title {#my-id}`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[rkyv(omit_bounds)]
    pub children: Vec<Node>,
    pub position: Position,
}

/// An RDX component node.
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[rkyv(serialize_bounds(
    __S: rkyv::ser::Writer + rkyv::ser::Allocator + rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
))]
#[rkyv(deserialize_bounds(
    __D: rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
))]
#[rkyv(bytecheck(bounds(
    __C: rkyv::validation::ArchiveContext + rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
)))]
pub struct ComponentNode {
    pub name: String,
    #[serde(rename = "isInline")]
    pub is_inline: bool,
    #[rkyv(omit_bounds)]
    pub attributes: Vec<AttributeNode>,
    #[rkyv(omit_bounds)]
    pub children: Vec<Node>,
    /// Raw source text of the component body (between open/close tags).
    /// Preserved verbatim for components that need whitespace-sensitive content
    /// (e.g. CodeBlock). Empty for self-closing components.
    #[serde(
        default,
        rename = "rawContent",
        skip_serializing_if = "String::is_empty"
    )]
    pub raw_content: String,
    pub position: Position,
}

/// A single attribute with its own positional data.
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[rkyv(serialize_bounds(
    __S: rkyv::ser::Writer + rkyv::ser::Allocator + rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
))]
#[rkyv(deserialize_bounds(
    __D: rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
))]
#[rkyv(bytecheck(bounds(
    __C: rkyv::validation::ArchiveContext + rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
)))]
pub struct AttributeNode {
    pub name: String,
    #[rkyv(omit_bounds)]
    pub value: AttributeValue,
    pub position: Position,
}

/// Supported attribute value types.
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[rkyv(serialize_bounds(
    __S: rkyv::ser::Writer + rkyv::ser::Allocator + rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
))]
#[rkyv(deserialize_bounds(
    __D: rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
))]
#[rkyv(bytecheck(bounds(
    __C: rkyv::validation::ArchiveContext + rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
)))]
#[serde(untagged)]
pub enum AttributeValue {
    Null,
    Bool(bool),
    Number(#[rkyv(with = rkyv_json::AsJsonString)] serde_json::Number),
    String(String),
    Array(#[rkyv(with = rkyv_json::AsJsonString)] Vec<serde_json::Value>),
    Object(#[rkyv(with = rkyv_json::AsJsonString)] serde_json::Map<String, serde_json::Value>),
    Variable(#[rkyv(omit_bounds)] VariableNode),
}

/// A footnote node (definition or reference).
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[rkyv(serialize_bounds(
    __S: rkyv::ser::Writer + rkyv::ser::Allocator + rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
))]
#[rkyv(deserialize_bounds(
    __D: rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
))]
#[rkyv(bytecheck(bounds(
    __C: rkyv::validation::ArchiveContext + rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
)))]
pub struct FootnoteNode {
    pub label: String,
    #[rkyv(omit_bounds)]
    pub children: Vec<Node>,
    pub position: Position,
}

/// A link node with URL and optional title.
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[rkyv(serialize_bounds(
    __S: rkyv::ser::Writer + rkyv::ser::Allocator + rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
))]
#[rkyv(deserialize_bounds(
    __D: rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
))]
#[rkyv(bytecheck(bounds(
    __C: rkyv::validation::ArchiveContext + rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
)))]
pub struct LinkNode {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[rkyv(omit_bounds)]
    pub children: Vec<Node>,
    pub position: Position,
}

/// An image node with URL, optional title, and alt text.
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[rkyv(serialize_bounds(
    __S: rkyv::ser::Writer + rkyv::ser::Allocator + rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
))]
#[rkyv(deserialize_bounds(
    __D: rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
))]
#[rkyv(bytecheck(bounds(
    __C: rkyv::validation::ArchiveContext + rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
)))]
pub struct ImageNode {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
    #[rkyv(omit_bounds)]
    pub children: Vec<Node>,
    pub position: Position,
}

/// A fenced code block with optional language, meta string, and display metadata.
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct CodeBlockNode {
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<String>,
    /// Display title or filename (from info string `title="..."`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Sorted, deduplicated line numbers to emphasize.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight: Option<Vec<u32>>,
    /// Whether to display line numbers.
    #[serde(rename = "showLineNumbers", skip_serializing_if = "Option::is_none")]
    pub show_line_numbers: Option<bool>,
    /// Whether to render as a unified diff.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<bool>,
    /// Caption text for numbered code listings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    pub position: Position,
}

/// A literal text node.
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct TextNode {
    pub value: String,
    pub position: Position,
}

/// An inline code node with optional language hint.
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct CodeInlineNode {
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    pub position: Position,
}

/// A variable interpolation node.
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct VariableNode {
    pub path: String,
    pub position: Position,
}

/// An explicit error node for host-level error boundaries.
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct ErrorNode {
    pub message: String,
    #[serde(rename = "rawContent")]
    pub raw_content: String,
    pub position: Position,
}

/// A citation reference node containing one or more citation keys.
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct CitationNode {
    pub keys: Vec<CitationKey>,
    pub position: Position,
}

/// A single citation key with optional prefix and locator.
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct CitationKey {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locator: Option<String>,
}

/// A cross-reference node pointing to a labeled element elsewhere in the document.
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct CrossRefNode {
    pub target: String,
    pub position: Position,
}

/// An inline math expression node containing raw LaTeX and a structured parse tree.
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[rkyv(serialize_bounds(
    __S: rkyv::ser::Writer + rkyv::ser::Allocator + rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
))]
#[rkyv(deserialize_bounds(
    __D: rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
))]
#[rkyv(bytecheck(bounds(
    __C: rkyv::validation::ArchiveContext + rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
)))]
pub struct MathNode {
    pub raw: String,
    #[rkyv(omit_bounds)]
    pub tree: MathExpr,
    pub position: Position,
}

/// A display math expression node containing raw LaTeX, a structured parse tree, and an optional label.
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[rkyv(serialize_bounds(
    __S: rkyv::ser::Writer + rkyv::ser::Allocator + rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
))]
#[rkyv(deserialize_bounds(
    __D: rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
))]
#[rkyv(bytecheck(bounds(
    __C: rkyv::validation::ArchiveContext + rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
)))]
pub struct MathDisplayNode {
    pub raw: String,
    #[rkyv(omit_bounds)]
    pub tree: MathExpr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub position: Position,
}

// ---------------------------------------------------------------------------
// Math expression supporting types
// ---------------------------------------------------------------------------

/// A mathematical operator symbol with its classification.
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct MathOperator {
    pub symbol: String,
    pub kind: OperatorKind,
}

/// Classification of a mathematical operator.
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum OperatorKind {
    Binary,
    Relation,
    Prefix,
    Postfix,
    Large,
    Punctuation,
}

/// A delimiter character used in fenced expressions.
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Delimiter {
    Paren,
    Bracket,
    Brace,
    Angle,
    Pipe,
    DoublePipe,
    Floor,
    Ceil,
    None,
}

/// Style for fraction rendering.
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum FracStyle {
    Display,
    Text,
    Auto,
}

/// Limit placement style for big operators.
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum LimitStyle {
    DisplayLimits,
    Limits,
    NoLimits,
}

/// Delimiter style for matrix environments.
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum MatrixDelimiters {
    Plain,
    Paren,
    Bracket,
    Brace,
    Pipe,
    DoublePipe,
}

/// Column alignment specifier for array environments.
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ColumnAlign {
    Left,
    Center,
    Right,
}

/// Named math spacing amounts.
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum MathSpace {
    Thin,
    Medium,
    Thick,
    Quad,
    QQuad,
    NegThin,
    Custom(String),
}

/// Smash mode indicating which dimension to suppress.
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SmashMode {
    Top,
    Bottom,
    Both,
}

/// Math style (display size) override.
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum MathStyle {
    Display,
    Text,
    Script,
    ScriptScript,
}

/// Math font override.
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum MathFont {
    Roman,
    Bold,
    Italic,
    BoldItalic,
    SansSerif,
    Monospace,
    Blackboard,
    Calligraphic,
    Fraktur,
    Script,
}

/// Accent mark kind.
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum AccentKind {
    Hat,
    Tilde,
    Vec,
    Dot,
    Ddot,
    Bar,
    Acute,
    Grave,
    Breve,
    Check,
    WideHat,
    WideTilde,
}

/// A row in an alignment environment (align/alignat).
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[rkyv(serialize_bounds(
    __S: rkyv::ser::Writer + rkyv::ser::Allocator + rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
))]
#[rkyv(deserialize_bounds(
    __D: rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
))]
#[rkyv(bytecheck(bounds(
    __C: rkyv::validation::ArchiveContext + rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
)))]
pub struct AlignRow {
    #[rkyv(omit_bounds)]
    pub cells: Vec<MathExpr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// A row in a cases environment.
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[rkyv(serialize_bounds(
    __S: rkyv::ser::Writer + rkyv::ser::Allocator + rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
))]
#[rkyv(deserialize_bounds(
    __D: rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
))]
#[rkyv(bytecheck(bounds(
    __C: rkyv::validation::ArchiveContext + rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
)))]
pub struct CaseRow {
    #[rkyv(omit_bounds)]
    pub expr: MathExpr,
    #[rkyv(omit_bounds)]
    pub condition: Option<MathExpr>,
}

/// Structured math expression tree (Section 2.11).
///
/// This is a recursive algebraic type representing the parsed structure of a LaTeX
/// math expression. `Box<MathExpr>` is used for recursive fields to keep the type
/// sized.
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[rkyv(serialize_bounds(
    __S: rkyv::ser::Writer + rkyv::ser::Allocator + rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
))]
#[rkyv(deserialize_bounds(
    __D: rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
))]
#[rkyv(bytecheck(bounds(
    __C: rkyv::validation::ArchiveContext + rkyv::rancor::Fallible<Error: rkyv::rancor::Source>,
)))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MathExpr {
    // Atoms
    Ident {
        value: String,
    },
    Number {
        value: String,
    },
    Operator(MathOperator),
    Text {
        value: String,
    },

    // Grouping
    Row {
        #[rkyv(omit_bounds)]
        children: Vec<MathExpr>,
    },
    Fenced {
        open: Delimiter,
        close: Delimiter,
        #[rkyv(omit_bounds)]
        body: Vec<MathExpr>,
    },

    // Scripts
    Superscript {
        #[rkyv(omit_bounds)]
        base: Box<MathExpr>,
        #[rkyv(omit_bounds)]
        script: Box<MathExpr>,
    },
    Subscript {
        #[rkyv(omit_bounds)]
        base: Box<MathExpr>,
        #[rkyv(omit_bounds)]
        script: Box<MathExpr>,
    },
    Subsuperscript {
        #[rkyv(omit_bounds)]
        base: Box<MathExpr>,
        #[rkyv(omit_bounds)]
        sub: Box<MathExpr>,
        #[rkyv(omit_bounds)]
        sup: Box<MathExpr>,
    },

    // Layout
    Frac {
        #[rkyv(omit_bounds)]
        numerator: Box<MathExpr>,
        #[rkyv(omit_bounds)]
        denominator: Box<MathExpr>,
        style: FracStyle,
    },
    Sqrt {
        #[rkyv(omit_bounds)]
        index: Option<Box<MathExpr>>,
        #[rkyv(omit_bounds)]
        body: Box<MathExpr>,
    },

    // Over/Under
    Overline {
        #[rkyv(omit_bounds)]
        body: Box<MathExpr>,
    },
    Underline {
        #[rkyv(omit_bounds)]
        body: Box<MathExpr>,
    },
    Overbrace {
        #[rkyv(omit_bounds)]
        body: Box<MathExpr>,
        #[rkyv(omit_bounds)]
        annotation: Option<Box<MathExpr>>,
    },
    Underbrace {
        #[rkyv(omit_bounds)]
        body: Box<MathExpr>,
        #[rkyv(omit_bounds)]
        annotation: Option<Box<MathExpr>>,
    },
    Overset {
        #[rkyv(omit_bounds)]
        over: Box<MathExpr>,
        #[rkyv(omit_bounds)]
        base: Box<MathExpr>,
    },
    Underset {
        #[rkyv(omit_bounds)]
        under: Box<MathExpr>,
        #[rkyv(omit_bounds)]
        base: Box<MathExpr>,
    },
    Accent {
        kind: AccentKind,
        #[rkyv(omit_bounds)]
        body: Box<MathExpr>,
    },

    // Big operators
    BigOperator {
        op: MathOperator,
        limits: LimitStyle,
        #[rkyv(omit_bounds)]
        lower: Option<Box<MathExpr>>,
        #[rkyv(omit_bounds)]
        upper: Option<Box<MathExpr>>,
    },

    // Matrices & arrays
    Matrix {
        #[rkyv(omit_bounds)]
        rows: Vec<Vec<MathExpr>>,
        delimiters: MatrixDelimiters,
    },
    Cases {
        #[rkyv(omit_bounds)]
        rows: Vec<CaseRow>,
    },
    Array {
        columns: Vec<ColumnAlign>,
        #[rkyv(omit_bounds)]
        rows: Vec<Vec<MathExpr>>,
    },

    // Alignment environments
    Align {
        #[rkyv(omit_bounds)]
        rows: Vec<AlignRow>,
        numbered: bool,
    },
    Gather {
        #[rkyv(omit_bounds)]
        rows: Vec<MathExpr>,
        numbered: bool,
    },

    // Spacing
    Space(MathSpace),
    Phantom {
        #[rkyv(omit_bounds)]
        body: Box<MathExpr>,
    },
    HPhantom {
        #[rkyv(omit_bounds)]
        body: Box<MathExpr>,
    },
    VPhantom {
        #[rkyv(omit_bounds)]
        body: Box<MathExpr>,
    },
    Smash {
        #[rkyv(omit_bounds)]
        body: Box<MathExpr>,
        mode: SmashMode,
    },

    // Style overrides
    StyleOverride {
        style: MathStyle,
        #[rkyv(omit_bounds)]
        body: Box<MathExpr>,
    },
    FontOverride {
        font: MathFont,
        #[rkyv(omit_bounds)]
        body: Box<MathExpr>,
    },
    Color {
        color: String,
        #[rkyv(omit_bounds)]
        body: Box<MathExpr>,
    },

    // Chemistry
    Chem {
        value: String,
    },

    // Error recovery
    Error {
        raw: String,
        message: String,
    },
}

impl Node {
    /// Returns a mutable reference to this node's children, if it has any.
    pub fn children_mut(&mut self) -> Option<&mut Vec<Node>> {
        match self {
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
            | Node::ThematicBreak(b)
            | Node::DefinitionList(b)
            | Node::DefinitionTerm(b)
            | Node::DefinitionDescription(b) => Some(&mut b.children),
            Node::Link(l) => Some(&mut l.children),
            Node::Image(i) => Some(&mut i.children),
            Node::Component(c) => Some(&mut c.children),
            Node::FootnoteDefinition(n) => Some(&mut n.children),
            _ => None,
        }
    }

    /// Returns a reference to this node's children, if it has any.
    pub fn children(&self) -> Option<&[Node]> {
        match self {
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
            | Node::ThematicBreak(b)
            | Node::DefinitionList(b)
            | Node::DefinitionTerm(b)
            | Node::DefinitionDescription(b) => Some(&b.children),
            Node::Link(l) => Some(&l.children),
            Node::Image(i) => Some(&i.children),
            Node::Component(c) => Some(&c.children),
            Node::FootnoteDefinition(n) => Some(&n.children),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(line: usize, col: usize, off: usize) -> Point {
        Point {
            line,
            column: col,
            offset: off,
        }
    }

    fn span(sl: usize, sc: usize, so: usize, el: usize, ec: usize, eo: usize) -> Position {
        Position {
            start: pos(sl, sc, so),
            end: pos(el, ec, eo),
        }
    }

    #[test]
    fn root_serializes_type_field() {
        let root = Root {
            node_type: RootType::Root,
            frontmatter: None,
            children: vec![],
            position: span(1, 1, 0, 1, 1, 0),
        };
        let json = serde_json::to_value(&root).unwrap();
        assert_eq!(json["type"], "root");
        assert!(json["frontmatter"].is_null());
        assert_eq!(json["children"], serde_json::json!([]));
    }

    #[test]
    fn component_node_serializes_correctly() {
        let node = Node::Component(ComponentNode {
            name: "Badge".into(),
            is_inline: false,
            attributes: vec![
                AttributeNode {
                    name: "status".into(),
                    value: AttributeValue::String("beta".into()),
                    position: span(1, 8, 7, 1, 22, 21),
                },
                AttributeNode {
                    name: "active".into(),
                    value: AttributeValue::Bool(true),
                    position: span(1, 23, 22, 1, 36, 35),
                },
            ],
            children: vec![Node::Text(TextNode {
                value: "New Feature".into(),
                position: span(1, 37, 36, 1, 48, 47),
            })],
            raw_content: String::new(),
            position: span(1, 1, 0, 1, 55, 54),
        });

        let json = serde_json::to_value(&node).unwrap();
        assert_eq!(json["type"], "component");
        assert_eq!(json["name"], "Badge");
        assert_eq!(json["isInline"], false);
        assert_eq!(json["attributes"][0]["name"], "status");
        assert_eq!(json["attributes"][0]["value"], "beta");
        assert_eq!(json["attributes"][1]["name"], "active");
        assert_eq!(json["attributes"][1]["value"], true);
        assert_eq!(json["children"][0]["type"], "text");
        assert_eq!(json["children"][0]["value"], "New Feature");
    }

    #[test]
    fn attribute_value_null_serializes_to_null() {
        let val = AttributeValue::Null;
        let json = serde_json::to_value(&val).unwrap();
        assert!(json.is_null());
    }

    #[test]
    fn attribute_value_number() {
        let val = AttributeValue::Number(serde_json::Number::from(42));
        let json = serde_json::to_value(&val).unwrap();
        assert_eq!(json, 42);
    }

    #[test]
    fn attribute_value_json_object() {
        let mut map = serde_json::Map::new();
        map.insert("type".into(), serde_json::Value::String("bar".into()));
        let val = AttributeValue::Object(map);
        let json = serde_json::to_value(&val).unwrap();
        assert_eq!(json["type"], "bar");
    }

    #[test]
    fn attribute_value_json_array() {
        let val = AttributeValue::Array(vec![
            serde_json::Value::from(10),
            serde_json::Value::from(20),
        ]);
        let json = serde_json::to_value(&val).unwrap();
        assert_eq!(json, serde_json::json!([10, 20]));
    }

    #[test]
    fn variable_node_serializes() {
        let node = Node::Variable(VariableNode {
            path: "frontmatter.title".into(),
            position: span(1, 1, 0, 1, 20, 19),
        });
        let json = serde_json::to_value(&node).unwrap();
        assert_eq!(json["type"], "variable");
        assert_eq!(json["path"], "frontmatter.title");
    }

    #[test]
    fn error_node_serializes() {
        let node = Node::Error(ErrorNode {
            message: "Unclosed tag".into(),
            raw_content: "<Notice>".into(),
            position: span(1, 1, 0, 1, 9, 8),
        });
        let json = serde_json::to_value(&node).unwrap();
        assert_eq!(json["type"], "error");
        assert_eq!(json["message"], "Unclosed tag");
        assert_eq!(json["rawContent"], "<Notice>");
    }

    #[test]
    fn standard_block_omits_none_fields() {
        let node = Node::Heading(StandardBlockNode {
            depth: Some(2),
            ordered: None,
            checked: None,
            id: None,
            children: vec![],
            position: span(1, 1, 0, 1, 10, 9),
        });
        let json = serde_json::to_value(&node).unwrap();
        assert_eq!(json["depth"], 2);
        assert!(json.get("ordered").is_none());
        assert!(json.get("checked").is_none());
        assert!(json.get("id").is_none());
    }

    #[test]
    fn roundtrip_component_node() {
        let original = Node::Component(ComponentNode {
            name: "Chart".into(),
            is_inline: true,
            attributes: vec![],
            children: vec![],
            raw_content: String::new(),
            position: span(1, 1, 0, 1, 10, 9),
        });
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: Node = serde_json::from_str(&serialized).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn code_inline_node_serializes() {
        let node = Node::CodeInline(CodeInlineNode {
            value: "x + 1".into(),
            lang: Some("rust".into()),
            position: span(1, 1, 0, 1, 10, 9),
        });
        let json = serde_json::to_value(&node).unwrap();
        assert_eq!(json["type"], "code_inline");
        assert_eq!(json["value"], "x + 1");
        assert_eq!(json["lang"], "rust");
    }

    #[test]
    fn code_inline_node_omits_lang_when_none() {
        let node = Node::CodeInline(CodeInlineNode {
            value: "foo()".into(),
            lang: None,
            position: span(1, 1, 0, 1, 7, 6),
        });
        let json = serde_json::to_value(&node).unwrap();
        assert_eq!(json["type"], "code_inline");
        assert!(json.get("lang").is_none());
    }

    #[test]
    fn code_block_node_new_fields_serialize() {
        let node = Node::CodeBlock(CodeBlockNode {
            value: "fn main() {}".into(),
            lang: Some("rust".into()),
            meta: None,
            title: Some("main.rs".into()),
            highlight: Some(vec![1, 3]),
            show_line_numbers: Some(true),
            diff: None,
            caption: None,
            position: span(1, 1, 0, 3, 1, 20),
        });
        let json = serde_json::to_value(&node).unwrap();
        assert_eq!(json["type"], "code_block");
        assert_eq!(json["title"], "main.rs");
        assert_eq!(json["highlight"], serde_json::json!([1, 3]));
        assert_eq!(json["showLineNumbers"], true);
        assert!(json.get("diff").is_none());
        assert!(json.get("caption").is_none());
    }

    #[test]
    fn citation_node_serializes() {
        let node = Node::Citation(CitationNode {
            keys: vec![
                CitationKey {
                    id: "smith2024".into(),
                    prefix: Some("see ".into()),
                    locator: Some("p. 42".into()),
                },
                CitationKey {
                    id: "jones2023".into(),
                    prefix: None,
                    locator: None,
                },
            ],
            position: span(1, 1, 0, 1, 20, 19),
        });
        let json = serde_json::to_value(&node).unwrap();
        assert_eq!(json["type"], "citation");
        assert_eq!(json["keys"][0]["id"], "smith2024");
        assert_eq!(json["keys"][0]["prefix"], "see ");
        assert_eq!(json["keys"][0]["locator"], "p. 42");
        assert_eq!(json["keys"][1]["id"], "jones2023");
        assert!(json["keys"][1].get("prefix").is_none());
        assert!(json["keys"][1].get("locator").is_none());
    }

    #[test]
    fn cross_ref_node_serializes() {
        let node = Node::CrossRef(CrossRefNode {
            target: "fig:architecture".into(),
            position: span(1, 1, 0, 1, 20, 19),
        });
        let json = serde_json::to_value(&node).unwrap();
        assert_eq!(json["type"], "cross_ref");
        assert_eq!(json["target"], "fig:architecture");
    }

    #[test]
    fn math_inline_node_serializes() {
        let node = Node::MathInline(MathNode {
            raw: "x^2".into(),
            tree: MathExpr::Superscript {
                base: Box::new(MathExpr::Ident { value: "x".into() }),
                script: Box::new(MathExpr::Number { value: "2".into() }),
            },
            position: span(1, 1, 0, 1, 6, 5),
        });
        let json = serde_json::to_value(&node).unwrap();
        assert_eq!(json["type"], "math_inline");
        assert_eq!(json["raw"], "x^2");
        assert_eq!(json["tree"]["type"], "superscript");
        assert_eq!(json["tree"]["base"]["type"], "ident");
        assert_eq!(json["tree"]["base"]["value"], "x");
        assert_eq!(json["tree"]["script"]["type"], "number");
        assert_eq!(json["tree"]["script"]["value"], "2");
    }

    #[test]
    fn math_display_node_serializes_with_label() {
        let node = Node::MathDisplay(MathDisplayNode {
            raw: "E = mc^2".into(),
            tree: MathExpr::Row {
                children: vec![MathExpr::Ident { value: "E".into() }],
            },
            label: Some("eq:einstein".into()),
            position: span(1, 1, 0, 1, 12, 11),
        });
        let json = serde_json::to_value(&node).unwrap();
        assert_eq!(json["type"], "math_display");
        assert_eq!(json["raw"], "E = mc^2");
        assert_eq!(json["label"], "eq:einstein");
        assert_eq!(json["tree"]["type"], "row");
    }

    #[test]
    fn math_display_node_omits_label_when_none() {
        let node = Node::MathDisplay(MathDisplayNode {
            raw: "x".into(),
            tree: MathExpr::Ident { value: "x".into() },
            label: None,
            position: span(1, 1, 0, 1, 3, 2),
        });
        let json = serde_json::to_value(&node).unwrap();
        assert!(json.get("label").is_none());
    }

    #[test]
    fn definition_list_variants_have_children() {
        let dl = Node::DefinitionList(StandardBlockNode {
            depth: None,
            ordered: None,
            checked: None,
            id: None,
            children: vec![Node::DefinitionTerm(StandardBlockNode {
                depth: None,
                ordered: None,
                checked: None,
                id: None,
                children: vec![Node::Text(TextNode {
                    value: "Term".into(),
                    position: span(1, 1, 0, 1, 5, 4),
                })],
                position: span(1, 1, 0, 1, 5, 4),
            })],
            position: span(1, 1, 0, 2, 1, 10),
        });
        let json = serde_json::to_value(&dl).unwrap();
        assert_eq!(json["type"], "definition_list");
        assert_eq!(json["children"][0]["type"], "definition_term");
        assert_eq!(json["children"][0]["children"][0]["value"], "Term");
    }

    #[test]
    fn definition_list_children_accessible() {
        let node = Node::DefinitionList(StandardBlockNode {
            depth: None,
            ordered: None,
            checked: None,
            id: None,
            children: vec![],
            position: span(1, 1, 0, 1, 1, 0),
        });
        assert!(node.children().is_some());
    }

    #[test]
    fn math_frac_roundtrip() {
        let expr = MathExpr::Frac {
            numerator: Box::new(MathExpr::Number { value: "1".into() }),
            denominator: Box::new(MathExpr::Number { value: "2".into() }),
            style: FracStyle::Auto,
        };
        let serialized = serde_json::to_string(&expr).unwrap();
        let deserialized: MathExpr = serde_json::from_str(&serialized).unwrap();
        assert_eq!(expr, deserialized);
    }

    #[test]
    fn math_error_variant_serializes() {
        let expr = MathExpr::Error {
            raw: r"\unknown".into(),
            message: "Unknown command".into(),
        };
        let json = serde_json::to_value(&expr).unwrap();
        assert_eq!(json["type"], "error");
        assert_eq!(json["raw"], r"\unknown");
        assert_eq!(json["message"], "Unknown command");
    }
}
