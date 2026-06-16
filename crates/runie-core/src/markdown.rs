//! Unified markdown AST shared between core (line-count math) and the TUI
//! (styled rendering).
//!
//! Single parsing pass produces both block structure and inline spans,
//! so line counts in core stay in sync with rendered output in the TUI.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

// ── Inline spans ─────────────────────────────────────────────────────────────

/// Inline markdown element.
#[derive(Debug, Clone, PartialEq)]
pub enum MdInline {
    Text(String),
    Bold(String),
    Italic(String),
    Strike(String),
    Code(String),
    SoftBreak,
    HardBreak,
}

impl MdInline {
    /// The text content of this span (empty for breaks).
    pub fn as_text(&self) -> &str {
        match self {
            MdInline::Text(s) => s,
            MdInline::Bold(s) => s,
            MdInline::Italic(s) => s,
            MdInline::Strike(s) => s,
            MdInline::Code(s) => s,
            MdInline::SoftBreak | MdInline::HardBreak => "",
        }
    }
}

// ── Block AST ────────────────────────────────────────────────────────────────

/// Unified markdown block — represents both block structure and, for Text
/// blocks, the parsed inline spans so the TUI can style them without
/// re-parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum CodeBlock {
    /// Plain text with inline spans already extracted.
    Text {
        content: String,
        inlines: Vec<MdInline>,
    },
    Code { lang: String, content: String },
    List { ordered: bool, items: Vec<String> },
    Blockquote(String),
}

/// Legacy alias so existing call sites don't break.
pub use CodeBlock as Block;

/// Parse markdown into a list of blocks with inline spans extracted.
/// Single pass — both block structure and inline styling are computed together.
pub fn parse_markdown(text: &str) -> Vec<CodeBlock> {
    let (parse_text, trailing) = split_unclosed_fence(text);
    let mut blocks = extract_blocks(parse_text);
    if let Some(t) = trailing {
        let inlines = parse_inline_spans(t);
        blocks.push(CodeBlock::Text { content: t.to_string(), inlines });
    }
    blocks
}

/// Extract code blocks, lists, and blockquotes from text, together with
/// inline spans for text sections.
/// Kept for backward compatibility; prefer `parse_markdown` for new code.
pub fn extract_code_blocks(text: &str) -> Vec<CodeBlock> {
    parse_markdown(text)
}

// ── Inline span parser ──────────────────────────────────────────────────────

/// Parse inline markdown spans (bold, italic, code, breaks) from plain text.
pub fn parse_inline_spans(text: &str) -> Vec<MdInline> {
    let parser = Parser::new_ext(text, md_options());
    parse_spans_from_events(parser)
}

fn md_options() -> Options {
    Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS
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
            Event::Code(code) => {
                spans.push(MdInline::Code(code.to_string()));
            }
            Event::SoftBreak => {
                spans.push(MdInline::SoftBreak);
            }
            Event::HardBreak => {
                spans.push(MdInline::HardBreak);
            }
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

/// Minimal style marker for parsing (core has no Ratatui dependency).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Style {
    None,
    Bold,
    Italic,
    Strike,
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
pub fn inlines_to_text(inlines: &[MdInline]) -> String {
    let mut out = String::new();
    let mut active_markers: Vec<&'static str> = Vec::new();
    for span in inlines {
        match span {
            MdInline::Text(s) => {
                // Close any active markers before plain text
                while !active_markers.is_empty() {
                    out.push_str(active_markers.last().unwrap());
                    active_markers.pop();
                }
                out.push_str(s);
            }
            MdInline::Bold(s) => {
                out.push_str("**");
                out.push_str(s);
                out.push_str("**");
            }
            MdInline::Italic(s) => {
                out.push('*');
                out.push_str(s);
                out.push('*');
            }
            MdInline::Strike(s) => {
                out.push_str("~~");
                out.push_str(s);
                out.push_str("~~");
            }
            MdInline::Code(s) => {
                out.push('`');
                out.push_str(s);
                out.push('`');
            }
            MdInline::SoftBreak | MdInline::HardBreak => out.push('\n'),
        }
    }
    out
}

// ── Block parser ───────────────────────────────────────────────────────────

fn split_unclosed_fence(text: &str) -> (&str, Option<&str>) {
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
        items: Vec<String>,
        current: String,
    },
    Quote {
        content: String,
    },
}

struct BlockParser {
    blocks: Vec<CodeBlock>,
    /// Raw text buffer — inline markers (**, *, `) are injected here so
    /// parse_inline_spans sees the full markdown syntax.
    text_buf: String,
    state: BlockState,
    /// Stack of active inline markers (for Top state only).
    inline_stack: Vec<InlineMarker>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum InlineMarker {
    Bold,
    Italic,
    Strike,
}

impl BlockParser {
    fn new() -> Self {
        Self { blocks: Vec::new(), text_buf: String::new(), state: BlockState::Top, inline_stack: Vec::new() }
    }

    fn finish(mut self) -> Vec<CodeBlock> {
        self.flush_text();
        self.blocks
    }

    fn flush_text(&mut self) {
        if !self.text_buf.is_empty() {
            let content = std::mem::take(&mut self.text_buf);
            let inlines = parse_inline_spans(&content);
            self.blocks.push(CodeBlock::Text { content, inlines });
        }
    }

    fn handle(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.start_tag(tag),
            Event::End(tag_end) => self.end_tag(tag_end),
            Event::Text(t) | Event::Code(t) => self.push_text(&t),
            Event::SoftBreak | Event::HardBreak => self.push_break(),
            _ => {}
        }
    }

    fn start_tag(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::CodeBlock(kind) => self.start_code(kind),
            Tag::List(order) => self.start_list(order.is_some()),
            Tag::Item => self.start_item(),
            Tag::BlockQuote(_) => self.start_quote(),
            Tag::Strong => {
                if matches!(self.state, BlockState::Top) {
                    self.inline_stack.push(InlineMarker::Bold);
                    self.text_buf.push_str("**");
                }
            }
            Tag::Emphasis => {
                if matches!(self.state, BlockState::Top) {
                    self.inline_stack.push(InlineMarker::Italic);
                    self.text_buf.push('*');
                }
            }
            Tag::Strikethrough => {
                if matches!(self.state, BlockState::Top) {
                    self.inline_stack.push(InlineMarker::Strike);
                    self.text_buf.push_str("~~");
                }
            }
            _ => {}
        }
    }

    fn start_code(&mut self, kind: pulldown_cmark::CodeBlockKind<'_>) {
        self.flush_text();
        let lang = match kind {
            pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
            pulldown_cmark::CodeBlockKind::Indented => String::new(),
        };
        self.state = BlockState::Code { lang, content: String::new() };
    }

    fn start_list(&mut self, ordered: bool) {
        self.flush_text();
        self.state = BlockState::List { ordered, items: Vec::new(), current: String::new() };
    }

    fn start_item(&mut self) {
        if let BlockState::List { items, current, .. } = &mut self.state {
            if !current.is_empty() {
                items.push(std::mem::take(current));
            }
        }
    }

    fn start_quote(&mut self) {
        self.flush_text();
        self.state = BlockState::Quote { content: String::new() };
    }

    fn end_tag(&mut self, tag_end: TagEnd) {
        match tag_end {
            TagEnd::CodeBlock => self.end_code(),
            TagEnd::List(_) => self.end_list(),
            TagEnd::Item => self.end_item(),
            TagEnd::BlockQuote(_) => self.end_quote(),
            TagEnd::Strong => {
                if matches!(self.state, BlockState::Top) {
                    self.text_buf.push_str("**");
                    self.inline_stack.retain(|m| !matches!(m, InlineMarker::Bold));
                }
            }
            TagEnd::Emphasis => {
                if matches!(self.state, BlockState::Top) {
                    self.text_buf.push('*');
                    self.inline_stack.retain(|m| !matches!(m, InlineMarker::Italic));
                }
            }
            TagEnd::Strikethrough => {
                if matches!(self.state, BlockState::Top) {
                    self.text_buf.push_str("~~");
                    self.inline_stack.retain(|m| !matches!(m, InlineMarker::Strike));
                }
            }
            _ => {}
        }
    }

    fn end_code(&mut self) {
        if let BlockState::Code { lang, content } = std::mem::take(&mut self.state) {
            self.blocks.push(CodeBlock::Code { lang, content });
        }
    }

    fn end_list(&mut self) {
        if let BlockState::List { ordered, mut items, current } = std::mem::take(&mut self.state) {
            if !current.is_empty() {
                items.push(current);
            }
            if !items.is_empty() {
                self.blocks.push(CodeBlock::List { ordered, items });
            }
        }
    }

    fn end_item(&mut self) {
        if let BlockState::List { items, current, .. } = &mut self.state {
            if !current.is_empty() {
                items.push(std::mem::take(current));
            }
        }
    }

    fn end_quote(&mut self) {
        if let BlockState::Quote { content } = std::mem::take(&mut self.state) {
            self.blocks.push(CodeBlock::Blockquote(content));
        }
    }

    fn push_text(&mut self, text: &str) {
        match &mut self.state {
            BlockState::Top => self.text_buf.push_str(text),
            BlockState::Code { content, .. } => content.push_str(text),
            BlockState::List { current, .. } => current.push_str(text),
            BlockState::Quote { content } => content.push_str(text),
        }
    }

    fn push_break(&mut self) {
        match &mut self.state {
            BlockState::Top => self.text_buf.push('\n'),
            BlockState::Code { content, .. } => content.push('\n'),
            BlockState::List { current, .. } => current.push('\n'),
            BlockState::Quote { content } => content.push('\n'),
        }
    }
}

fn extract_blocks(text: &str) -> Vec<CodeBlock> {
    let mut parser = BlockParser::new();
    for event in Parser::new_ext(text, md_options()) {
        parser.handle(event);
    }
    parser.finish()
}

// ── Legacy helpers ─────────────────────────────────────────────────────────

/// Return the plain-text content of a Text block (for layout.rs line math).
pub fn text_block_content(block: &CodeBlock) -> Option<&str> {
    match block {
        CodeBlock::Text { content, .. } => Some(content),
        _ => None,
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_markdown_splits_fenced_code() {
        let text = "hello\n```rust\nlet x = 1;\n```\nworld";
        let blocks = parse_markdown(text);
        assert_eq!(blocks.len(), 3);
        assert!(matches!(&blocks[0], CodeBlock::Text { content, .. } if content == "hello"));
        assert!(matches!(&blocks[1], CodeBlock::Code { lang, content } if lang == "rust" && content == "let x = 1;\n"));
        assert!(matches!(&blocks[2], CodeBlock::Text { content, .. } if content == "world"));
    }

    #[test]
    fn parse_markdown_handles_unclosed_fence() {
        let text = "hello\n```rust\nlet x = 1;";
        let blocks = parse_markdown(text);
        assert_eq!(blocks.len(), 2);
        assert!(matches!(&blocks[0], CodeBlock::Text { content, .. } if content == "hello"));
        assert!(matches!(&blocks[1], CodeBlock::Text { content, .. } if content.starts_with("```rust")));
    }

    #[test]
    fn parse_markdown_handles_lists() {
        let text = "items:\n- one\n- two\n";
        let blocks = parse_markdown(text);
        assert_eq!(blocks.len(), 2);
        assert!(matches!(&blocks[0], CodeBlock::Text { content, .. } if content == "items:"));
        assert!(matches!(&blocks[1], CodeBlock::List { ordered, items } if !ordered && items.len() == 2));
    }

    #[test]
    fn parse_markdown_handles_blockquote() {
        let text = "> quote line 1\n> quote line 2";
        let blocks = parse_markdown(text);
        assert_eq!(blocks.len(), 1);
        assert!(matches!(&blocks[0], CodeBlock::Blockquote(q) if q.lines().count() == 2));
    }

    #[test]
    fn text_block_has_inlines() {
        let blocks = parse_markdown("hello **bold** world");
        assert_eq!(blocks.len(), 1);
        let CodeBlock::Text { content, inlines } = &blocks[0] else { panic!() };
        assert_eq!(content, "hello **bold** world");
        assert!(inlines.iter().any(|s| matches!(s, MdInline::Text(t) if t == "hello ")));
        assert!(inlines.iter().any(|s| matches!(s, MdInline::Bold(b) if b == "bold")));
        assert!(inlines.iter().any(|s| matches!(s, MdInline::Text(t) if t == " world")));
    }

    #[test]
    fn inlines_to_text_round_trips() {
        let text = "hello **bold** world";
        let inlines = parse_inline_spans(text);
        assert_eq!(inlines_to_text(&inlines), text);
    }

    #[test]
    fn extract_code_blocks_alias_works() {
        // Verify backward compat — extract_code_blocks returns same blocks as parse_markdown
        let text = "hello\n```rust\nlet x;\n```";
        assert_eq!(extract_code_blocks(text), parse_markdown(text));
    }

    #[test]
    fn text_block_content_helper() {
        let blocks = parse_markdown("hello **bold**");
        assert_eq!(text_block_content(&blocks[0]), Some("hello **bold**"));
    }
}
