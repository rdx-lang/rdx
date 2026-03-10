use rdx_ast::*;

use crate::source_map::SourceMap;

/// Result of parsing attributes: Ok with list and end position, or Err with error info.
pub(crate) type AttrResult = Result<(Vec<AttributeNode>, bool, usize), AttrError>;

pub(crate) struct AttrError {
    pub message: String,
    pub raw: String,
    pub start: usize,
    pub end: usize,
}

/// Parse the rest of a tag after the tag name: attributes then `>` or `/>`.
/// Enforces the zero-whitespace rule around `=` per spec 2.3.1.
/// Returns Ok((attributes, self_closing, end_pos)) or Err on parse failure.
pub(crate) fn parse_tag_rest(
    input: &str,
    mut pos: usize,
    base_offset: usize,
    sm: &SourceMap,
    tag_start: usize,
) -> AttrResult {
    let bytes = input.as_bytes();
    let mut attributes = Vec::new();

    loop {
        // Skip whitespace between attributes
        while pos < input.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= input.len() {
            return Err(AttrError {
                message: "Unexpected end of input in tag".into(),
                raw: input[tag_start..].to_string(),
                start: base_offset + tag_start,
                end: base_offset + input.len(),
            });
        }

        // Check for end of tag
        if bytes[pos] == b'>' {
            return Ok((attributes, false, pos + 1));
        }
        if bytes[pos] == b'/' && pos + 1 < input.len() && bytes[pos + 1] == b'>' {
            return Ok((attributes, true, pos + 2));
        }

        // Parse attribute name: [a-zA-Z_][a-zA-Z0-9_-]*
        let attr_start = pos;
        if !bytes[pos].is_ascii_alphabetic() && bytes[pos] != b'_' {
            return Err(AttrError {
                message: format!("Invalid attribute name character '{}'", bytes[pos] as char),
                raw: input[tag_start..].to_string(),
                start: base_offset + tag_start,
                end: base_offset + input.len(),
            });
        }
        while pos < input.len()
            && (bytes[pos].is_ascii_alphanumeric() || bytes[pos] == b'_' || bytes[pos] == b'-')
        {
            pos += 1;
        }
        let attr_name = input[attr_start..pos].to_string();

        // Zero-whitespace rule: `=` MUST immediately follow the attribute name (no whitespace).
        // Per spec 2.3.1, whitespace between name and `=` is NOT permitted.
        if pos < input.len() && bytes[pos] == b'=' {
            pos += 1; // consume =

            // Zero-whitespace rule: value MUST immediately follow `=` (no whitespace).
            if pos >= input.len() {
                return Err(AttrError {
                    message: "Unexpected end of input after '='".into(),
                    raw: input[tag_start..].to_string(),
                    start: base_offset + tag_start,
                    end: base_offset + input.len(),
                });
            }

            // Determine attribute value type
            if bytes[pos] == b'"' || bytes[pos] == b'\'' {
                // String attribute (spec 2.3.2)
                let (value, val_end) = parse_string_attr(input, pos).map_err(|msg| AttrError {
                    message: msg,
                    raw: input[attr_start..].to_string(),
                    start: base_offset + attr_start,
                    end: base_offset + input.len(),
                })?;
                attributes.push(AttributeNode {
                    name: attr_name,
                    value: AttributeValue::String(value),
                    position: sm.position(base_offset + attr_start, base_offset + val_end),
                });
                pos = val_end;
            } else if bytes[pos] == b'{' && pos + 1 < input.len() && bytes[pos + 1] == b'{' {
                // JSON attribute ={{ ... }} (spec 2.3.4)
                match parse_json_attr(input, pos) {
                    Ok((value, val_end)) => {
                        attributes.push(AttributeNode {
                            name: attr_name,
                            value,
                            position: sm.position(base_offset + attr_start, base_offset + val_end),
                        });
                        pos = val_end;
                    }
                    Err(json_err) => {
                        // Malformed JSON -> emit error node per spec 3.1
                        attributes.push(AttributeNode {
                            name: attr_name,
                            value: AttributeValue::Null, // placeholder
                            position: sm
                                .position(base_offset + attr_start, base_offset + json_err.end),
                        });
                        return Err(AttrError {
                            message: json_err.message,
                            raw: json_err.raw,
                            start: base_offset + json_err.start,
                            end: base_offset + json_err.end,
                        });
                    }
                }
            } else if bytes[pos] == b'{' && pos + 1 < input.len() && bytes[pos + 1] == b'$' {
                // Variable attribute {$path} (spec 2.3.5)
                match parse_variable_attr(input, pos, base_offset, sm) {
                    Ok((value, val_end)) => {
                        attributes.push(AttributeNode {
                            name: attr_name,
                            value,
                            position: sm.position(base_offset + attr_start, base_offset + val_end),
                        });
                        pos = val_end;
                    }
                    Err(var_err) => {
                        // Invalid variable path -> emit error per spec 3.5
                        return Err(AttrError {
                            message: var_err.message,
                            raw: var_err.raw,
                            start: base_offset + var_err.start,
                            end: base_offset + var_err.end,
                        });
                    }
                }
            } else if bytes[pos] == b'{' {
                // Primitive attribute {value} (spec 2.3.3)
                match parse_primitive_attr(input, pos) {
                    Ok((value, val_end)) => {
                        attributes.push(AttributeNode {
                            name: attr_name,
                            value,
                            position: sm.position(base_offset + attr_start, base_offset + val_end),
                        });
                        pos = val_end;
                    }
                    Err(msg) => {
                        return Err(AttrError {
                            message: msg,
                            raw: input[attr_start..].to_string(),
                            start: base_offset + attr_start,
                            end: base_offset + input.len(),
                        });
                    }
                }
            } else {
                return Err(AttrError {
                    message: format!("Invalid attribute value after '=' for '{}'", attr_name),
                    raw: input[attr_start..].to_string(),
                    start: base_offset + attr_start,
                    end: base_offset + input.len(),
                });
            }
        } else {
            // Boolean shorthand (spec 2.2.5)
            // Position spans only the attribute name token
            attributes.push(AttributeNode {
                name: attr_name,
                value: AttributeValue::Bool(true),
                position: sm.position(base_offset + attr_start, base_offset + pos),
            });
        }
    }
}

/// Parse a string attribute value starting at the opening quote.
/// Handles `\"` / `\'` and `\\` escapes per spec 2.3.2.
/// All other `\X` sequences are passed through as-is.
fn parse_string_attr(input: &str, pos: usize) -> Result<(String, usize), String> {
    let bytes = input.as_bytes();
    let quote = bytes[pos];
    let mut i = pos + 1;
    let mut value = String::new();
    while i < input.len() {
        if bytes[i] == b'\\' && i + 1 < input.len() {
            if bytes[i + 1] == quote {
                value.push(quote as char);
                i += 2;
            } else if bytes[i + 1] == b'\\' {
                value.push('\\');
                i += 2;
            } else {
                // Pass through as-is per spec 2.3.2
                value.push('\\');
                value.push(bytes[i + 1] as char);
                i += 2;
            }
        } else if bytes[i] == quote {
            return Ok((value, i + 1));
        } else {
            // Handle multi-byte UTF-8
            let ch = input[i..].chars().next().unwrap();
            value.push(ch);
            i += ch.len_utf8();
        }
    }
    Err("Unclosed string attribute".into())
}

/// Parse a primitive attribute {value} (bool, number, null).
/// Per spec 2.3.3: valid contents are integers, floats (including negative
/// and scientific notation), `true`, `false`, and `null`.
fn parse_primitive_attr(input: &str, pos: usize) -> Result<(AttributeValue, usize), String> {
    let bytes = input.as_bytes();
    if bytes[pos] != b'{' {
        return Err("Expected '{'".into());
    }
    let close = input[pos..]
        .find('}')
        .ok_or("Unclosed primitive attribute")?
        + pos;
    let content = &input[pos + 1..close];
    // No trimming — spec enforces exact content (whitespace is part of the value)
    // However, the spec examples show `{true}` and `{2}` without spaces, and the
    // zero-whitespace rule in 2.3.1 applies to around `=`, not inside `{}`.
    // We trim to be lenient for the content inside braces.
    let trimmed = content.trim();
    let value = match trimmed {
        "true" => AttributeValue::Bool(true),
        "false" => AttributeValue::Bool(false),
        "null" => AttributeValue::Null,
        _ => {
            if let Ok(n) = trimmed.parse::<i64>() {
                AttributeValue::Number(n.into())
            } else if let Ok(f) = trimmed.parse::<f64>() {
                match serde_json::Number::from_f64(f) {
                    Some(n) => AttributeValue::Number(n),
                    None => return Err(format!("Non-finite float value: {}", trimmed)),
                }
            } else {
                return Err(format!("Invalid primitive value: {}", trimmed));
            }
        }
    };
    Ok((value, close + 1))
}

struct JsonAttrError {
    message: String,
    raw: String,
    start: usize,
    end: usize,
}

/// Parse a JSON attribute ={{ ... }}
/// Per spec 2.3.4: content between `{{` and `}}` must be valid JSON.
/// For objects, the `{{ }}` implicitly provides the outer `{ }`.
/// For arrays, the content `[...]` is used as-is.
fn parse_json_attr(input: &str, pos: usize) -> Result<(AttributeValue, usize), JsonAttrError> {
    let bytes = input.as_bytes();
    if bytes[pos] != b'{' || bytes.get(pos + 1) != Some(&b'{') {
        return Err(JsonAttrError {
            message: "Expected '{{' for JSON attribute".into(),
            raw: String::new(),
            start: pos,
            end: pos,
        });
    }

    // Find matching }} handling nested braces in JSON strings
    let mut i = pos + 2;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape_next = false;

    while i < input.len() {
        if escape_next {
            escape_next = false;
            i += 1;
            continue;
        }
        let b = bytes[i];
        if in_string {
            if b == b'\\' {
                escape_next = true;
            } else if b == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        match b {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                if depth > 0 {
                    depth -= 1;
                } else if i + 1 < input.len() && bytes[i + 1] == b'}' {
                    // Found closing }}
                    let json_content = input[pos + 2..i].trim();
                    // Try as-is first (for arrays), then wrapped in {} (for objects)
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_content) {
                        return match val {
                            serde_json::Value::Object(map) => {
                                Ok((AttributeValue::Object(map), i + 2))
                            }
                            serde_json::Value::Array(arr) => {
                                Ok((AttributeValue::Array(arr), i + 2))
                            }
                            other => {
                            return Err(JsonAttrError {
                                message: format!(
                                    "JSON attribute must be an object or array, got: {}",
                                    other
                                ),
                                raw: input[pos..i + 2].to_string(),
                                start: pos,
                                end: i + 2,
                            });
                        }
                        };
                    }
                    // Wrap in {} for object shorthand
                    let wrapped = format!("{{{}}}", json_content);
                    if let Ok(serde_json::Value::Object(map)) =
                        serde_json::from_str::<serde_json::Value>(&wrapped)
                    {
                        return Ok((AttributeValue::Object(map), i + 2));
                    }
                    // Malformed JSON -> error per spec 3.1
                    return Err(JsonAttrError {
                        message: format!("Malformed JSON in attribute: {}", json_content),
                        raw: input[pos..i + 2].to_string(),
                        start: pos,
                        end: i + 2,
                    });
                }
            }
            _ => {}
        }
        i += 1;
    }

    Err(JsonAttrError {
        message: "Unclosed JSON attribute '{{...}}'".into(),
        raw: input[pos..].to_string(),
        start: pos,
        end: input.len(),
    })
}

struct VarAttrError {
    message: String,
    raw: String,
    start: usize,
    end: usize,
}

/// Parse a variable attribute {$path}.
/// Validates path against spec 2.4.1 grammar.
fn parse_variable_attr(
    input: &str,
    pos: usize,
    _base_offset: usize,
    sm: &SourceMap,
) -> Result<(AttributeValue, usize), VarAttrError> {
    let bytes = input.as_bytes();
    if bytes[pos] != b'{' || bytes.get(pos + 1) != Some(&b'$') {
        return Err(VarAttrError {
            message: "Expected '{$'".into(),
            raw: String::new(),
            start: pos,
            end: pos,
        });
    }
    let close = input[pos..].find('}').ok_or_else(|| VarAttrError {
        message: "Unclosed variable attribute".into(),
        raw: input[pos..].to_string(),
        start: pos,
        end: input.len(),
    })? + pos;
    let path = &input[pos + 2..close];
    if crate::is_valid_variable_path(path) {
        let var_node = VariableNode {
            path: path.to_string(),
            position: sm.position(_base_offset + pos, _base_offset + close + 1),
        };
        Ok((AttributeValue::Variable(var_node), close + 1))
    } else {
        Err(VarAttrError {
            message: format!("Invalid variable path: {}", path),
            raw: input[pos..close + 1].to_string(),
            start: pos,
            end: close + 1,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    #[test]
    fn string_double_quotes() {
        let root = parse("<Comp label=\"hello\" />\n");
        match &root.children[0] {
            Node::Component(c) => {
                assert_eq!(
                    c.attributes[0].value,
                    AttributeValue::String("hello".into())
                );
            }
            other => panic!("Expected component, got {:?}", other),
        }
    }

    #[test]
    fn string_single_quotes() {
        let root = parse("<Comp label='hello' />\n");
        match &root.children[0] {
            Node::Component(c) => {
                assert_eq!(
                    c.attributes[0].value,
                    AttributeValue::String("hello".into())
                );
            }
            other => panic!("Expected component, got {:?}", other),
        }
    }

    #[test]
    fn string_escaped_quote() {
        let root = parse("<Comp label=\"say \\\"hi\\\"\" />\n");
        match &root.children[0] {
            Node::Component(c) => {
                assert_eq!(
                    c.attributes[0].value,
                    AttributeValue::String("say \"hi\"".into())
                );
            }
            other => panic!("Expected component, got {:?}", other),
        }
    }

    #[test]
    fn string_unrecognized_escape_passthrough() {
        let root = parse("<Comp label=\"hello\\nworld\" />\n");
        match &root.children[0] {
            Node::Component(c) => {
                // \n is not a recognized escape, passed through as-is
                assert_eq!(
                    c.attributes[0].value,
                    AttributeValue::String("hello\\nworld".into())
                );
            }
            other => panic!("Expected component, got {:?}", other),
        }
    }

    #[test]
    fn primitive_bool_true() {
        let root = parse("<Comp x={true} />\n");
        match &root.children[0] {
            Node::Component(c) => assert_eq!(c.attributes[0].value, AttributeValue::Bool(true)),
            other => panic!("Expected component, got {:?}", other),
        }
    }

    #[test]
    fn primitive_null() {
        let root = parse("<Comp x={null} />\n");
        match &root.children[0] {
            Node::Component(c) => assert_eq!(c.attributes[0].value, AttributeValue::Null),
            other => panic!("Expected component, got {:?}", other),
        }
    }

    #[test]
    fn primitive_negative_float() {
        let root = parse("<Comp x={-3.14} />\n");
        match &root.children[0] {
            Node::Component(c) => match &c.attributes[0].value {
                AttributeValue::Number(n) => {
                    assert!((n.as_f64().unwrap() - (-3.14)).abs() < f64::EPSILON)
                }
                other => panic!("Expected number, got {:?}", other),
            },
            other => panic!("Expected component, got {:?}", other),
        }
    }

    #[test]
    fn primitive_scientific_notation() {
        let root = parse("<Comp x={2.5e10} />\n");
        match &root.children[0] {
            Node::Component(c) => match &c.attributes[0].value {
                AttributeValue::Number(n) => assert!((n.as_f64().unwrap() - 2.5e10).abs() < 1.0),
                other => panic!("Expected number, got {:?}", other),
            },
            other => panic!("Expected component, got {:?}", other),
        }
    }

    #[test]
    fn json_object_attribute() {
        let root = parse("<Chart config={{\"type\": \"bar\", \"data\": [10, 20]}} />\n");
        match &root.children[0] {
            Node::Component(c) => match &c.attributes[0].value {
                AttributeValue::Object(map) => {
                    assert_eq!(map["type"], serde_json::Value::String("bar".into()));
                }
                other => panic!("Expected object, got {:?}", other),
            },
            other => panic!("Expected component, got {:?}", other),
        }
    }

    #[test]
    fn json_array_attribute() {
        let root = parse("<Comp items={{[\"a\", \"b\"]}} />\n");
        match &root.children[0] {
            Node::Component(c) => match &c.attributes[0].value {
                AttributeValue::Array(arr) => assert_eq!(arr.len(), 2),
                other => panic!("Expected array, got {:?}", other),
            },
            other => panic!("Expected component, got {:?}", other),
        }
    }

    #[test]
    fn malformed_json_produces_error() {
        let root = parse("<Chart config={{invalid json}} />\n");
        let has_error = root.children.iter().any(|n| matches!(n, Node::Error(_)));
        assert!(
            has_error,
            "Malformed JSON should produce error: {:?}",
            root.children
        );
    }

    #[test]
    fn variable_attribute() {
        let root = parse("<Comp x={$config.theme} />\n");
        match &root.children[0] {
            Node::Component(c) => match &c.attributes[0].value {
                AttributeValue::Variable(v) => assert_eq!(v.path, "config.theme"),
                other => panic!("Expected variable, got {:?}", other),
            },
            other => panic!("Expected component, got {:?}", other),
        }
    }

    #[test]
    fn invalid_variable_attr_produces_error() {
        let root = parse("<Comp x={$123.bad} />\n");
        let has_error = root.children.iter().any(|n| matches!(n, Node::Error(_)));
        assert!(
            has_error,
            "Invalid variable path in attr should error: {:?}",
            root.children
        );
    }

    #[test]
    fn boolean_shorthand() {
        let root = parse("<Input disabled />\n");
        match &root.children[0] {
            Node::Component(c) => {
                assert_eq!(c.attributes[0].name, "disabled");
                assert_eq!(c.attributes[0].value, AttributeValue::Bool(true));
            }
            other => panic!("Expected component, got {:?}", other),
        }
    }

    #[test]
    fn all_attribute_types_combined() {
        let root = parse(
            "<W label=\"hi\" count={42} active={true} data={{[1,2]}} ref={$cfg.x} disabled />\n",
        );
        match &root.children[0] {
            Node::Component(c) => {
                assert_eq!(c.attributes.len(), 6);
                assert_eq!(c.attributes[0].value, AttributeValue::String("hi".into()));
                assert_eq!(c.attributes[1].value, AttributeValue::Number(42.into()));
                assert_eq!(c.attributes[2].value, AttributeValue::Bool(true));
                assert!(matches!(&c.attributes[3].value, AttributeValue::Array(_)));
                assert!(matches!(
                    &c.attributes[4].value,
                    AttributeValue::Variable(_)
                ));
                assert_eq!(c.attributes[5].value, AttributeValue::Bool(true));
            }
            other => panic!("Expected component, got {:?}", other),
        }
    }
}
