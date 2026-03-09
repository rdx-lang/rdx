use rdx_ast::*;

use crate::attributes::{self, AttrError};
use crate::source_map::SourceMap;

/// A parsed opening or self-closing tag with position info and attributes.
#[derive(Debug, Clone)]
pub(crate) struct ParsedTag {
    pub name: String,
    pub attributes: Vec<AttributeNode>,
    pub self_closing: bool,
    pub start: usize, // absolute byte offset of `<`
    pub end: usize,   // absolute byte offset after `>` or `/>`
}

/// Try to parse an opening or self-closing tag at `pos` in `input`.
/// `pos` must point to `<`. `base_offset` is added for absolute offsets.
/// Returns Ok(Some((tag, next_pos))) on success, Ok(None) if not a valid tag,
/// or Err with error info for malformed attributes.
pub(crate) fn try_parse_open_tag(
    input: &str,
    pos: usize,
    base_offset: usize,
    sm: &SourceMap,
) -> Result<Option<(ParsedTag, usize)>, AttrError> {
    let bytes = input.as_bytes();
    if pos >= input.len() || bytes[pos] != b'<' {
        return Ok(None);
    }
    let mut i = pos + 1;
    if i >= input.len() || !bytes[i].is_ascii_uppercase() {
        return Ok(None);
    }

    // Parse tag name: [A-Z][a-zA-Z0-9_]* (spec 2.2.1)
    let name_start = i;
    while i < input.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
        i += 1;
    }
    let name = input[name_start..i].to_string();

    // Parse attributes and closing bracket
    let (attributes, self_closing, end_pos) =
        attributes::parse_tag_rest(input, i, base_offset, sm, pos)?;

    Ok(Some((
        ParsedTag {
            name,
            attributes,
            self_closing,
            start: base_offset + pos,
            end: base_offset + end_pos,
        },
        end_pos,
    )))
}

/// Try to parse a closing tag `</Name>` at `pos`. Returns Some((name, end_pos)).
pub(crate) fn try_parse_close_tag(input: &str, pos: usize) -> Option<(String, usize)> {
    let bytes = input.as_bytes();
    if pos + 2 >= input.len() || bytes[pos] != b'<' || bytes[pos + 1] != b'/' {
        return None;
    }
    let mut i = pos + 2;
    if i >= input.len() || !bytes[i].is_ascii_uppercase() {
        return None;
    }
    let name_start = i;
    while i < input.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
        i += 1;
    }
    let name = input[name_start..i].to_string();
    // Skip whitespace before >
    while i < input.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i < input.len() && bytes[i] == b'>' {
        Some((name, i + 1))
    } else {
        None
    }
}
