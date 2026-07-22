//! Markdown parsing: block and inline span extraction from `pulldown_cmark`.
//!
//! Single parsing pass produces block structure (`CodeBlock`) and inline spans
//! (`MdInline`), so line counts in core stay in sync with rendered output.
//! This replaces the former `blocks.rs` + `inline.rs` split.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

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
}

impl BlockParser {
    fn new() -> Self {
        Self {
            blocks: Vec::new(),
            content_buf: String::new(),
            inline_buf: Vec::new(),
            style_stack: vec![BlStyle::None],
            state: BlockState::Top,
        }
    }

    fn finish(mut self) -> Vec<CodeBlock> {
        self.flush_text();
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
            _ => {}
        }
    }

    #[allow(clippy::too_many_lines)]
    fn start_tag(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::CodeBlock(kind) => {
                self.flush_text();
                let lang = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                    pulldown_cmark::CodeBlockKind::Indented => String::new(),
                };
                self.state = BlockState::Code { lang, content: String::new() };
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
                self.state = BlockState::Quote { inlines: Vec::new() };
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
            BlockState::Quote { inlines } => {
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
            BlockState::Code { content, .. } => content.push_str(code),
            BlockState::List { current, .. } => {
                current.push(MdInline::Code(code.to_string()));
            }
            BlockState::Quote { inlines } => {
                inlines.push(MdInline::Code(code.to_string()));
            }
        }
    }

    fn push_break(&mut self) {
        match &mut self.state {
            BlockState::Top => {
                self.inline_buf.push(MdInline::SoftBreak);
                self.content_buf.push('\n');
            }
            BlockState::Code { content, .. } => content.push('\n'),
            BlockState::List { current, .. } => {
                current.push(MdInline::SoftBreak);
            }
            BlockState::Quote { inlines } => {
                inlines.push(MdInline::SoftBreak);
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn end_tag(&mut self, tag_end: TagEnd) {
        match tag_end {
            TagEnd::CodeBlock => {
                if let BlockState::Code { lang, content } = std::mem::take(&mut self.state) {
                    self.blocks.push(CodeBlock::Code { lang, content });
                }
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
                if let BlockState::Quote { inlines } = std::mem::take(&mut self.state) {
                    self.blocks.push(CodeBlock::Blockquote(inlines));
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
