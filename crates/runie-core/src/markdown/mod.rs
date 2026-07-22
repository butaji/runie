//! Unified markdown AST shared between core (line-count math) and the TUI
//! (styled rendering).
//!
//! Single parsing pass produces both block structure and inline spans,
//! so line counts in core stay in sync with rendered output in the TUI.

mod heal;
pub(crate) mod parsing;
#[cfg(test)]
mod tests;

pub use heal::heal_markdown;
pub use parsing::{extract_blocks, inlines_to_text, parse_inline_spans};

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

    /// True for soft or hard line breaks.
    pub fn is_break(&self) -> bool {
        matches!(self, MdInline::SoftBreak | MdInline::HardBreak)
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
    Code {
        lang: String,
        content: String,
    },
    List {
        ordered: bool,
        /// Each list item is a list of inline spans.
        items: Vec<Vec<MdInline>>,
    },
    /// A blockquote with inline spans for styling.
    Blockquote(Vec<MdInline>),
}

/// Legacy alias so existing call sites don't break.
pub use CodeBlock as Block;

/// Parse markdown into a list of blocks with inline spans extracted.
/// Single pass — both block structure and inline styling are computed together.
pub fn parse_markdown(text: &str) -> Vec<CodeBlock> {
    let (parse_text, trailing) = parsing::split_unclosed_fence(text);
    let mut blocks = parsing::extract_blocks(parse_text);
    if let Some(t) = trailing {
        let inlines = parsing::parse_inline_spans(t);
        blocks.push(CodeBlock::Text { content: t.to_owned(), inlines });
    }
    blocks
}

/// Extract code blocks, lists, and blockquotes from text, together with
/// inline spans for text sections.
/// Kept for backward compatibility; prefer `parse_markdown` for new code.
pub fn extract_code_blocks(text: &str) -> Vec<CodeBlock> {
    parse_markdown(text)
}

// ── Legacy helpers ─────────────────────────────────────────────────────────

/// Return the plain-text content of a Text block (for layout.rs line math).
pub fn text_block_content(block: &CodeBlock) -> Option<&str> {
    match block {
        CodeBlock::Text { content, .. } => Some(content),
        _ => None,
    }
}
