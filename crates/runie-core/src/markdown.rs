//! Markdown block decomposition shared between core and the TUI renderer.
//!
//! Core uses this for scroll math; the TUI uses the same decomposition so
//! line counts stay in sync with actual rendered rows.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

/// A block of text or code extracted from markdown.
#[derive(Debug, Clone, PartialEq)]
pub enum CodeBlock {
    Text(String),
    Code { lang: String, content: String },
    List { ordered: bool, items: Vec<String> },
    Blockquote(String),
}

/// Extract code blocks (``` fenced), lists, and blockquotes from text.
pub fn extract_code_blocks(text: &str) -> Vec<CodeBlock> {
    let (parse_text, trailing) = split_unclosed_fence(text);
    let mut blocks = extract_blocks(parse_text);
    if let Some(t) = trailing {
        blocks.push(CodeBlock::Text(t.to_string()));
    }
    blocks
}

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
    text_buf: String,
    state: BlockState,
}

impl BlockParser {
    fn new() -> Self {
        Self {
            blocks: Vec::new(),
            text_buf: String::new(),
            state: BlockState::Top,
        }
    }

    fn finish(mut self) -> Vec<CodeBlock> {
        self.flush_text();
        self.blocks
    }

    fn flush_text(&mut self) {
        if !self.text_buf.is_empty() {
            self.blocks
                .push(CodeBlock::Text(std::mem::take(&mut self.text_buf)));
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

fn extract_blocks(text: &str) -> Vec<CodeBlock> {
    let mut parser = BlockParser::new();
    for event in Parser::new_ext(text, Options::empty()) {
        parser.handle(event);
    }
    parser.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_code_blocks_splits_fenced_code() {
        let text = "hello\n```rust\nlet x = 1;\n```\nworld";
        let blocks = extract_code_blocks(text);
        assert_eq!(blocks.len(), 3);
        assert!(matches!(blocks[0], CodeBlock::Text(ref t) if t == "hello"));
        assert!(
            matches!(blocks[1], CodeBlock::Code { ref lang, ref content } if lang == "rust" && content == "let x = 1;\n")
        );
        assert!(matches!(blocks[2], CodeBlock::Text(ref t) if t == "world"));
    }

    #[test]
    fn extract_code_blocks_handles_unclosed_fence() {
        let text = "hello\n```rust\nlet x = 1;";
        let blocks = extract_code_blocks(text);
        assert_eq!(blocks.len(), 2);
        assert!(matches!(blocks[0], CodeBlock::Text(ref t) if t == "hello"));
        assert!(matches!(blocks[1], CodeBlock::Text(ref t) if t == "```rust\nlet x = 1;"));
    }

    #[test]
    fn extract_code_blocks_handles_lists() {
        let text = "items:\n- one\n- two\n";
        let blocks = extract_code_blocks(text);
        assert_eq!(blocks.len(), 2);
        assert!(matches!(blocks[0], CodeBlock::Text(ref t) if t == "items:"));
        assert!(
            matches!(blocks[1], CodeBlock::List { ordered, ref items } if !ordered && items.len() == 2)
        );
    }

    #[test]
    fn extract_code_blocks_handles_blockquote() {
        let text = "> quote line 1\n> quote line 2";
        let blocks = extract_code_blocks(text);
        assert_eq!(blocks.len(), 1);
        assert!(matches!(blocks[0], CodeBlock::Blockquote(ref q) if q.lines().count() == 2));
    }
}
