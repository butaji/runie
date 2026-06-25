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
        Self {
            blocks: Vec::new(),
            text_buf: String::new(),
            state: BlockState::Top,
            inline_stack: Vec::new(),
        }
    }

    fn finish(mut self) -> Vec<CodeBlock> {
        self.flush_text();
        self.blocks
    }

    fn flush_text(&mut self) {
        if !self.text_buf.is_empty() {
            let content = std::mem::take(&mut self.text_buf);
            let inlines = super::parse_inline_spans(&content);
            self.blocks.push(CodeBlock::Text { content, inlines });
        }
    }

    fn handle(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.start_tag(tag),
            Event::End(tag_end) => self.end_tag(tag_end),
            Event::Text(t) | Event::Code(t) => self.push_text(&t),
            Event::SoftBreak | Event::HardBreak => self.push_break(),
            // intentionally ignored: other event types fall through
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
            // intentionally ignored: other tags fall through
            _ => {}
        }
    }

    fn start_code(&mut self, kind: pulldown_cmark::CodeBlockKind<'_>) {
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

    fn start_list(&mut self, ordered: bool) {
        self.flush_text();
        self.state = BlockState::List {
            ordered,
            items: Vec::new(),
            current: String::new(),
        };
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
        self.state = BlockState::Quote {
            content: String::new(),
        };
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
                    self.inline_stack
                        .retain(|m| !matches!(m, InlineMarker::Bold));
                }
            }
            TagEnd::Emphasis => {
                if matches!(self.state, BlockState::Top) {
                    self.text_buf.push('*');
                    self.inline_stack
                        .retain(|m| !matches!(m, InlineMarker::Italic));
                }
            }
            TagEnd::Strikethrough => {
                if matches!(self.state, BlockState::Top) {
                    self.text_buf.push_str("~~");
                    self.inline_stack
                        .retain(|m| !matches!(m, InlineMarker::Strike));
                }
            }
            // intentionally ignored: other tags fall through
            _ => {}
        }
    }

    fn end_code(&mut self) {
        if let BlockState::Code { lang, content } = std::mem::take(&mut self.state) {
            self.blocks.push(CodeBlock::Code { lang, content });
        }
    }

    fn end_list(&mut self) {
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

pub fn extract_blocks(text: &str) -> Vec<CodeBlock> {
    let mut parser = BlockParser::new();
    for event in Parser::new_ext(text, md_options()) {
        parser.handle(event);
    }
    parser.finish()
}
