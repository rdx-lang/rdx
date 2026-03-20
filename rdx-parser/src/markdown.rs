use rdx_ast::*;

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag};

use crate::source_map::SourceMap;
use crate::tags;
use crate::text;

/// A stack frame during pulldown-cmark event processing.
struct Frame {
    kind: FrameKind,
    children: Vec<Node>,
    start_offset: usize,
    code_text: String,
}

enum FrameKind {
    Paragraph,
    Heading {
        level: u8,
        id: Option<String>,
    },
    List(bool),
    ListItem {
        checked: Option<bool>,
    },
    Blockquote,
    Table,
    TableRow,
    TableCell,
    Emphasis,
    Strong,
    Strikethrough,
    Link {
        url: String,
        title: Option<String>,
    },
    Image {
        url: String,
        title: Option<String>,
    },
    CodeBlock {
        lang: Option<String>,
        meta: Option<String>,
    },
    FootnoteDefinition {
        label: String,
    },
    HtmlBlock,
    DefinitionList,
    DefinitionTerm,
    DefinitionDescription,
}

fn std_block(children: Vec<Node>, pos: Position) -> StandardBlockNode {
    StandardBlockNode {
        depth: None,
        ordered: None,
        checked: None,
        id: None,
        children,
        position: pos,
    }
}

impl Frame {
    fn new(kind: FrameKind, start_offset: usize) -> Self {
        Frame {
            kind,
            children: Vec::new(),
            start_offset,
            code_text: String::new(),
        }
    }

    fn into_node(self, end_offset: usize, sm: &SourceMap) -> Node {
        let pos = sm.position(self.start_offset, end_offset);
        match self.kind {
            FrameKind::Paragraph => Node::Paragraph(std_block(self.children, pos)),
            FrameKind::Heading { level, id } => Node::Heading(StandardBlockNode {
                depth: Some(level),
                ordered: None,
                checked: None,
                id,
                children: self.children,
                position: pos,
            }),
            FrameKind::List(o) => Node::List(StandardBlockNode {
                depth: None,
                ordered: Some(o),
                checked: None,
                id: None,
                children: self.children,
                position: pos,
            }),
            FrameKind::ListItem { checked } => Node::ListItem(StandardBlockNode {
                depth: None,
                ordered: None,
                checked,
                id: None,
                children: self.children,
                position: pos,
            }),
            FrameKind::Blockquote => Node::Blockquote(std_block(self.children, pos)),
            FrameKind::Table => Node::Table(std_block(self.children, pos)),
            FrameKind::TableRow => Node::TableRow(std_block(self.children, pos)),
            FrameKind::TableCell => Node::TableCell(std_block(self.children, pos)),
            FrameKind::Emphasis => Node::Emphasis(std_block(self.children, pos)),
            FrameKind::Strong => Node::Strong(std_block(self.children, pos)),
            FrameKind::Strikethrough => Node::Strikethrough(std_block(self.children, pos)),
            FrameKind::Link { url, title } => Node::Link(LinkNode {
                url,
                title,
                children: self.children,
                position: pos,
            }),
            FrameKind::Image { url, title } => Node::Image(ImageNode {
                url,
                title,
                alt: None,
                children: self.children,
                position: pos,
            }),
            FrameKind::CodeBlock { lang, meta } => {
                let CodeMeta {
                    title: parsed_title,
                    highlight: parsed_highlight,
                    show_line_numbers: parsed_show_line_numbers,
                    diff: parsed_diff,
                    caption: parsed_caption,
                    remaining_meta,
                } = meta.as_deref().map(parse_code_meta).unwrap_or_default();
                let final_meta = remaining_meta.or(
                    if parsed_title.is_some()
                        || parsed_highlight.is_some()
                        || parsed_show_line_numbers.is_some()
                        || parsed_diff.is_some()
                        || parsed_caption.is_some()
                    {
                        None
                    } else {
                        meta
                    },
                );
                Node::CodeBlock(CodeBlockNode {
                    value: self.code_text,
                    lang,
                    meta: final_meta,
                    title: parsed_title,
                    highlight: parsed_highlight,
                    show_line_numbers: parsed_show_line_numbers,
                    diff: parsed_diff,
                    caption: parsed_caption,
                    position: pos,
                })
            }
            FrameKind::FootnoteDefinition { label } => Node::FootnoteDefinition(FootnoteNode {
                label,
                children: self.children,
                position: pos,
            }),
            FrameKind::HtmlBlock => Node::Html(std_block(self.children, pos)),
            FrameKind::DefinitionList => Node::DefinitionList(std_block(self.children, pos)),
            FrameKind::DefinitionTerm => Node::DefinitionTerm(std_block(self.children, pos)),
            FrameKind::DefinitionDescription => {
                Node::DefinitionDescription(std_block(self.children, pos))
            }
        }
    }
}

fn heading_level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn is_code_frame(stack: &[Frame]) -> bool {
    stack
        .iter()
        .any(|f| matches!(f.kind, FrameKind::CodeBlock { .. }))
}

/// Parse a markdown-only region using pulldown-cmark and convert to AST nodes.
/// Handles inline RDX components that appear as InlineHtml events.
pub(crate) fn parse_markdown_region(
    text: &str,
    base_offset: usize,
    sm: &SourceMap,
    full_input: &str,
) -> Vec<Node> {
    if text.trim().is_empty() {
        return vec![];
    }

    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_DEFINITION_LIST);

    let parser = Parser::new_ext(text, opts);
    let iter = parser.into_offset_iter();

    let mut stack: Vec<Frame> = Vec::new();
    let mut result: Vec<Node> = Vec::new();
    let mut comp_stack: Vec<(tags::ParsedTag, Vec<Node>)> = Vec::new();

    for (event, range) in iter {
        let abs_start = range.start + base_offset;
        let abs_end = range.end + base_offset;

        match event {
            Event::Start(tag) => {
                let frame = match tag {
                    Tag::Paragraph => Frame::new(FrameKind::Paragraph, abs_start),
                    Tag::Heading { level, id, .. } => Frame::new(
                        FrameKind::Heading {
                            level: heading_level_to_u8(level),
                            id: id.map(|s| s.to_string()),
                        },
                        abs_start,
                    ),
                    Tag::List(start_num) => {
                        Frame::new(FrameKind::List(start_num.is_some()), abs_start)
                    }
                    Tag::Item => Frame::new(FrameKind::ListItem { checked: None }, abs_start),
                    Tag::BlockQuote(_) => Frame::new(FrameKind::Blockquote, abs_start),
                    Tag::Table(_) => Frame::new(FrameKind::Table, abs_start),
                    Tag::TableHead | Tag::TableRow => Frame::new(FrameKind::TableRow, abs_start),
                    Tag::TableCell => Frame::new(FrameKind::TableCell, abs_start),
                    Tag::Emphasis => Frame::new(FrameKind::Emphasis, abs_start),
                    Tag::Strong => Frame::new(FrameKind::Strong, abs_start),
                    Tag::Strikethrough => Frame::new(FrameKind::Strikethrough, abs_start),
                    Tag::Link {
                        dest_url, title, ..
                    } => Frame::new(
                        FrameKind::Link {
                            url: dest_url.to_string(),
                            title: if title.is_empty() {
                                None
                            } else {
                                Some(title.to_string())
                            },
                        },
                        abs_start,
                    ),
                    Tag::Image {
                        dest_url, title, ..
                    } => Frame::new(
                        FrameKind::Image {
                            url: dest_url.to_string(),
                            title: if title.is_empty() {
                                None
                            } else {
                                Some(title.to_string())
                            },
                        },
                        abs_start,
                    ),
                    Tag::CodeBlock(kind) => {
                        let (lang, meta) = match kind {
                            CodeBlockKind::Fenced(info) => {
                                let info = info.to_string();
                                if info.is_empty() {
                                    (None, None)
                                } else if let Some((l, m)) = info.split_once(' ') {
                                    (Some(l.to_string()), Some(m.to_string()))
                                } else {
                                    (Some(info), None)
                                }
                            }
                            CodeBlockKind::Indented => (None, None),
                        };
                        Frame::new(FrameKind::CodeBlock { lang, meta }, abs_start)
                    }
                    Tag::FootnoteDefinition(label) => Frame::new(
                        FrameKind::FootnoteDefinition {
                            label: label.to_string(),
                        },
                        abs_start,
                    ),
                    Tag::HtmlBlock => Frame::new(FrameKind::HtmlBlock, abs_start),
                    Tag::DefinitionList => Frame::new(FrameKind::DefinitionList, abs_start),
                    Tag::DefinitionListTitle => Frame::new(FrameKind::DefinitionTerm, abs_start),
                    Tag::DefinitionListDefinition => {
                        Frame::new(FrameKind::DefinitionDescription, abs_start)
                    }
                    _ => continue,
                };
                stack.push(frame);
            }

            Event::End(_) => {
                if let Some(frame) = stack.pop() {
                    let node = frame.into_node(abs_end, sm);
                    push_node(&mut stack, &mut result, &mut comp_stack, node);
                }
            }

            Event::Text(ref cow_text) => {
                if is_code_frame(&stack) {
                    if let Some(frame) = stack.last_mut() {
                        frame.code_text.push_str(cow_text);
                    }
                } else {
                    // Use raw source to preserve RDX escape sequences
                    // pulldown-cmark strips backslash escapes (\{ -> {), but we
                    // need the original backslash for RDX escape handling.
                    let effective_start = if abs_start > 0
                        && full_input.as_bytes().get(abs_start - 1) == Some(&b'\\')
                    {
                        abs_start - 1
                    } else {
                        abs_start
                    };
                    let raw_text = if effective_start < abs_end && abs_end <= full_input.len() {
                        &full_input[effective_start..abs_end]
                    } else {
                        cow_text.as_ref()
                    };
                    let text_nodes = text::process_text(raw_text, effective_start, sm, false);
                    for node in text_nodes {
                        push_node(&mut stack, &mut result, &mut comp_stack, node);
                    }
                }
            }

            Event::Code(ref code) => {
                let node = Node::CodeInline(CodeInlineNode {
                    value: code.to_string(),
                    lang: None,
                    position: sm.position(abs_start, abs_end),
                });
                push_node(&mut stack, &mut result, &mut comp_stack, node);
            }

            Event::Html(ref html_text) | Event::InlineHtml(ref html_text) => {
                let is_inline_html = matches!(event, Event::InlineHtml(_));
                let html_str = html_text.to_string();

                // Try to parse all RDX components from this HTML event.
                // A single HTML block event may contain multiple component lines.
                let comp_stack_len = comp_stack.len();
                let nodes = parse_html_for_components(
                    &html_str,
                    is_inline_html,
                    abs_start,
                    abs_end,
                    sm,
                    &mut comp_stack,
                );
                if !nodes.is_empty() {
                    for node in nodes {
                        push_node(&mut stack, &mut result, &mut comp_stack, node);
                    }
                    continue;
                }
                // Open tag was pushed to comp_stack (no nodes emitted yet)
                if comp_stack.len() > comp_stack_len {
                    continue;
                }

                // Check if component close tag
                let trimmed = html_str.trim();
                if try_handle_close_tag(
                    trimmed,
                    &html_str,
                    abs_start,
                    abs_end,
                    sm,
                    full_input,
                    &mut stack,
                    &mut result,
                    &mut comp_stack,
                ) {
                    continue;
                }

                // Regular HTML — add to current HtmlBlock frame or emit standalone
                if let Some(frame) = stack.last_mut()
                    && matches!(frame.kind, FrameKind::HtmlBlock)
                {
                    frame.children.push(Node::Text(TextNode {
                        value: html_str,
                        position: sm.position(abs_start, abs_end),
                    }));
                    continue;
                }
                let node = Node::Html(std_block(
                    vec![Node::Text(TextNode {
                        value: html_str,
                        position: sm.position(abs_start, abs_end),
                    })],
                    sm.position(abs_start, abs_end),
                ));
                push_node(&mut stack, &mut result, &mut comp_stack, node);
            }

            Event::TaskListMarker(checked) => {
                // Set the checked field on the current ListItem frame
                if let Some(frame) = stack.last_mut()
                    && let FrameKind::ListItem { checked: ref mut c } = frame.kind
                {
                    *c = Some(checked);
                }
            }

            Event::FootnoteReference(label) => {
                let node = Node::FootnoteReference(FootnoteNode {
                    label: label.to_string(),
                    children: vec![],
                    position: sm.position(abs_start, abs_end),
                });
                push_node(&mut stack, &mut result, &mut comp_stack, node);
            }

            Event::InlineMath(ref math_text) => {
                let raw = math_text.to_string();
                let tree = rdx_math::parse(&raw);
                let node = Node::MathInline(MathNode {
                    raw,
                    tree,
                    position: sm.position(abs_start, abs_end),
                });
                push_node(&mut stack, &mut result, &mut comp_stack, node);
            }

            Event::DisplayMath(ref math_text) => {
                let raw = math_text.to_string();
                let tree = rdx_math::parse(&raw);
                let node = Node::MathDisplay(MathDisplayNode {
                    raw,
                    tree,
                    label: None,
                    position: sm.position(abs_start, abs_end),
                });
                push_node(&mut stack, &mut result, &mut comp_stack, node);
            }

            Event::SoftBreak | Event::HardBreak => {
                let node = Node::Text(TextNode {
                    value: "\n".to_string(),
                    position: sm.position(abs_start, abs_end),
                });
                push_node(&mut stack, &mut result, &mut comp_stack, node);
            }

            Event::Rule => {
                let node = Node::ThematicBreak(std_block(vec![], sm.position(abs_start, abs_end)));
                push_node(&mut stack, &mut result, &mut comp_stack, node);
            }
        }
    }

    // Handle unclosed components in the markdown region (spec 3.2)
    while let Some((open_tag, children)) = comp_stack.pop() {
        let err = Node::Error(ErrorNode {
            message: format!("Unclosed tag <{}>", open_tag.name),
            raw_content: format!("<{}>", open_tag.name),
            position: sm.position(open_tag.start, open_tag.start + open_tag.name.len() + 2),
        });
        result.push(err);
        result.extend(children);
    }

    // Post-process: merge adjacent Text nodes and detect citations/cross-refs
    let result = merge_and_process_text_nodes(result, sm, full_input);

    // Post-process: attach inline code language hints (step 8)
    apply_inline_code_lang_hints(result)
}

/// Parse an HTML event for RDX components.
/// A single HTML block may contain multiple component lines, e.g.:
/// ```html
///   <NavItem href="/a">Intro</NavItem>
///   <NavItem href="/b">API</NavItem>
/// ```
/// Returns a vec of parsed component nodes (empty if no RDX components found).
fn parse_html_for_components(
    html_str: &str,
    is_inline: bool,
    abs_start: usize,
    _abs_end: usize,
    sm: &SourceMap,
    comp_stack: &mut Vec<(tags::ParsedTag, Vec<Node>)>,
) -> Vec<Node> {
    let mut nodes = Vec::new();
    let mut pos = 0;
    let bytes = html_str.as_bytes();
    let mut found_any = false;

    while pos < html_str.len() {
        // Skip whitespace
        while pos < html_str.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= html_str.len() {
            break;
        }

        // Look for `<` followed by uppercase letter
        if bytes[pos] == b'<' && pos + 1 < html_str.len() && bytes[pos + 1].is_ascii_uppercase() {
            let tag_start_abs = abs_start + pos;
            match tags::try_parse_open_tag(html_str, pos, abs_start, sm) {
                Ok(Some((parsed_tag, tag_end))) => {
                    found_any = true;
                    if parsed_tag.self_closing {
                        // Find end of line for position
                        let line_end = html_str[tag_end..]
                            .find('\n')
                            .map(|i| tag_end + i)
                            .unwrap_or(html_str.len());
                        nodes.push(Node::Component(ComponentNode {
                            name: parsed_tag.name,
                            is_inline,
                            attributes: parsed_tag.attributes,
                            children: vec![],
                            raw_content: String::new(),
                            position: sm.position(tag_start_abs, abs_start + line_end),
                        }));
                        pos = line_end;
                    } else {
                        // Look for matching close tag in remaining text
                        let close_tag_str = format!("</{}>", parsed_tag.name);
                        if let Some(close_rel) = html_str[tag_end..].find(&close_tag_str) {
                            let close_pos = tag_end + close_rel;
                            let close_end = close_pos + close_tag_str.len();
                            let body_text = &html_str[tag_end..close_pos];
                            let body_offset = abs_start + tag_end;
                            let children = if body_text.trim().is_empty() {
                                vec![]
                            } else {
                                text::process_text(body_text, body_offset, sm, false)
                            };
                            nodes.push(Node::Component(ComponentNode {
                                name: parsed_tag.name,
                                is_inline,
                                attributes: parsed_tag.attributes,
                                children,
                                raw_content: body_text.to_string(),
                                position: sm.position(tag_start_abs, abs_start + close_end),
                            }));
                            pos = close_end;
                        } else {
                            // Opening tag only — push to comp_stack
                            let tag = tags::ParsedTag {
                                name: parsed_tag.name,
                                attributes: parsed_tag.attributes,
                                self_closing: false,
                                start: tag_start_abs,
                                end: abs_start + tag_end,
                            };
                            comp_stack.push((tag, Vec::new()));
                            pos = tag_end;
                        }
                    }
                    continue;
                }
                Ok(None) => {}
                Err(attr_err) => {
                    found_any = true;
                    nodes.push(Node::Error(ErrorNode {
                        message: attr_err.message,
                        raw_content: attr_err.raw,
                        position: sm.position(attr_err.start, attr_err.end),
                    }));
                }
            }
        } else if bytes[pos] == b'<'
            && pos + 2 < html_str.len()
            && bytes[pos + 1] == b'/'
            && bytes[pos + 2].is_ascii_uppercase()
        {
            // Close tag in HTML block — handled by try_handle_close_tag later
            break;
        }

        // Skip to next line or next `<`
        if let Some(next_lt) = html_str[pos + 1..].find('<') {
            pos = pos + 1 + next_lt;
        } else {
            break;
        }
    }

    if found_any { nodes } else { vec![] }
}

/// Try to handle an HTML event as an RDX closing tag.
/// Returns true if handled.
#[allow(clippy::too_many_arguments)]
fn try_handle_close_tag(
    trimmed: &str,
    html_str: &str,
    abs_start: usize,
    abs_end: usize,
    sm: &SourceMap,
    full_input: &str,
    stack: &mut Vec<Frame>,
    result: &mut Vec<Node>,
    comp_stack: &mut Vec<(tags::ParsedTag, Vec<Node>)>,
) -> bool {
    if !trimmed.starts_with("</") {
        return false;
    }
    let after_slash = &trimmed[2..];
    if after_slash.is_empty() || !after_slash.as_bytes()[0].is_ascii_uppercase() {
        return false;
    }

    if let Some((name, _)) = tags::try_parse_close_tag(trimmed, 0) {
        if let Some((open_tag, children)) = comp_stack.pop() {
            if open_tag.name == name {
                // Extract raw body text from source between open tag end and close tag start
                let raw_content = if open_tag.end <= abs_start {
                    full_input[open_tag.end..abs_start].to_string()
                } else {
                    String::new()
                };
                let node = Node::Component(ComponentNode {
                    name: open_tag.name,
                    is_inline: false,
                    attributes: open_tag.attributes,
                    children,
                    raw_content,
                    position: sm.position(open_tag.start, abs_end),
                });
                push_node(stack, result, comp_stack, node);
            } else {
                // Misnested tags (spec 3.3)
                let err = Node::Error(ErrorNode {
                    message: format!(
                        "Misnested tags: expected </{}>, found </{}>",
                        open_tag.name, name
                    ),
                    raw_content: html_str.to_string(),
                    position: sm.position(abs_start, abs_end),
                });
                push_node(stack, result, comp_stack, err);
            }
        } else {
            // Stray close tag
            let err = Node::Error(ErrorNode {
                message: format!("Unexpected closing tag </{}>", name),
                raw_content: html_str.to_string(),
                position: sm.position(abs_start, abs_end),
            });
            push_node(stack, result, comp_stack, err);
        }
        return true;
    }
    false
}

/// Merge adjacent `Text` nodes and re-process the combined raw source for citations/cross-refs.
///
/// pulldown-cmark splits citation-like syntax `[@key]` into separate Text events:
/// `[`, `@key`, `]`. This pass detects such runs and re-reads the raw source over their
/// combined span, passing it through `text::process_text` to detect `[@...]` citations.
///
/// IMPORTANT: we use the raw `full_input` source — not the already-processed text values —
/// so that escape sequences like `\[@` are preserved and correctly handled.
///
/// This is applied recursively to all node children.
pub(crate) fn merge_and_process_text_nodes(
    nodes: Vec<Node>,
    sm: &SourceMap,
    full_input: &str,
) -> Vec<Node> {
    // First recursively process children, then merge at this level
    let nodes: Vec<Node> = nodes
        .into_iter()
        .map(|n| merge_text_recursive(n, sm, full_input))
        .collect();

    merge_text_at_level(nodes, sm, full_input)
}

fn merge_text_recursive(node: Node, sm: &SourceMap, full_input: &str) -> Node {
    match node {
        Node::Paragraph(mut b) => {
            b.children = merge_and_process_text_nodes(b.children, sm, full_input);
            Node::Paragraph(b)
        }
        Node::Heading(mut b) => {
            b.children = merge_and_process_text_nodes(b.children, sm, full_input);
            Node::Heading(b)
        }
        Node::ListItem(mut b) => {
            b.children = merge_and_process_text_nodes(b.children, sm, full_input);
            Node::ListItem(b)
        }
        Node::Blockquote(mut b) => {
            b.children = merge_and_process_text_nodes(b.children, sm, full_input);
            Node::Blockquote(b)
        }
        Node::Emphasis(mut b) => {
            b.children = merge_and_process_text_nodes(b.children, sm, full_input);
            Node::Emphasis(b)
        }
        Node::Strong(mut b) => {
            b.children = merge_and_process_text_nodes(b.children, sm, full_input);
            Node::Strong(b)
        }
        Node::Strikethrough(mut b) => {
            b.children = merge_and_process_text_nodes(b.children, sm, full_input);
            Node::Strikethrough(b)
        }
        Node::TableCell(mut b) => {
            b.children = merge_and_process_text_nodes(b.children, sm, full_input);
            Node::TableCell(b)
        }
        Node::Link(mut l) => {
            l.children = merge_and_process_text_nodes(l.children, sm, full_input);
            Node::Link(l)
        }
        Node::Image(mut i) => {
            i.children = merge_and_process_text_nodes(i.children, sm, full_input);
            Node::Image(i)
        }
        other => other,
    }
}

/// Merge consecutive Text nodes at a single level and re-process for citations/cross-refs.
///
/// Uses the raw `full_input` source over the combined span of the run to correctly handle
/// backslash escapes (which may have been stripped by pulldown-cmark in the node values).
fn merge_text_at_level(nodes: Vec<Node>, sm: &SourceMap, full_input: &str) -> Vec<Node> {
    let mut result = Vec::new();
    let mut text_run: Vec<TextNode> = Vec::new();

    macro_rules! flush_run {
        () => {
            if !text_run.is_empty() {
                let start_offset = text_run[0].position.start.offset;
                let end_offset = text_run.last().unwrap().position.end.offset;

                // Check raw source region for citation/cross-ref patterns
                let raw = if start_offset < end_offset && end_offset <= full_input.len() {
                    &full_input[start_offset..end_offset]
                } else {
                    // Fallback: concatenate node values if source range is unavailable
                    ""
                };

                let needs_reprocess =
                    raw.contains("[@") || raw.contains("{@") || contains_citation_pattern(raw);

                if needs_reprocess && !raw.is_empty() {
                    // Re-process raw source text for citation/cross-ref patterns
                    let processed = text::process_text(raw, start_offset, sm, false);
                    result.extend(processed);
                } else {
                    for t in text_run.drain(..) {
                        result.push(Node::Text(t));
                    }
                }
                text_run.clear();
            }
        };
    }

    for node in nodes {
        match node {
            Node::Text(t) => {
                text_run.push(t);
            }
            other => {
                flush_run!();
                result.push(other);
            }
        }
    }
    flush_run!();
    result
}

/// Check if text contains a citation pattern: `[...@...]` where `[` is followed eventually
/// by `@` and then `]`, forming a potential `[prefix @key]` citation.
fn contains_citation_pattern(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'[' {
            // Look for @ before ]
            let mut j = i + 1;
            let mut found_at = false;
            while j < bytes.len() && bytes[j] != b']' {
                if bytes[j] == b'@' {
                    found_at = true;
                }
                j += 1;
            }
            if found_at && j < bytes.len() && bytes[j] == b']' {
                // Check it's not followed by ( (which would make it a link)
                let after = j + 1;
                if after >= bytes.len() || bytes[after] != b'(' {
                    return true;
                }
            }
        }
        i += 1;
    }
    false
}

/// Push a node to the right destination: component stack, frame stack, or result.
#[allow(clippy::ptr_arg)]
fn push_node(
    stack: &mut Vec<Frame>,
    result: &mut Vec<Node>,
    comp_stack: &mut Vec<(tags::ParsedTag, Vec<Node>)>,
    node: Node,
) {
    if let Some((_, children)) = comp_stack.last_mut() {
        children.push(node);
    } else if let Some(frame) = stack.last_mut() {
        frame.children.push(node);
    } else {
        result.push(node);
    }
}

/// Parse the meta string from a code block info string.
///
/// Structured fields extracted from a fenced code block's info string.
#[derive(Debug, Default)]
struct CodeMeta {
    title: Option<String>,
    highlight: Option<Vec<u32>>,
    show_line_numbers: Option<bool>,
    diff: Option<bool>,
    caption: Option<String>,
    remaining_meta: Option<String>,
}

/// Extracts structured fields:
/// - `title="..."` or `title='...'` — display title
/// - `{3-5,12}` — highlight lines
/// - `showLineNumbers` — flag
/// - `diff` — flag
/// - `caption="..."` or `caption='...'` — caption text
///
/// `remaining_meta` is `None` when any structured fields were recognized (meta is fully consumed).
fn parse_code_meta(meta: &str) -> CodeMeta {
    let mut title: Option<String> = None;
    let mut highlight: Option<Vec<u32>> = None;
    let mut show_line_numbers: Option<bool> = None;
    let mut diff: Option<bool> = None;
    let mut caption: Option<String> = None;
    let mut found_structured = false;

    let mut remaining = meta.trim();
    let mut unrecognized_parts: Vec<&str> = Vec::new();

    while !remaining.is_empty() {
        // Skip whitespace
        remaining = remaining.trim_start();
        if remaining.is_empty() {
            break;
        }

        // Try title="..." or title='...'
        if remaining.starts_with("title=") {
            let rest = &remaining[6..];
            if let Some((val, consumed)) = parse_quoted_value(rest) {
                title = Some(val);
                remaining = &remaining[6 + consumed..];
                found_structured = true;
                continue;
            }
        }

        // Try caption="..." or caption='...'
        if remaining.starts_with("caption=") {
            let rest = &remaining[8..];
            if let Some((val, consumed)) = parse_quoted_value(rest) {
                caption = Some(val);
                remaining = &remaining[8 + consumed..];
                found_structured = true;
                continue;
            }
        }

        // Try {lines} highlight syntax: {3-5,12}
        if remaining.starts_with('{')
            && let Some(close) = remaining.find('}')
        {
            let content = &remaining[1..close];
            let lines = parse_highlight_lines(content);
            if !lines.is_empty() {
                highlight = Some(lines);
                remaining = &remaining[close + 1..];
                found_structured = true;
                continue;
            }
        }

        // Try showLineNumbers flag
        if remaining.starts_with("showLineNumbers") {
            let rest = &remaining[15..];
            if rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\t') {
                show_line_numbers = Some(true);
                remaining = rest.trim_start();
                found_structured = true;
                continue;
            }
        }

        // Try diff flag
        if remaining.starts_with("diff") {
            let rest = &remaining[4..];
            if rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\t') {
                diff = Some(true);
                remaining = rest.trim_start();
                found_structured = true;
                continue;
            }
        }

        // Unrecognized token — consume until next whitespace or end
        let token_end = remaining
            .find(|c: char| c.is_ascii_whitespace())
            .unwrap_or(remaining.len());
        unrecognized_parts.push(&remaining[..token_end]);
        remaining = &remaining[token_end..];
    }

    let remaining_meta = if unrecognized_parts.is_empty() {
        None
    } else {
        Some(unrecognized_parts.join(" "))
    };

    // If no structured fields were found, treat the whole meta as remaining
    if !found_structured {
        return CodeMeta {
            remaining_meta: Some(meta.to_string()),
            ..CodeMeta::default()
        };
    }

    CodeMeta {
        title,
        highlight,
        show_line_numbers,
        diff,
        caption,
        remaining_meta,
    }
}

/// Parse a quoted string value starting at the first character (quote char).
/// Returns `(value, bytes_consumed)` or `None` if not a valid quoted string.
fn parse_quoted_value(s: &str) -> Option<(String, usize)> {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let quote = bytes[0];
    if quote != b'"' && quote != b'\'' {
        return None;
    }
    let mut i = 1;
    let mut value = String::new();
    while i < s.len() {
        if bytes[i] == b'\\' && i + 1 < s.len() {
            if bytes[i + 1] == quote {
                value.push(quote as char);
                i += 2;
            } else if bytes[i + 1] == b'\\' {
                value.push('\\');
                i += 2;
            } else {
                value.push('\\');
                value.push(bytes[i + 1] as char);
                i += 2;
            }
        } else if bytes[i] == quote {
            return Some((value, i + 1));
        } else {
            let ch = s[i..].chars().next().unwrap();
            value.push(ch);
            i += ch.len_utf8();
        }
    }
    None // unclosed quote
}

/// Parse a highlight lines specification like `3-5,12,20-22`.
/// Returns a sorted, deduplicated list of line numbers.
fn parse_highlight_lines(spec: &str) -> Vec<u32> {
    let mut lines = Vec::new();
    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((start, end)) = part.split_once('-') {
            if let (Ok(s), Ok(e)) = (start.trim().parse::<u32>(), end.trim().parse::<u32>()) {
                for n in s..=e {
                    lines.push(n);
                }
            } else {
                return Vec::new(); // invalid spec
            }
        } else if let Ok(n) = part.parse::<u32>() {
            lines.push(n);
        } else {
            return Vec::new(); // invalid spec
        }
    }
    lines.sort_unstable();
    lines.dedup();
    lines
}

/// Post-process a list of nodes to attach inline code language hints.
///
/// When pulldown-cmark emits a `CodeInline` node, the `{lang}` suffix (if present)
/// is included in the immediately following `Text` node. This function merges them:
/// if a `Text` node starts with `{identifier}` immediately after a `CodeInline`,
/// the identifier is moved into `CodeInlineNode.lang` and stripped from the text.
pub(crate) fn apply_inline_code_lang_hints(nodes: Vec<Node>) -> Vec<Node> {
    let mut result = Vec::with_capacity(nodes.len());
    let mut iter = nodes.into_iter().peekable();

    while let Some(node) = iter.next() {
        if let Node::CodeInline(mut ci) = node {
            // Peek at next node: if it's a Text starting with {lang}
            if let Some(Node::Text(text_node)) = iter.peek()
                && let Some((lang, rest_text)) = extract_lang_hint(&text_node.value)
            {
                let text_pos = text_node.position.clone();
                ci.lang = Some(lang);
                iter.next(); // consume the text node
                result.push(Node::CodeInline(ci));
                // Only re-emit the text node if there's remaining text after the hint
                if !rest_text.is_empty() {
                    result.push(Node::Text(TextNode {
                        value: rest_text,
                        position: text_pos,
                    }));
                }
                continue;
            }
            result.push(Node::CodeInline(ci));
        } else {
            // Recursively apply to children
            let node = apply_lang_hints_recursive(node);
            result.push(node);
        }
    }

    result
}

/// Extract `{lang}` from the start of a text string.
/// Returns `(lang, remaining_text)` if the text starts with a valid `{identifier}`.
fn extract_lang_hint(text: &str) -> Option<(String, String)> {
    if !text.starts_with('{') {
        return None;
    }
    let close = text.find('}')?;
    let lang = &text[1..close];
    // Validate: must be [a-zA-Z][a-zA-Z0-9_-]*
    if lang.is_empty() {
        return None;
    }
    let bytes = lang.as_bytes();
    if !bytes[0].is_ascii_alphabetic() {
        return None;
    }
    for &b in &bytes[1..] {
        if !b.is_ascii_alphanumeric() && b != b'_' && b != b'-' {
            return None;
        }
    }
    Some((lang.to_string(), text[close + 1..].to_string()))
}

fn apply_lang_hints_recursive(node: Node) -> Node {
    match node {
        Node::Paragraph(mut b) => {
            b.children = apply_inline_code_lang_hints(b.children);
            Node::Paragraph(b)
        }
        Node::Heading(mut b) => {
            b.children = apply_inline_code_lang_hints(b.children);
            Node::Heading(b)
        }
        Node::ListItem(mut b) => {
            b.children = apply_inline_code_lang_hints(b.children);
            Node::ListItem(b)
        }
        Node::Blockquote(mut b) => {
            b.children = apply_inline_code_lang_hints(b.children);
            Node::Blockquote(b)
        }
        Node::Emphasis(mut b) => {
            b.children = apply_inline_code_lang_hints(b.children);
            Node::Emphasis(b)
        }
        Node::Strong(mut b) => {
            b.children = apply_inline_code_lang_hints(b.children);
            Node::Strong(b)
        }
        Node::Strikethrough(mut b) => {
            b.children = apply_inline_code_lang_hints(b.children);
            Node::Strikethrough(b)
        }
        Node::TableCell(mut b) => {
            b.children = apply_inline_code_lang_hints(b.children);
            Node::TableCell(b)
        }
        Node::Link(mut l) => {
            l.children = apply_inline_code_lang_hints(l.children);
            Node::Link(l)
        }
        Node::Image(mut i) => {
            i.children = apply_inline_code_lang_hints(i.children);
            Node::Image(i)
        }
        other => other,
    }
}
