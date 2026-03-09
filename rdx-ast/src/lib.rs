use serde::{Deserialize, Serialize};

/// Positional data mapping an AST node back to its source `.rdx` file.
/// Line and column numbers are 1-indexed. Offsets are 0-indexed byte offsets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub start: Point,
    pub end: Point,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

/// The root of an RDX document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Root {
    #[serde(rename = "type")]
    pub node_type: RootType,
    pub frontmatter: Option<serde_json::Value>,
    pub children: Vec<Node>,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RootType {
    #[serde(rename = "root")]
    Root,
}

/// A union of all possible RDX nodes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Node {
    #[serde(rename = "text")]
    Text(TextNode),
    #[serde(rename = "code_inline")]
    CodeInline(TextNode),
    #[serde(rename = "code_block")]
    CodeBlock(CodeBlockNode),
    #[serde(rename = "paragraph")]
    Paragraph(StandardBlockNode),
    #[serde(rename = "heading")]
    Heading(StandardBlockNode),
    #[serde(rename = "list")]
    List(StandardBlockNode),
    #[serde(rename = "list_item")]
    ListItem(StandardBlockNode),
    #[serde(rename = "blockquote")]
    Blockquote(StandardBlockNode),
    #[serde(rename = "thematic_break")]
    ThematicBreak(StandardBlockNode),
    #[serde(rename = "html")]
    Html(StandardBlockNode),
    #[serde(rename = "table")]
    Table(StandardBlockNode),
    #[serde(rename = "table_row")]
    TableRow(StandardBlockNode),
    #[serde(rename = "table_cell")]
    TableCell(StandardBlockNode),
    #[serde(rename = "link")]
    Link(LinkNode),
    #[serde(rename = "image")]
    Image(ImageNode),
    #[serde(rename = "emphasis")]
    Emphasis(StandardBlockNode),
    #[serde(rename = "strong")]
    Strong(StandardBlockNode),
    #[serde(rename = "strikethrough")]
    Strikethrough(StandardBlockNode),
    #[serde(rename = "footnote_definition")]
    FootnoteDefinition(FootnoteNode),
    #[serde(rename = "footnote_reference")]
    FootnoteReference(FootnoteNode),
    #[serde(rename = "math_inline")]
    MathInline(TextNode),
    #[serde(rename = "math_display")]
    MathDisplay(TextNode),
    #[serde(rename = "component")]
    Component(ComponentNode),
    #[serde(rename = "variable")]
    Variable(VariableNode),
    #[serde(rename = "error")]
    Error(ErrorNode),
}

/// A standard CommonMark block node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    pub children: Vec<Node>,
    pub position: Position,
}

/// An RDX component node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentNode {
    pub name: String,
    #[serde(rename = "isInline")]
    pub is_inline: bool,
    pub attributes: Vec<AttributeNode>,
    pub children: Vec<Node>,
    pub position: Position,
}

/// A single attribute with its own positional data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttributeNode {
    pub name: String,
    pub value: AttributeValue,
    pub position: Position,
}

/// Supported attribute value types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AttributeValue {
    Null,
    Bool(bool),
    Number(serde_json::Number),
    String(String),
    Array(Vec<serde_json::Value>),
    Object(serde_json::Map<String, serde_json::Value>),
    Variable(VariableNode),
}

/// A footnote node (definition or reference).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FootnoteNode {
    pub label: String,
    pub children: Vec<Node>,
    pub position: Position,
}

/// A link node with URL and optional title.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkNode {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub children: Vec<Node>,
    pub position: Position,
}

/// An image node with URL, optional title, and alt text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageNode {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
    pub children: Vec<Node>,
    pub position: Position,
}

/// A fenced code block with optional language and meta string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeBlockNode {
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<String>,
    pub position: Position,
}

/// A literal text node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextNode {
    pub value: String,
    pub position: Position,
}

/// A variable interpolation node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariableNode {
    pub path: String,
    pub position: Position,
}

/// An explicit error node for host-level error boundaries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorNode {
    pub message: String,
    #[serde(rename = "rawContent")]
    pub raw_content: String,
    pub position: Position,
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
            | Node::ThematicBreak(b) => Some(&mut b.children),
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
            | Node::ThematicBreak(b) => Some(&b.children),
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
            position: span(1, 1, 0, 1, 10, 9),
        });
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: Node = serde_json::from_str(&serialized).unwrap();
        assert_eq!(original, deserialized);
    }
}
