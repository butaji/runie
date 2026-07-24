//! Markdown parsing: block and inline span extraction from `pulldown_cmark`.
//!
//! Single parsing pass produces block structure (`CodeBlock`) and inline spans
//! (`MdInline`), so line counts in core stay in sync with rendered output.
//! This replaces the former `blocks.rs` + `inline.rs` split.

use pulldown_cmark::{Alignment, Event, Options, Parser, Tag, TagEnd};

use super::{CodeBlock, MdInline};

// ── Parser options ────────────────────────────────────────────────────────────

/// Parser options: standard markdown plus tables, strikethrough, tasklists.
pub(crate) fn md_options() -> Options {
    Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS
}

// ── Inline parsing ────────────────────────────────────────────────────────────

/// Parse inline markdown spans (bold, italic, code, breaks) from plain text.
pub fn parse_inline_spans(text: &str) -> Vec<MdInline> {
    let parser = Parser::new_ext(text, md_options());
    parse_spans_from_events(parser)
}

fn parse_spans_from_events<'a, I>(parser: I) -> Vec<MdInline>
where
    I: Iterator<Item = Event<'a>>,
{
    let mut spans = Vec::new();
    let mut style_stack: Vec<Style> = vec![Style::None];

    for event in parser {
        match event {
            Event::Text(t) => {
                let current = *style_stack.last().unwrap();
                match current {
                    Style::Bold => spans.push(MdInline::Bold(t.to_string())),
                    Style::Italic => spans.push(MdInline::Italic(t.to_string())),
                    Style::Strike => spans.push(MdInline::Strike(t.to_string())),
                    Style::None => spans.push(MdInline::Text(t.to_string())),
                }
            }
            Event::Code(code) => spans.push(MdInline::Code(code.to_string())),
            Event::SoftBreak => spans.push(MdInline::SoftBreak),
            Event::HardBreak => spans.push(MdInline::HardBreak),
            Event::Start(tag) => {
                style_stack.push(style_for_tag(&tag, *style_stack.last().unwrap()));
            }
            Event::End(_) => {
                style_stack.pop();
            }
            _ => {}
        }
    }
    spans
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Style {
    #[default]
    None,
    Bold,
    Italic,
    Strike,
}

impl Style {
    fn marker(&self) -> &'static str {
        match self {
            Style::None => "",
            Style::Bold => "**",
            Style::Italic => "*",
            Style::Strike => "~~",
        }
    }
}

fn style_for_tag(tag: &Tag<'_>, current: Style) -> Style {
    match tag {
        Tag::Strong => Style::Bold,
        Tag::Emphasis => Style::Italic,
        Tag::Strikethrough => Style::Strike,
        _ => current,
    }
}

/// Convert parsed inline spans back to raw markdown text (round-trips).
/// Used to reconstruct the raw text for the `content` field of a Text block.
#[allow(clippy::too_many_lines)]
pub fn inlines_to_text(inlines: &[MdInline]) -> String {
    use std::fmt::Write;

    let mut out = String::new();
    let mut current: Style = Style::None;
    let mut outer: Style = Style::None;

    for span in inlines {
        let new_style = match span {
            MdInline::Bold(_) => Style::Bold,
            MdInline::Italic(_) => Style::Italic,
            MdInline::Strike(_) => Style::Strike,
            _ => Style::None,
        };

        match span {
            MdInline::Text(s) => {
                out.push_str(s);
                current = Style::None;
            }
            MdInline::Bold(s) | MdInline::Italic(s) | MdInline::Strike(s) => {
                if new_style == outer {
                    out.push_str(s);
                    current = new_style;
                } else {
                    outer = current;
                    out.write_str(new_style.marker()).unwrap();
                    out.push_str(s);
                    out.write_str(new_style.marker()).unwrap();
                    current = new_style;
                }
            }
            MdInline::Code(s) => {
                if current != Style::None {
                    out.write_str(current.marker()).unwrap();
                }
                out.push('`');
                out.push_str(s);
                out.push('`');
                if current != Style::None {
                    out.write_str(current.marker()).unwrap();
                }
            }
            MdInline::SoftBreak | MdInline::HardBreak => {
                out.push('\n');
            }
        }
    }

    if current != Style::None {
        out.write_str(current.marker()).unwrap();
    }

    out
}

// ── Block parsing ─────────────────────────────────────────────────────────────

/// Split unclosed code fences so the trailing fence is kept as raw text
/// (rendered as a Text block) rather than an unclosed Code block.
pub(crate) fn split_unclosed_fence(text: &str) -> (&str, Option<&str>) {
    let mut offset = 0;
    let mut in_code = false;
    let mut last_fence_start: Option<usize> = None;
    for line in text.split_inclusive('\n') {
        if line.starts_with("```") {
            if in_code {
                last_fence_start = None;
                in_code = false;
            } else {
                last_fence_start = Some(offset);
                in_code = true;
            }
        }
        offset += line.len();
    }
    if let Some(start) = last_fence_start {
        (&text[..start], Some(&text[start..]))
    } else {
        (text, None)
    }
}

// ── Block state machine ────────────────────────────────────────────────────────

#[derive(Default)]
enum BlockState {
    #[default]
    Top,
    Heading {
        level: u8,
        inlines: Vec<MdInline>,
    },
    Code {
        lang: String,
        content: String,
    },
    List {
        ordered: bool,
        items: Vec<Vec<MdInline>>,
        current: Vec<MdInline>,
    },
    Quote {
        inlines: Vec<MdInline>,
        depth: usize,
    },
    Table {
        headers: Vec<String>,
        alignments: Vec<Option<bool>>,
        rows: Vec<Vec<String>>,
        current_row: Vec<String>,
        in_header: bool,
    },
}

/// Minimal style marker used while collecting styled spans from events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlStyle {
    None,
    Bold,
    Italic,
    Strike,
}

/// Accumulates text content for blocks.
struct BlockParser {
    blocks: Vec<CodeBlock>,
    content_buf: String,
    inline_buf: Vec<MdInline>,
    style_stack: Vec<BlStyle>,
    state: BlockState,
    /// Stack of outer quote inlines when nesting blockquotes (e.g. `> a\n>> b`).
    /// Each entry holds the inlines accumulated for an outer quote level.
    /// Stack of (depth, inlines) for outer quote levels when nesting.
    quotes_stack: Vec<(usize, Vec<MdInline>)>,
}

impl BlockParser {
    fn new() -> Self {
        Self {
            blocks: Vec::new(),
            content_buf: String::new(),
            inline_buf: Vec::new(),
            style_stack: vec![BlStyle::None],
            state: BlockState::Top,
            quotes_stack: Vec::new(),
        }
    }

    fn finish(mut self) -> Vec<CodeBlock> {
        self.flush_text();
        // Emit any remaining quote blocks (including from the stack)
        if let BlockState::Quote { inlines, depth } = &mut self.state {
            if !inlines.is_empty() {
                self.blocks.push(CodeBlock::Blockquote(std::mem::take(inlines), *depth));
            }
        }
        for (depth, inlines) in self.quotes_stack.into_iter().rev() {
            if !inlines.is_empty() {
                self.blocks.push(CodeBlock::Blockquote(inlines, depth));
            }
        }
        self.blocks
    }

    fn emit_inline(&mut self, text: &str, style: BlStyle) {
        if text.is_empty() {
            return;
        }
        let inline = match style {
            BlStyle::Bold => MdInline::Bold(text.to_owned()),
            BlStyle::Italic => MdInline::Italic(text.to_owned()),
            BlStyle::Strike => MdInline::Strike(text.to_owned()),
            BlStyle::None => MdInline::Text(text.to_owned()),
        };
        self.inline_buf.push(inline);
    }

    fn flush_text(&mut self) {
        if self.content_buf.is_empty() && self.inline_buf.is_empty() {
            return;
        }
        // Only flush text blocks in Top state; Heading/Table/Quote manage their own content
        if !matches!(self.state, BlockState::Top) {
            return;
        }
        let content = std::mem::take(&mut self.content_buf);
        let inlines = std::mem::take(&mut self.inline_buf);
        self.blocks.push(CodeBlock::Text { content, inlines });
    }

    fn handle(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.start_tag(tag),
            Event::End(tag_end) => self.end_tag(tag_end),
            Event::Text(t) => self.push_text(&t),
            Event::Code(t) => self.push_code(&t),
            Event::SoftBreak => self.push_break(),
            Event::HardBreak => self.push_break(),
            Event::Rule => self.handle_rule(),
            _ => {}
        }
    }

    fn handle_rule(&mut self) {
        self.flush_text();
        self.blocks.push(CodeBlock::HorizontalRule);
    }

    #[allow(clippy::too_many_lines)]
    fn start_tag(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Heading { level, .. } => {
                self.flush_text();
                let level_num = match level {
                    pulldown_cmark::HeadingLevel::H1 => 1,
                    pulldown_cmark::HeadingLevel::H2 => 2,
                    pulldown_cmark::HeadingLevel::H3 => 3,
                    pulldown_cmark::HeadingLevel::H4 => 4,
                    pulldown_cmark::HeadingLevel::H5 => 5,
                    pulldown_cmark::HeadingLevel::H6 => 6,
                };
                self.state = BlockState::Heading { level: level_num, inlines: Vec::new() };
            }
            Tag::CodeBlock(kind) => {
                self.flush_text();
                let lang = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                    pulldown_cmark::CodeBlockKind::Indented => String::new(),
                };
                self.state = BlockState::Code { lang, content: String::new() };
            }
            Tag::Table(alignments) => {
                self.flush_text();
                self.state = BlockState::Table {
                    headers: Vec::new(),
                    alignments: alignments.iter().map(|a| match a {
                        Alignment::Left => Some(false),
                        Alignment::Right => Some(true),
                        Alignment::Center => None,
                        Alignment::None => None,
                    }).collect(),
                    rows: Vec::new(),
                    current_row: Vec::new(),
                    in_header: true,
                };
            }
            Tag::TableHead => {
                // Table head started, headers will be collected via push_text
            }
            Tag::TableRow => {
                if let BlockState::Table { headers, rows, current_row, in_header, .. } = &mut self.state {
                    if *in_header && !headers.is_empty() {
                        *in_header = false;
                    } else if !current_row.is_empty() {
                        rows.push(std::mem::take(current_row));
                    }
                    *current_row = Vec::new();
                }
            }
            Tag::TableCell => {
                // Cell started, text will be collected via push_text
            }
            Tag::List(order) => {
                self.flush_text();
                self.content_buf.clear();
                self.inline_buf.clear();
                self.state = BlockState::List { ordered: order.is_some(), items: Vec::new(), current: Vec::new() };
            }
            Tag::Item => {
                if let BlockState::List { items, current, .. } = &mut self.state {
                    if !current.is_empty() {
                        items.push(std::mem::take(current));
                    }
                }
            }
            Tag::BlockQuote(_) => {
                if matches!(self.state, BlockState::Top) {
                    self.flush_text();
                }
                // If already in a quote, save the current inlines to the stack and
                // start a fresh quote for the nested level
                if let BlockState::Quote { inlines, depth } = std::mem::take(&mut self.state) {
                    if !inlines.is_empty() {
                        self.blocks.push(CodeBlock::Blockquote(inlines.clone(), depth));
                    }
                    self.quotes_stack.push((depth, inlines));
                    self.state = BlockState::Quote { inlines: Vec::new(), depth: depth + 1 };
                } else {
                    self.state = BlockState::Quote { inlines: Vec::new(), depth: 1 };
                }
            }
            Tag::Strong => self.style_stack.push(BlStyle::Bold),
            Tag::Emphasis => self.style_stack.push(BlStyle::Italic),
            Tag::Strikethrough => self.style_stack.push(BlStyle::Strike),
            _ => {}
        }
    }

    fn push_text(&mut self, text: &str) {
        let current_style = *self.style_stack.last().unwrap();
        match &mut self.state {
            BlockState::Top => {
                self.emit_inline(text, current_style);
                self.content_buf.push_str(text);
            }
            BlockState::Heading { inlines, .. } => {
                if !text.is_empty() {
                    let inline = match current_style {
                        BlStyle::Bold => MdInline::Bold(text.to_owned()),
                        BlStyle::Italic => MdInline::Italic(text.to_owned()),
                        BlStyle::Strike => MdInline::Strike(text.to_owned()),
                        BlStyle::None => MdInline::Text(text.to_owned()),
                    };
                    inlines.push(inline);
                }
                self.content_buf.push_str(text);
            }
            BlockState::Code { content, .. } => content.push_str(text),
            BlockState::List { current, .. } => {
                if !text.is_empty() {
                    let inline = match current_style {
                        BlStyle::Bold => MdInline::Bold(text.to_owned()),
                        BlStyle::Italic => MdInline::Italic(text.to_owned()),
                        BlStyle::Strike => MdInline::Strike(text.to_owned()),
                        BlStyle::None => MdInline::Text(text.to_owned()),
                    };
                    current.push(inline);
                }
            }
            BlockState::Quote { inlines, .. } => {
                if !text.is_empty() {
                    let inline = match current_style {
                        BlStyle::Bold => MdInline::Bold(text.to_owned()),
                        BlStyle::Italic => MdInline::Italic(text.to_owned()),
                        BlStyle::Strike => MdInline::Strike(text.to_owned()),
                        BlStyle::None => MdInline::Text(text.to_owned()),
                    };
                    inlines.push(inline);
                }
            }
            BlockState::Table { headers, current_row, in_header, .. } => {
                if *in_header {
                    headers.push(text.to_owned());
                } else {
                    current_row.push(text.to_owned());
                }
            }
        }
    }

    fn push_code(&mut self, code: &str) {
        match &mut self.state {
            BlockState::Top => {
                self.inline_buf.push(MdInline::Code(code.to_string()));
                self.content_buf.push('`');
                self.content_buf.push_str(code);
                self.content_buf.push('`');
            }
            BlockState::Heading { inlines, .. } => {
                inlines.push(MdInline::Code(code.to_string()));
                self.content_buf.push('`');
                self.content_buf.push_str(code);
                self.content_buf.push('`');
            }
            BlockState::Code { content, .. } => content.push_str(code),
            BlockState::List { current, .. } => {
                current.push(MdInline::Code(code.to_string()));
            }
            BlockState::Quote { inlines, .. } => {
                inlines.push(MdInline::Code(code.to_string()));
            }
            BlockState::Table { .. } => {
                // Inline code in tables - store as cell text
            }
        }
    }

    fn push_break(&mut self) {
        match &mut self.state {
            BlockState::Top => {
                self.inline_buf.push(MdInline::SoftBreak);
                self.content_buf.push('\n');
            }
            BlockState::Heading { inlines, .. } => {
                inlines.push(MdInline::SoftBreak);
                self.content_buf.push('\n');
            }
            BlockState::Code { content, .. } => content.push('\n'),
            BlockState::List { current, .. } => {
                current.push(MdInline::SoftBreak);
            }
            BlockState::Quote { inlines, .. } => {
                inlines.push(MdInline::SoftBreak);
            }
            BlockState::Table { .. } => {
                // Table cells don't have breaks
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn end_tag(&mut self, tag_end: TagEnd) {
        match tag_end {
            TagEnd::Heading(_level) => {
                if let BlockState::Heading { level: lvl, inlines } = std::mem::take(&mut self.state) {
                    let content = std::mem::take(&mut self.content_buf);
                    self.blocks.push(CodeBlock::Heading { level: lvl, content, inlines });
                }
            }
            TagEnd::CodeBlock => {
                if let BlockState::Code { lang, content } = std::mem::take(&mut self.state) {
                    self.blocks.push(CodeBlock::Code { lang, content });
                }
            }
            TagEnd::Table => {
                if let BlockState::Table { headers, alignments, rows, current_row, .. } = std::mem::take(&mut self.state) {
                    // Push last row if not empty
                    if !current_row.is_empty() {
                        let mut all_rows = rows;
                        all_rows.push(current_row);
                        self.blocks.push(CodeBlock::Table { headers, alignments, rows: all_rows });
                    } else if !headers.is_empty() {
                        self.blocks.push(CodeBlock::Table { headers, alignments, rows });
                    }
                }
            }
            TagEnd::TableHead => {
                // Table head ended, separator row will follow
            }
            TagEnd::TableRow => {
                // Row ended, push_text handles adding to rows
            }
            TagEnd::TableCell => {
                // Cell ended
            }
            TagEnd::List(_) => {
                if let BlockState::List { ordered, mut items, current } = std::mem::take(&mut self.state) {
                    if !current.is_empty() {
                        items.push(current);
                    }
                    if !items.is_empty() {
                        self.blocks.push(CodeBlock::List { ordered, items });
                    }
                }
            }
            TagEnd::Item => {
                if let BlockState::List { items, current, .. } = &mut self.state {
                    if !current.is_empty() {
                        items.push(std::mem::take(current));
                    }
                }
            }
            TagEnd::BlockQuote(_) => {
                if let BlockState::Quote { inlines, depth } = std::mem::take(&mut self.state) {
                    if !inlines.is_empty() {
                        self.blocks.push(CodeBlock::Blockquote(inlines, depth));
                    }
                    // If there are outer quote levels on the stack, pop back to the parent
                    if let Some((outer_depth, outer_inlines)) = self.quotes_stack.pop() {
                        self.state = BlockState::Quote { inlines: outer_inlines, depth: outer_depth };
                    }
                }
                self.content_buf.clear();
                self.inline_buf.clear();
            }
            TagEnd::Strong => {
                self.style_stack.pop();
            }
            TagEnd::Emphasis => {
                self.style_stack.pop();
            }
            TagEnd::Strikethrough => {
                self.style_stack.pop();
            }
            _ => {}
        }
    }
}

/// Parse markdown text into a list of blocks.
pub fn extract_blocks(text: &str) -> Vec<CodeBlock> {
    let mut parser = BlockParser::new();
    for event in Parser::new_ext(text, md_options()) {
        parser.handle(event);
    }
    parser.finish()
}
