#[derive(Debug, Clone)]
pub struct Warning {
    pub line_number: usize,
    pub message: String,
}

/// Converts MDX content to RDX format.
///
/// This is a text-level converter that handles common MDX patterns:
/// - Removes import/export statements
/// - Converts JSX comments to HTML comments
/// - Strips JS expression attributes with warnings
/// - Converts className to class
///
/// Returns the converted content and a list of warnings encountered.
pub fn convert_mdx_to_rdx(input: &str) -> (String, Vec<Warning>) {
    let mut warnings = Vec::new();
    let lines: Vec<&str> = input.lines().collect();
    let mut output_lines = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let line_number = i + 1;

        // Handle multi-line imports/exports
        if should_skip_import_export(line) {
            // Skip lines that are part of import/export statements
            while i < lines.len() && !is_import_export_end(lines[i]) {
                i += 1;
            }
            // Skip the final line of the import/export
            if i < lines.len() {
                i += 1;
            }
            continue;
        }

        // Convert the line
        let (converted, line_warnings) = convert_line(line, line_number);
        warnings.extend(line_warnings);
        output_lines.push(converted);
        i += 1;
    }

    (output_lines.join("\n"), warnings)
}

/// Determines if a line is the start of an import/export statement.
fn should_skip_import_export(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("import ") || trimmed.starts_with("export ")
}

/// Determines if a line is the end of a multi-line import/export.
fn is_import_export_end(line: &str) -> bool {
    let trimmed = line.trim_end();

    // Lines ending with 'from' typically mark the end of an import statement
    if line.contains("from")
        && (trimmed.ends_with("'") || trimmed.ends_with("\"") || trimmed.ends_with("}"))
    {
        return true;
    }

    // Single-line import: must contain 'from' and end with quote or brace
    if line.contains("import") && line.contains("from") {
        return trimmed.ends_with("'") || trimmed.ends_with("\"") || trimmed.ends_with("}");
    }

    // Single-line export: could be export const/let/var/default = ...
    // These typically end with } or ;
    if line.trim().starts_with("export ") {
        // If it's on one line and doesn't continue, it should end with } or ;
        // For simplicity, if the line both starts and seems complete, treat it as end
        if trimmed.ends_with("}")
            || trimmed.ends_with(";")
            || trimmed.ends_with("'")
            || trimmed.ends_with("\"")
        {
            // Make sure brace count is balanced
            let open_braces = line.matches('{').count();
            let close_braces = line.matches('}').count();
            return open_braces == close_braces;
        }
    }

    false
}

/// Converts a single line from MDX to RDX.
fn convert_line(line: &str, line_number: usize) -> (String, Vec<Warning>) {
    let mut warnings = Vec::new();
    let mut result = String::new();
    let mut i = 0;
    let chars: Vec<char> = line.chars().collect();

    while i < chars.len() {
        // Check for JSX comment {/* ... */}
        if chars[i] == '{' && i + 1 < chars.len() && chars[i + 1] == '/' {
            // Look ahead to see if this is a JSX comment
            let mut j = i + 2;
            if j < chars.len() && chars[j] == '*' {
                // Find the closing */}
                j += 1;
                let comment_start = j;
                let mut found_end = false;

                while j + 1 < chars.len() {
                    if chars[j] == '*' && chars[j + 1] == '/' {
                        // Check if followed by '}'
                        if j + 2 < chars.len() && chars[j + 2] == '}' {
                            // Found complete JSX comment
                            let comment_text = chars[comment_start..j]
                                .iter()
                                .collect::<String>()
                                .trim()
                                .to_string();
                            result.push_str(&format!("<!-- {} -->", comment_text));
                            i = j + 3; // skip past */}
                            found_end = true;
                            break;
                        }
                    }
                    j += 1;
                }

                if found_end {
                    continue;
                }
            }
        }

        // Check for attribute expressions like onClick={...}
        // Preserve RDX-valid values: {true}, {false}, {null}, {numbers}, {$var}, {{json}}
        if chars[i] == '=' && i + 1 < chars.len() && chars[i + 1] == '{' {
            // Find the closing } (accounting for nesting)
            let mut j = i + 2;
            let mut brace_depth = 1;

            while j < chars.len() {
                if chars[j] == '{' {
                    brace_depth += 1;
                } else if chars[j] == '}' {
                    brace_depth -= 1;
                    if brace_depth == 0 {
                        break;
                    }
                }
                j += 1;
            }

            if brace_depth == 0 {
                let expr: String = chars[(i + 2)..j].iter().collect();
                let trimmed_expr = expr.trim();

                // Check if this is a valid RDX attribute value
                if is_rdx_valid_expression(trimmed_expr) {
                    // Keep it as-is, push ={...} to result
                    for ch in &chars[i..=j] {
                        result.push(*ch);
                    }
                    i = j + 1;
                    continue;
                }

                // It's a JS expression — strip the attribute
                let mut attr_start = result.len();
                while attr_start > 0
                    && result.as_bytes()[attr_start - 1] != b' '
                    && result.as_bytes()[attr_start - 1] != b'<'
                {
                    attr_start -= 1;
                }
                let attr_name = result[attr_start..].trim().to_string();

                warnings.push(Warning {
                    line_number,
                    message: format!(
                        "Stripped JS expression attribute {}={{{}}}",
                        attr_name, trimmed_expr
                    ),
                });
                result.truncate(attr_start);
                result = result.trim_end().to_string();
                i = j + 1;
                continue;
            }
        }

        result.push(chars[i]);
        i += 1;
    }

    // Convert className to class
    result = result.replace("className=", "class=");

    (result, warnings)
}

/// Check if an expression inside `={...}` is valid RDX (not JS).
fn is_rdx_valid_expression(expr: &str) -> bool {
    // RDX primitives: true, false, null
    if matches!(expr, "true" | "false" | "null") {
        return true;
    }

    // Numbers (including negative, floats, scientific notation)
    if expr.parse::<f64>().is_ok() {
        return true;
    }

    // Variable references: $path.to.var
    if let Some(path) = expr.strip_prefix('$') {
        // Validate: [a-zA-Z_][a-zA-Z0-9_]*(\.[a-zA-Z_][a-zA-Z0-9_]*)*
        return !path.is_empty()
            && path.split('.').all(|seg| {
                !seg.is_empty()
                    && seg.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
                    && seg.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            });
    }

    // Double-brace JSON: the outer braces are already stripped by our caller,
    // so if the expression starts with { it was originally ={{...}}
    if expr.starts_with('{') && expr.ends_with('}') {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_imports() {
        let input = "import { Component } from 'react'\n\nHello world";
        let (output, warnings) = convert_mdx_to_rdx(input);
        assert_eq!(output.trim(), "Hello world");
        assert_eq!(warnings.len(), 0);
    }

    #[test]
    fn test_jsx_comment_conversion() {
        let input = "Some text {/* JSX comment */} more text";
        let (output, _) = convert_mdx_to_rdx(input);
        assert!(output.contains("<!-- JSX comment -->"));
    }

    #[test]
    fn test_classname_to_class() {
        let input = r#"<div className="container">Text</div>"#;
        let (output, _) = convert_mdx_to_rdx(input);
        assert!(output.contains("class="));
        assert!(!output.contains("className="));
    }

    #[test]
    fn test_multiline_import() {
        let input = "import { something }\nfrom 'module'\n\nContent";
        let (output, _) = convert_mdx_to_rdx(input);
        // The output should not contain the import lines
        assert!(!output.contains("import"));
    }

    #[test]
    fn test_attribute_expression_warning() {
        let input = r#"<button onClick={handleClick}>Click</button>"#;
        let (_output, warnings) = convert_mdx_to_rdx(input);
        assert!(!warnings.is_empty());
        assert!(warnings[0].message.contains("onClick"));
    }

    #[test]
    fn test_preserve_rdx_valid_attributes() {
        let input = r#"<Badge active={true} count={42} data={$frontmatter.title} />"#;
        let (output, warnings) = convert_mdx_to_rdx(input);
        assert!(
            warnings.is_empty(),
            "Should not warn on valid RDX attributes"
        );
        assert!(output.contains("active={true}"));
        assert!(output.contains("count={42}"));
        assert!(output.contains("data={$frontmatter.title}"));
    }
}
