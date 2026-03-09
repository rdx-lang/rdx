/// Extract YAML frontmatter from the document.
///
/// Per spec 2.1: Opening `---` MUST begin at line 1, column 1. Closing `---`
/// MUST be the sole non-whitespace content on its line, followed by newline or EOF.
/// Any `---` after line 1 that isn't the closing delimiter is a thematic break.
pub(crate) fn extract_frontmatter(input: &str) -> (Option<serde_json::Value>, usize) {
    let opener = if input.starts_with("---\n") {
        4
    } else if input.starts_with("---\r\n") {
        5
    } else {
        return (None, 0);
    };

    let rest = &input[opener..];
    let mut pos = 0;

    while pos < rest.len() {
        if let Some(nl) = rest[pos..].find('\n') {
            let line_start = pos;
            let line_end = pos + nl;
            let line = rest[line_start..line_end].trim_end_matches('\r');
            if line.trim() == "---" && line.trim_start().len() == 3 {
                let yaml_content = &rest[..line_start];
                let body_start = opener + line_end + 1;
                let frontmatter = serde_saphyr::from_str::<serde_json::Value>(yaml_content)
                    .ok()
                    .filter(|v| !v.is_null());
                return (frontmatter, body_start);
            }
            pos = line_end + 1;
        } else {
            // Last line without trailing newline
            let line = rest[pos..].trim_end_matches('\r');
            if line.trim() == "---" && line.trim_start().len() == 3 {
                let yaml_content = &rest[..pos];
                let body_start = input.len();
                let frontmatter = serde_saphyr::from_str::<serde_json::Value>(yaml_content)
                    .ok()
                    .filter(|v| !v.is_null());
                return (frontmatter, body_start);
            }
            break;
        }
    }
    (None, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_frontmatter() {
        let (fm, offset) = extract_frontmatter("---\ntitle: Hello\n---\nBody\n");
        assert!(fm.is_some());
        assert_eq!(fm.unwrap()["title"], "Hello");
        assert_eq!(&"---\ntitle: Hello\n---\nBody\n"[offset..], "Body\n");
    }

    #[test]
    fn no_frontmatter_when_not_at_start() {
        let (fm, _) = extract_frontmatter("\n---\ntitle: Hello\n---\n");
        assert!(fm.is_none());
    }

    #[test]
    fn frontmatter_at_eof() {
        let (fm, offset) = extract_frontmatter("---\nfoo: bar\n---");
        assert!(fm.is_some());
        assert_eq!(offset, 16); // entire input consumed
    }

    #[test]
    fn no_closing_delimiter() {
        let (fm, _) = extract_frontmatter("---\ntitle: Hello\nno closing\n");
        assert!(fm.is_none());
    }

    #[test]
    fn crlf_frontmatter() {
        let (fm, _) = extract_frontmatter("---\r\ntitle: Hi\r\n---\r\nBody");
        assert!(fm.is_some());
    }
}
