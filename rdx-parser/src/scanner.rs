use crate::source_map::SourceMap;
use crate::tags::{self, ParsedTag};

/// A segment of the document body: either Markdown or a block-level component.
#[derive(Debug)]
pub(crate) enum Segment {
    Markdown {
        start: usize,
        end: usize,
    },
    BlockComponent {
        tag: ParsedTag,
        body_start: usize,
        body_end: usize,
        close_end: usize,
    },
    BlockSelfClosing {
        tag: ParsedTag,
    },
    MathBlock {
        value_start: usize,
        value_end: usize,
        block_end: usize,
        /// Optional label from `{#identifier}` on the opening `$$` line.
        label: Option<String>,
    },
    Error {
        message: String,
        raw: String,
        start: usize,
        end: usize,
    },
}

/// Scan body for top-level block component regions.
/// Block components are those where the tag is the sole non-whitespace
/// content on its line, per spec 2.2.4.
/// All offsets in returned segments are absolute (base_offset already added).
pub(crate) fn scan_segments(body: &str, base_offset: usize, sm: &SourceMap) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut md_start = 0usize; // relative to body
    let bytes = body.as_bytes();
    let mut pos = 0;
    let mut in_code_fence: Option<(u8, usize)> = None; // (fence_char, count) for opening fence

    while pos < body.len() {
        let line_start = pos;

        // Skip leading whitespace on this line
        let mut content_pos = pos;
        while content_pos < body.len()
            && (bytes[content_pos] == b' ' || bytes[content_pos] == b'\t')
        {
            content_pos += 1;
        }

        // Track fenced code blocks: ``` or ~~~ (3+ chars)
        if content_pos < body.len() {
            let fence_char = bytes[content_pos];
            if fence_char == b'`' || fence_char == b'~' {
                let fence_start = content_pos;
                let mut fence_count = 0;
                while content_pos < body.len() && bytes[content_pos] == fence_char {
                    fence_count += 1;
                    content_pos += 1;
                }
                if fence_count >= 3 {
                    if let Some((open_char, open_count)) = in_code_fence {
                        // Closing fence must use the same character and be >= same count
                        if fence_char == open_char
                            && is_whitespace_until_eol(body, content_pos)
                            && fence_count >= open_count
                        {
                            in_code_fence = None;
                        }
                    } else {
                        in_code_fence = Some((fence_char, fence_count));
                    }
                    // Skip to next line
                    while pos < body.len() && bytes[pos] != b'\n' {
                        pos += 1;
                    }
                    if pos < body.len() {
                        pos += 1;
                    }
                    continue;
                }
                // Reset content_pos if fence was < 3
                content_pos = fence_start;
                while content_pos < body.len()
                    && (bytes[content_pos] == b' ' || bytes[content_pos] == b'\t')
                {
                    content_pos += 1;
                }
            }
        }

        // Detect display math blocks: $$ on its own line
        if content_pos + 1 < body.len()
            && bytes[content_pos] == b'$'
            && bytes[content_pos + 1] == b'$'
            && in_code_fence.is_none()
        {
            // After $$, may have optional {#label} before end of line
            let after_dollars = content_pos + 2;
            let (label, line_rest_start) = extract_math_label(body, after_dollars);
            if is_whitespace_until_eol(body, line_rest_start) {
                // Find closing $$
                let search_start = skip_to_next_line(body, after_dollars);
                if let Some((close_line_start, close_end)) = find_math_close(body, search_start) {
                    flush_markdown(&mut segments, md_start, line_start, base_offset);
                    let value_start = skip_to_next_line(body, after_dollars);
                    segments.push(Segment::MathBlock {
                        value_start: base_offset + value_start,
                        value_end: base_offset + close_line_start,
                        block_end: base_offset + close_end,
                        label,
                    });
                    pos = skip_to_next_line(body, close_end);
                    md_start = pos;
                    continue;
                }
            }
        }

        // If inside a fenced code block, skip this line entirely
        if in_code_fence.is_some() {
            while pos < body.len() && bytes[pos] != b'\n' {
                pos += 1;
            }
            if pos < body.len() {
                pos += 1;
            }
            continue;
        }

        // Check for uppercase component tag at start of line
        if content_pos < body.len() && bytes[content_pos] == b'<' {
            if content_pos + 1 < body.len() && bytes[content_pos + 1].is_ascii_uppercase() {
                // Try opening/self-closing tag
                match tags::try_parse_open_tag(body, content_pos, base_offset, sm) {
                    Ok(Some((tag, tag_end))) => {
                        let rest_is_ws = is_whitespace_until_eol(body, tag_end);
                        if tag.self_closing && rest_is_ws {
                            flush_markdown(&mut segments, md_start, line_start, base_offset);
                            segments.push(Segment::BlockSelfClosing { tag });
                            pos = skip_to_next_line(body, tag_end);
                            md_start = pos;
                            continue;
                        } else if !tag.self_closing && rest_is_ws {
                            // Opening tag on its own line — find matching close on a later line
                            let tag_name = tag.name.clone();
                            let search_start = skip_to_next_line(body, tag_end);
                            match find_matching_close(
                                body,
                                search_start,
                                &tag_name,
                                base_offset,
                                sm,
                            ) {
                                Ok((close_line_start, close_tag_end)) => {
                                    flush_markdown(
                                        &mut segments,
                                        md_start,
                                        line_start,
                                        base_offset,
                                    );
                                    let bs = skip_to_next_line(body, tag_end).min(body.len());
                                    segments.push(Segment::BlockComponent {
                                        tag,
                                        body_start: base_offset + bs,
                                        body_end: base_offset + close_line_start,
                                        close_end: base_offset + close_tag_end,
                                    });
                                    pos = skip_to_next_line(body, close_tag_end);
                                    md_start = pos;
                                    continue;
                                }
                                Err(msg) => {
                                    flush_markdown(
                                        &mut segments,
                                        md_start,
                                        line_start,
                                        base_offset,
                                    );
                                    segments.push(Segment::Error {
                                        message: msg,
                                        raw: body[content_pos..].to_string(),
                                        start: base_offset + content_pos,
                                        end: base_offset + body.len(),
                                    });
                                    pos = body.len();
                                    md_start = pos;
                                    continue;
                                }
                            }
                        } else if !tag.self_closing && !rest_is_ws {
                            // Opening tag with content on same line — check for close tag on same line
                            let close_tag_str = format!("</{}>", tag.name);
                            let line_end = find_line_end(body, tag_end);
                            let rest_of_line = &body[tag_end..line_end];
                            if let Some(close_rel) = rest_of_line.find(&close_tag_str) {
                                let close_start = tag_end + close_rel;
                                let close_end = close_start + close_tag_str.len();
                                if is_whitespace_until_eol(body, close_end) {
                                    flush_markdown(
                                        &mut segments,
                                        md_start,
                                        line_start,
                                        base_offset,
                                    );
                                    segments.push(Segment::BlockComponent {
                                        tag,
                                        body_start: base_offset + tag_end,
                                        body_end: base_offset + close_start,
                                        close_end: base_offset + close_end,
                                    });
                                    pos = skip_to_next_line(body, close_end);
                                    md_start = pos;
                                    continue;
                                }
                            }
                            // Not a single-line component — fall through to markdown
                        }
                    }
                    Ok(None) => {} // not a valid tag, fall through to markdown
                    Err(attr_err) => {
                        // Attribute parse error (malformed JSON, invalid var path, etc.)
                        flush_markdown(&mut segments, md_start, line_start, base_offset);
                        segments.push(Segment::Error {
                            message: attr_err.message,
                            raw: attr_err.raw,
                            start: attr_err.start,
                            end: attr_err.end,
                        });
                        // Skip past the problematic line
                        pos = skip_to_next_line(body, content_pos);
                        md_start = pos;
                        continue;
                    }
                }
            } else if content_pos + 2 < body.len()
                && bytes[content_pos + 1] == b'/'
                && bytes[content_pos + 2].is_ascii_uppercase()
            {
                // Stray closing tag at top level
                if let Some((name, tag_end)) = tags::try_parse_close_tag(body, content_pos)
                    && is_whitespace_until_eol(body, tag_end)
                {
                    flush_markdown(&mut segments, md_start, line_start, base_offset);
                    segments.push(Segment::Error {
                        message: format!("Unexpected closing tag </{}>", name),
                        raw: body[content_pos..tag_end].to_string(),
                        start: base_offset + content_pos,
                        end: base_offset + tag_end,
                    });
                    pos = skip_to_next_line(body, tag_end);
                    md_start = pos;
                    continue;
                }
            }
        }

        // Advance to next line
        while pos < body.len() && bytes[pos] != b'\n' {
            pos += 1;
        }
        if pos < body.len() {
            pos += 1;
        }
    }

    flush_markdown(&mut segments, md_start, body.len(), base_offset);
    segments
}

fn is_whitespace_until_eol(input: &str, pos: usize) -> bool {
    let bytes = input.as_bytes();
    let mut i = pos;
    while i < input.len() && bytes[i] != b'\n' {
        if bytes[i] != b' ' && bytes[i] != b'\t' && bytes[i] != b'\r' {
            return false;
        }
        i += 1;
    }
    true
}

fn find_line_end(input: &str, pos: usize) -> usize {
    let bytes = input.as_bytes();
    let mut i = pos;
    while i < input.len() && bytes[i] != b'\n' {
        i += 1;
    }
    i
}

fn skip_to_next_line(input: &str, pos: usize) -> usize {
    let bytes = input.as_bytes();
    let mut i = pos;
    while i < input.len() && bytes[i] != b'\n' {
        i += 1;
    }
    if i < input.len() { i + 1 } else { i }
}

fn flush_markdown(segments: &mut Vec<Segment>, md_start: usize, md_end: usize, base_offset: usize) {
    if md_start < md_end {
        segments.push(Segment::Markdown {
            start: base_offset + md_start,
            end: base_offset + md_end,
        });
    }
}

/// Extract a `{#identifier}` label from position `pos` in `input`.
///
/// Returns `(Some(label), pos_after_label)` if found, or `(None, pos)` if not.
/// Identifier must match `[a-zA-Z_][a-zA-Z0-9_:.-]*`.
fn extract_math_label(input: &str, pos: usize) -> (Option<String>, usize) {
    let bytes = input.as_bytes();
    // Skip whitespace before {#
    let mut i = pos;
    while i < input.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    if i + 1 >= input.len() || bytes[i] != b'{' || bytes[i + 1] != b'#' {
        return (None, pos);
    }
    let id_start = i + 2;
    let mut j = id_start;
    // First char: [a-zA-Z_]
    if j >= input.len() || (!bytes[j].is_ascii_alphabetic() && bytes[j] != b'_') {
        return (None, pos);
    }
    j += 1;
    // Rest: [a-zA-Z0-9_:.-]*
    while j < input.len()
        && (bytes[j].is_ascii_alphanumeric()
            || bytes[j] == b'_'
            || bytes[j] == b':'
            || bytes[j] == b'.'
            || bytes[j] == b'-')
    {
        j += 1;
    }
    if j >= input.len() || bytes[j] != b'}' {
        return (None, pos);
    }
    let label = input[id_start..j].to_string();
    (Some(label), j + 1)
}

/// Find a closing `$$` on its own line. Returns (line_start, end_of_dollars).
fn find_math_close(input: &str, start: usize) -> Option<(usize, usize)> {
    let bytes = input.as_bytes();
    let mut pos = start;
    while pos < input.len() {
        let line_start = pos;
        let mut content_pos = pos;
        while content_pos < input.len()
            && (bytes[content_pos] == b' ' || bytes[content_pos] == b'\t')
        {
            content_pos += 1;
        }
        if content_pos + 1 < input.len()
            && bytes[content_pos] == b'$'
            && bytes[content_pos + 1] == b'$'
            && is_whitespace_until_eol(input, content_pos + 2)
        {
            return Some((line_start, content_pos + 2));
        }
        while pos < input.len() && bytes[pos] != b'\n' {
            pos += 1;
        }
        if pos < input.len() {
            pos += 1;
        }
    }
    None
}

/// Find the matching close tag for `tag_name` using strict LIFO matching (spec 2.2.3).
/// Only considers tags that are sole content on their line (block-level).
fn find_matching_close(
    input: &str,
    start: usize,
    tag_name: &str,
    base_offset: usize,
    sm: &SourceMap,
) -> Result<(usize, usize), String> {
    let bytes = input.as_bytes();
    let mut pos = start;
    let mut depth = 1u32;

    while pos < input.len() {
        let line_start = pos;
        let mut content_pos = pos;
        while content_pos < input.len()
            && (bytes[content_pos] == b' ' || bytes[content_pos] == b'\t')
        {
            content_pos += 1;
        }

        if content_pos < input.len() && bytes[content_pos] == b'<' {
            // Closing tag
            if content_pos + 2 < input.len()
                && bytes[content_pos + 1] == b'/'
                && bytes[content_pos + 2].is_ascii_uppercase()
            {
                if let Some((name, tag_end)) = tags::try_parse_close_tag(input, content_pos)
                    && name == tag_name
                    && is_whitespace_until_eol(input, tag_end)
                {
                    depth -= 1;
                    if depth == 0 {
                        return Ok((line_start, tag_end));
                    }
                }
            }
            // Opening tag with same name (nested same-type)
            else if content_pos + 1 < input.len()
                && bytes[content_pos + 1].is_ascii_uppercase()
                && let Ok(Some((tag, _))) =
                    tags::try_parse_open_tag(input, content_pos, base_offset, sm)
                && tag.name == tag_name
                && !tag.self_closing
                && is_whitespace_until_eol(input, tag.end - base_offset)
            {
                depth += 1;
            }
        }

        while pos < input.len() && bytes[pos] != b'\n' {
            pos += 1;
        }
        if pos < input.len() {
            pos += 1;
        }
    }

    Err(format!("Unclosed tag <{}>", tag_name))
}
