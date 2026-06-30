//! Block parser: collects styled inline spans directly from `pulldown_cmark` events.
//!
//! The `BlockParser` walks the `pulldown_cmark` event stream and collects:
//!   - Code blocks (fenced or indented)
//!   - Lists (ordered or unordered)
//!   - Block quotes
//!   - Text with inline styles (bold, italic, strike, code, breaks)
//!
//! Styled spans are collected in a single pass: when `Event::Text` arrives,
//! the current style from the style stack is used to emit the correct `MdInline`.
//! This means `BlockParser` no longer calls `parse_inline_spans` after-the-fact;
//! the styles come directly from the event stream.

use pulldown_cmark::{Event, Parser, Tag, TagEnd};

use super::md_options;
use super::CodeBlock;

// Note: Parser buffer flushes use `mem::take`. These are idiomatic Rust patterns
// for "move out of mutable buffer, reset to empty, push to result". Each flush
// takes accumulated text/code/list content and resets the parser state for the next block.

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
        items: Vec<Vec<super::MdInline>>,
        current: Vec<super::MdInline>,
    },
    Quote {
        inlines: Vec<super::MdInline>,
    },
}

/// Minimal style marker used while collecting styled spans from events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Style {
    None,
    Bold,
    Italic,
    Strike,
}

/// Accumulates text content for blocks.
struct BlockParser {
    blocks: Vec<CodeBlock>,
    /// Accumulated text for current block (plain text, no styles yet).
    content_buf: String,
    /// Accumulated inline spans for the current Text block.
    inline_buf: Vec<super::MdInline>,
    /// Current style stack while parsing inline content.
    style_stack: Vec<Style>,
    state: BlockState,
}

impl BlockParser {
    fn new() -> Self {
        Self {
            blocks: Vec::new(),
            content_buf: String::new(),
            inline_buf: Vec::new(),
            style_stack: vec![Style::None],
            state: BlockState::Top,
        }
    }

    fn finish(mut self) -> Vec<CodeBlock> {
        self.flush_text();
        self.blocks
    }

    /// Push a text segment as an inline span with the given style.
    fn emit_inline(&mut self, text: &str, style: Style) {
        if text.is_empty() {
            return;
        }
        let inline = match style {
            Style::Bold => super::MdInline::Bold(text.to_owned()),
            Style::Italic => super::MdInline::Italic(text.to_owned()),
            Style::Strike => super::MdInline::Strike(text.to_owned()),
            Style::None => super::MdInline::Text(text.to_owned()),
        };
        self.inline_buf.push(inline);
    }

    /// Push accumulated inline spans and flush the text block.
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

    fn start_tag(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::CodeBlock(kind) => {
                self.flush_text();
                let lang = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                    pulldown_cmark::CodeBlockKind::Indented => String::new(),
                };
                self.state = BlockState::Code {
                    lang,
                    content: String::new(),
                };
            }
            Tag::List(order) => {
                self.flush_text();
                // Clear inline buffers so preceding text doesn't become part of the first item.
                self.content_buf.clear();
                self.inline_buf.clear();
                self.state = BlockState::List {
                    ordered: order.is_some(),
                    items: Vec::new(),
                    current: Vec::new(),
                };
            }
            Tag::Item => {
                if let BlockState::List { items, current, .. } = &mut self.state {
                    if !current.is_empty() {
                        items.push(std::mem::take(current));
                    }
                }
            }
            Tag::BlockQuote(_) => {
                // Flush any pending text before starting the blockquote.
                if matches!(self.state, BlockState::Top) {
                    self.flush_text();
                }
                self.state = BlockState::Quote {
                    inlines: Vec::new(),
                };
            }
            // Style tags: push onto style stack so subsequent text gets the right style.
            Tag::Strong => self.style_stack.push(Style::Bold),
            Tag::Emphasis => self.style_stack.push(Style::Italic),
            Tag::Strikethrough => self.style_stack.push(Style::Strike),
            _ => {}
        }
    }

    /// Route text to the appropriate buffer based on current block state.
    /// All states emit inline spans directly from events (pulldown_cmark gives us the style).
    fn push_text(&mut self, text: &str) {
        let current_style = *self.style_stack.last().unwrap();
        match &mut self.state {
            BlockState::Top => {
                self.emit_inline(text, current_style);
                self.content_buf.push_str(text);
            }
            BlockState::Code { content, .. } => content.push_str(text),
            BlockState::List { current, .. } => {
                // Push inline directly to current item buffer (can't call emit_inline due to borrow).
                if !text.is_empty() {
                    let inline = match current_style {
                        Style::Bold => super::MdInline::Bold(text.to_owned()),
                        Style::Italic => super::MdInline::Italic(text.to_owned()),
                        Style::Strike => super::MdInline::Strike(text.to_owned()),
                        Style::None => super::MdInline::Text(text.to_owned()),
                    };
                    current.push(inline);
                }
            }
            BlockState::Quote { inlines } => {
                if !text.is_empty() {
                    let inline = match current_style {
                        Style::Bold => super::MdInline::Bold(text.to_owned()),
                        Style::Italic => super::MdInline::Italic(text.to_owned()),
                        Style::Strike => super::MdInline::Strike(text.to_owned()),
                        Style::None => super::MdInline::Text(text.to_owned()),
                    };
                    inlines.push(inline);
                }
            }
        }
    }

    fn push_code(&mut self, code: &str) {
        match &mut self.state {
            BlockState::Top => {
                self.inline_buf
                    .push(super::MdInline::Code(code.to_string()));
                self.content_buf.push('`');
                self.content_buf.push_str(code);
                self.content_buf.push('`');
            }
            BlockState::Code { content, .. } => content.push_str(code),
            BlockState::List { current, .. } => {
                current.push(super::MdInline::Code(code.to_string()));
            }
            BlockState::Quote { inlines } => {
                inlines.push(super::MdInline::Code(code.to_string()));
            }
        }
    }

    fn push_break(&mut self) {
        match &mut self.state {
            BlockState::Top => {
                self.inline_buf.push(super::MdInline::SoftBreak);
                self.content_buf.push('\n');
            }
            BlockState::Code { content, .. } => content.push('\n'),
            BlockState::List { current, .. } => {
                current.push(super::MdInline::SoftBreak);
            }
            BlockState::Quote { inlines } => {
                inlines.push(super::MdInline::SoftBreak);
            }
        }
    }

    fn end_tag(&mut self, tag_end: TagEnd) {
        match tag_end {
            TagEnd::CodeBlock => {
                if let BlockState::Code { lang, content } = std::mem::take(&mut self.state) {
                    self.blocks.push(CodeBlock::Code { lang, content });
                }
            }
            TagEnd::List(_) => {
                if let BlockState::List {
                    ordered,
                    mut items,
                    current,
                } = std::mem::take(&mut self.state)
                {
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
                // Clear buffers so they don't produce a spurious Text block after the Quote.
                self.content_buf.clear();
                self.inline_buf.clear();
            }
            // Style tags: pop from style stack.
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

pub fn extract_blocks(text: &str) -> Vec<CodeBlock> {
    let mut parser = BlockParser::new();
    for event in Parser::new_ext(text, md_options()) {
        parser.handle(event);
    }
    parser.finish()
}
