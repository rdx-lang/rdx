use std::collections::HashMap;

use rdx_ast::AttributeValue;
use serde::{Deserialize, Serialize};

mod validate;
pub use validate::{Diagnostic, Severity, validate};

pub mod builtins;
pub use builtins::standard_schema;

/// A schema registry mapping component names to their definitions.
///
/// # Example
///
/// ```rust
/// use rdx_schema::{Schema, ComponentSchema, PropSchema, PropType};
///
/// let schema = Schema::new()
///     .component("Notice", ComponentSchema::new()
///         .prop("type", PropSchema::required(PropType::String))
///         .prop("title", PropSchema::optional(PropType::String))
///     )
///     .component("Badge", ComponentSchema::new()
///         .prop("label", PropSchema::required(PropType::String))
///         .prop("color", PropSchema::optional(PropType::String))
///     );
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Schema {
    /// Component definitions keyed by tag name.
    pub components: HashMap<String, ComponentSchema>,
    /// When true, components not in the schema produce an error.
    /// When false (default), unknown components are silently accepted.
    #[serde(default)]
    pub strict: bool,
    /// Props that are valid on every component regardless of its own schema.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub global_props: Vec<(String, PropSchema)>,
}

impl Schema {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a component definition. Builder pattern.
    pub fn component(mut self, name: &str, schema: ComponentSchema) -> Self {
        self.components.insert(name.to_string(), schema);
        self
    }

    /// Enable strict mode: unknown components produce errors.
    pub fn strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }

    /// Register a global prop that is valid on any component. Builder pattern.
    pub fn global_prop(mut self, name: &str, schema: PropSchema) -> Self {
        self.global_props.push((name.to_string(), schema));
        self
    }
}

/// Schema definition for a single component.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComponentSchema {
    /// Allowed props keyed by attribute name.
    pub props: HashMap<String, PropSchema>,
    /// Whether the component must be self-closing (no children).
    #[serde(default)]
    pub self_closing: bool,
    /// Optional list of allowed child component names.
    /// If `None`, any children are allowed. If `Some`, only listed component names
    /// (and standard Markdown nodes) are permitted as direct children.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_children: Option<Vec<String>>,
    /// Human-readable description for tooling and error messages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl ComponentSchema {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a prop definition. Builder pattern.
    pub fn prop(mut self, name: &str, schema: PropSchema) -> Self {
        self.props.insert(name.to_string(), schema);
        self
    }

    /// Mark as self-closing (must not have children).
    pub fn self_closing(mut self, val: bool) -> Self {
        self.self_closing = val;
        self
    }

    /// Restrict allowed child component names.
    pub fn allowed_children(mut self, names: Vec<&str>) -> Self {
        self.allowed_children = Some(names.into_iter().map(|s| s.to_string()).collect());
        self
    }

    /// Set a description.
    pub fn description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    /// Inherit all props from `base` that are not already defined on `self`.
    ///
    /// This implements a "mixin" pattern: `self` keeps its own props unchanged,
    /// and any prop from `base` that does not already exist in `self` is copied
    /// over. When both `self` and `base` define a prop with the same name,
    /// `self`'s definition wins (component-specific takes priority over the
    /// base).
    ///
    /// # Example
    ///
    /// ```rust
    /// use rdx_schema::{ComponentSchema, PropSchema, PropType};
    ///
    /// let base = ComponentSchema::new()
    ///     .prop("id", PropSchema::optional(PropType::String))
    ///     .prop("class", PropSchema::optional(PropType::String));
    ///
    /// let derived = ComponentSchema::new()
    ///     .prop("id", PropSchema::required(PropType::String)) // overrides base
    ///     .extends(&base);
    ///
    /// // "id" comes from derived (required), "class" comes from base (optional).
    /// assert!(derived.props["id"].required);
    /// assert!(!derived.props["class"].required);
    /// ```
    pub fn extends(mut self, base: &ComponentSchema) -> Self {
        for (name, prop) in &base.props {
            self.props.entry(name.clone()).or_insert_with(|| prop.clone());
        }
        self
    }
}

/// Schema definition for a single prop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropSchema {
    /// The expected type.
    #[serde(rename = "type")]
    pub prop_type: PropType,
    /// Whether the prop is required.
    #[serde(default)]
    pub required: bool,
    /// Optional default value (informational; not applied by the validator).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    /// For `PropType::Enum`, the allowed string values.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
    /// Human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl PropSchema {
    /// A required prop of the given type.
    pub fn required(prop_type: PropType) -> Self {
        PropSchema {
            prop_type,
            required: true,
            default: None,
            values: None,
            description: None,
        }
    }

    /// An optional prop of the given type.
    pub fn optional(prop_type: PropType) -> Self {
        PropSchema {
            prop_type,
            required: false,
            default: None,
            values: None,
            description: None,
        }
    }

    /// A required enum prop restricted to specific string values.
    pub fn enum_required(values: Vec<&str>) -> Self {
        PropSchema {
            prop_type: PropType::Enum,
            required: true,
            default: None,
            values: Some(values.into_iter().map(|s| s.to_string()).collect()),
            description: None,
        }
    }

    /// An optional enum prop.
    pub fn enum_optional(values: Vec<&str>) -> Self {
        PropSchema {
            prop_type: PropType::Enum,
            required: false,
            default: None,
            values: Some(values.into_iter().map(|s| s.to_string()).collect()),
            description: None,
        }
    }

    /// Set a default value (informational).
    pub fn with_default(mut self, val: serde_json::Value) -> Self {
        self.default = Some(val);
        self
    }

    /// Set a description.
    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }
}

/// The expected type of a prop value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PropType {
    /// Any string value.
    String,
    /// A numeric value (integer or float).
    Number,
    /// A boolean value.
    Boolean,
    /// A restricted set of string values (see `PropSchema::values`).
    Enum,
    /// A JSON object (`{{ }}`).
    Object,
    /// A JSON array (`{{ }}`).
    Array,
    /// A context variable (`{$path}`).
    Variable,
    /// Accepts any attribute value type.
    Any,
}

/// Check whether an `AttributeValue` matches the expected `PropType`.
pub(crate) fn type_matches(value: &AttributeValue, expected: &PropType) -> bool {
    match expected {
        PropType::Any => true,
        PropType::String => matches!(value, AttributeValue::String(_)),
        PropType::Number => matches!(value, AttributeValue::Number(_)),
        PropType::Boolean => matches!(value, AttributeValue::Bool(_)),
        PropType::Object => matches!(value, AttributeValue::Object(_)),
        PropType::Array => matches!(value, AttributeValue::Array(_)),
        PropType::Variable => matches!(value, AttributeValue::Variable(_)),
        PropType::Enum => matches!(value, AttributeValue::String(_)),
    }
}

/// Get a human-readable name for an attribute value's type.
pub(crate) fn value_type_name(value: &AttributeValue) -> &'static str {
    match value {
        AttributeValue::Null => "null",
        AttributeValue::Bool(_) => "boolean",
        AttributeValue::Number(_) => "number",
        AttributeValue::String(_) => "string",
        AttributeValue::Array(_) => "array",
        AttributeValue::Object(_) => "object",
        AttributeValue::Variable(_) => "variable",
    }
}
