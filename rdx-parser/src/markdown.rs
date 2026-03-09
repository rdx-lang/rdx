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
            FrameKind::CodeBlock { lang, meta } => Node::CodeBlock(CodeBlockNode {
                value: self.code_text,
                lang,
                meta,
                position: pos,
            }),
            FrameKind::FootnoteDefinition { label } => Node::FootnoteDefinition(FootnoteNode {
                label,
                children: self.children,
                position: pos,
            }),
            FrameKind::HtmlBlock => Node::Html(std_block(self.children, pos)),
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
                let node = Node::CodeInline(TextNode {
                    value: code.to_string(),
                    position: sm.position(abs_start, abs_end),
                });
                push_node(&mut stack, &mut result, &mut comp_stack, node);
            }

            Event::Html(ref html_text) | Event::InlineHtml(ref html_text) => {
                let is_inline_html = matches!(event, Event::InlineHtml(_));
                let html_str = html_text.to_string();

                // Try to parse all RDX components from this HTML event.
                // A single HTML block event may contain multiple component lines.
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

                // Check if component close tag
                let trimmed = html_str.trim();
                if try_handle_close_tag(
                    trimmed,
                    &html_str,
                    abs_start,
                    abs_end,
                    sm,
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
                let node = Node::MathInline(TextNode {
                    value: math_text.to_string(),
                    position: sm.position(abs_start, abs_end),
                });
                push_node(&mut stack, &mut result, &mut comp_stack, node);
            }

            Event::DisplayMath(ref math_text) => {
                let node = Node::MathDisplay(TextNode {
                    value: math_text.to_string(),
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

    result
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
                let node = Node::Component(ComponentNode {
                    name: open_tag.name,
                    is_inline: false,
                    attributes: open_tag.attributes,
                    children,
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
