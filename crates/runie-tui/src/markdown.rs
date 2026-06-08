//! Markdown parsing for agent messages

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use crate::theme::C;

/// Parsed inline markdown span for styling.
#[derive(Debug, Clone, PartialEq)]
pub struct MdSpan {
    pub content: String,
    pub style: Style,
}

/// Parse inline markdown bold (**text**), italic (*text*), and code (`text`) into styled spans.
pub fn parse_inline_markdown(text: &str) -> Vec<MdSpan> {
    parse_inline_markdown_with_color(text, C.fg_bright)
}

/// Parse inline markdown with a custom base foreground color.
pub fn parse_inline_markdown_with_color(text: &str, base_color: Color) -> Vec<MdSpan> {
    let mut spans = Vec::new();
    let mut chars = text.chars().peekable();
    let mut current = String::new();
    let base = Style::default().fg(base_color);

    while let Some(c) = chars.next() {
        if c == '*' && chars.peek() == Some(&'*') {
            chars.next();
            flush_plain(&mut current, &mut spans, base);
            if let Some((found, inner)) = parse_delimited(&mut chars, "**") {
                if found {
                    spans.push(MdSpan { content: inner, style: base.add_modifier(Modifier::BOLD) });
                } else {
                    current.push_str("**");
                    current.push_str(&inner);
                }
            }
        } else if c == '`' {
            flush_plain(&mut current, &mut spans, base);
            let mut code_text = String::new();
            while let Some(cc) = chars.next() {
                if cc == '`' {
                    break;
                }
                code_text.push(cc);
            }
            spans.push(MdSpan {
                content: code_text,
                style: Style::default().fg(C.accent).bg(C.code_bg),
            });
        } else if c == '*' {
            flush_plain(&mut current, &mut spans, base);
            if let Some((found, inner)) = parse_delimited(&mut chars, "*") {
                if found {
                    spans.push(MdSpan { content: inner, style: base.add_modifier(Modifier::ITALIC) });
                } else {
                    current.push('*');
                    current.push_str(&inner);
                }
            }
        } else {
            current.push(c);
        }
    }
    flush_plain(&mut current, &mut spans, base);
    spans
}

fn parse_delimited(chars: &mut std::iter::Peekable<impl Iterator<Item = char>>, delim: &str) -> Option<(bool, String)> {
    let mut text = String::new();
    while let Some(c) = chars.next() {
        if delim == "*" && c == '*' {
            return Some((true, text));
        }
        if delim == "**" && c == '*' && chars.peek() == Some(&'*') {
            chars.next();
            return Some((true, text));
        }
        text.push(c);
    }
    Some((false, text))
}

fn flush_plain(current: &mut String, spans: &mut Vec<MdSpan>, style: Style) {
    if !current.is_empty() {
        spans.push(MdSpan {
            content: std::mem::take(current),
            style,
        });
    }
}

/// A block of text or code extracted from markdown.
#[derive(Debug, Clone, PartialEq)]
pub enum CodeBlock {
    Text(String),
    Code { lang: String, content: String },
}

/// Extract code blocks (``` fenced) from text.
/// Returns vec of Text/Code segments in order.
pub fn extract_code_blocks(text: &str) -> Vec<CodeBlock> {
    let mut blocks = Vec::new();
    let mut in_code = false;
    let mut lang = String::new();
    let mut code_lines: Vec<&str> = Vec::new();
    let mut text_lines: Vec<&str> = Vec::new();

    for line in text.lines() {
        if line.starts_with("```") {
            if in_code {
                // End of code block
                if !text_lines.is_empty() {
                    blocks.push(CodeBlock::Text(text_lines.join("\n")));
                    text_lines.clear();
                }
                blocks.push(CodeBlock::Code {
                    lang: std::mem::take(&mut lang),
                    content: code_lines.join("\n"),
                });
                code_lines.clear();
                in_code = false;
            } else {
                // Start of code block
                if !text_lines.is_empty() {
                    blocks.push(CodeBlock::Text(text_lines.join("\n")));
                    text_lines.clear();
                }
                lang = line[3..].trim().to_string();
                in_code = true;
            }
        } else if in_code {
            code_lines.push(line);
        } else {
            text_lines.push(line);
        }
    }

    // Handle remaining lines
    if in_code {
        // Unclosed code block — treat as text
        text_lines.push("```");
        text_lines.extend(code_lines);
        blocks.push(CodeBlock::Text(text_lines.join("\n")));
    } else if !text_lines.is_empty() {
        blocks.push(CodeBlock::Text(text_lines.join("\n")));
    }

    blocks
}

/// Convert MdSpan slices to ratatui Spans.
pub fn md_to_spans(md_spans: &[MdSpan]) -> Vec<Span<'static>> {
    md_spans
        .iter()
        .map(|s| Span::styled(s.content.clone(), s.style))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_code_parsed() {
        let spans = parse_inline_markdown("use `cargo test` to run");
        let code = spans.iter().find(|s| s.content == "cargo test");
        assert!(code.is_some(), "Should have code span 'cargo test'");
        assert_eq!(code.unwrap().style.fg, Some(C.accent));
        assert!(code.unwrap().style.bg.is_some(), "Code should have bg");
    }

    #[test]
    fn bold_text_parsed() {
        let spans = parse_inline_markdown("hello **world** test");
        let bold = spans.iter().find(|s| s.content == "world");
        assert!(bold.is_some());
        assert_eq!(bold.unwrap().style.add_modifier, Modifier::BOLD);
    }

    #[test]
    fn italic_text_parsed() {
        let spans = parse_inline_markdown("hello *world* test");
        let italic = spans.iter().find(|s| s.content == "world");
        assert!(italic.is_some());
        assert_eq!(italic.unwrap().style.add_modifier, Modifier::ITALIC);
    }

    #[test]
    fn detects_code_block() {
        let text = "Some text\n```rust\nfn main() {}\n```\nMore text";
        let blocks = extract_code_blocks(text);
        assert_eq!(blocks.len(), 3);
        assert!(matches!(&blocks[0], CodeBlock::Text(_)));
        assert!(matches!(&blocks[1], CodeBlock::Code { lang, .. } if lang == "rust"));
        assert!(matches!(&blocks[2], CodeBlock::Text(_)));
    }

    #[test]
    fn unclosed_code_block_becomes_text() {
        let text = "```rust\nfn main() {}";
        let blocks = extract_code_blocks(text);
        assert_eq!(blocks.len(), 1);
        assert!(matches!(&blocks[0], CodeBlock::Text(t) if t.contains("fn main"))
        );
    }
}
