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
    List { ordered: bool, items: Vec<String> },
    Blockquote(String),
}

/// Extract code blocks (``` fenced) from text.
/// Returns vec of Text/Code segments in order.
fn flush_text(lines: &mut Vec<&str>, blocks: &mut Vec<CodeBlock>) {
    if !lines.is_empty() {
        blocks.push(CodeBlock::Text(lines.join("\n")));
        lines.clear();
    }
}

pub fn extract_code_blocks(text: &str) -> Vec<CodeBlock> {
    let mut blocks = Vec::new();
    let mut in_code = false;
    let mut lang = String::new();
    let mut code_lines: Vec<&str> = Vec::new();
    let mut text_lines: Vec<&str> = Vec::new();
    let mut in_list = false;
    let mut list_ordered = false;
    let mut list_items: Vec<String> = Vec::new();
    let mut current_list_item = String::new();
    let mut in_blockquote = false;
    let mut blockquote_lines: Vec<&str> = Vec::new();

    fn flush_list(items: &mut Vec<String>, blocks: &mut Vec<CodeBlock>, ordered: bool) {
        if !items.is_empty() {
            blocks.push(CodeBlock::List { ordered, items: std::mem::take(items) });
        }
    }

    fn flush_blockquote(lines: &mut Vec<&str>, blocks: &mut Vec<CodeBlock>) {
        if !lines.is_empty() {
            blocks.push(CodeBlock::Blockquote(lines.join("\n")));
            lines.clear();
        }
    }

    for line in text.lines() {
        // Code blocks (``` fenced)
        if line.starts_with("```") {
            flush_text(&mut text_lines, &mut blocks);
            flush_list(&mut list_items, &mut blocks, list_ordered);
            flush_blockquote(&mut blockquote_lines, &mut blocks);
            if in_code {
                blocks.push(CodeBlock::Code {
                    lang: std::mem::take(&mut lang),
                    content: code_lines.join("\n"),
                });
                code_lines.clear();
            } else {
                lang = line[3..].trim().to_string();
            }
            in_code = !in_code;
            continue;
        }

        if in_code {
            code_lines.push(line);
            continue;
        }

        // Blockquotes (> )
        if line.starts_with("> ") {
            flush_text(&mut text_lines, &mut blocks);
            flush_list(&mut list_items, &mut blocks, list_ordered);
            blockquote_lines.push(&line[2..]);
            in_blockquote = true;
            continue;
        } else if in_blockquote && line.starts_with(">") {
            blockquote_lines.push(line.trim_start_matches('>').trim_start());
            continue;
        } else if in_blockquote {
            flush_blockquote(&mut blockquote_lines, &mut blocks);
            in_blockquote = false;
        }

        // Lists (- item, * item, 1. item, 2. item)
        let is_unordered = line.starts_with("- ") || line.starts_with("* ");
        let is_ordered = line.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
            && line.contains('.');

        if is_unordered || is_ordered {
            flush_text(&mut text_lines, &mut blocks);
            flush_blockquote(&mut blockquote_lines, &mut blocks);
            
            // Check if this continues a list
            let item_text = if is_unordered {
                line[2..].trim().to_string()
            } else {
                // "1. item" -> "item"
                line.split_once('.').map(|(_, rest)| rest.trim().to_string()).unwrap_or_default()
            };
            let this_ordered = is_ordered;
            
            if in_list && list_ordered == this_ordered {
                // Continue current list
                if !current_list_item.is_empty() {
                    list_items.push(std::mem::take(&mut current_list_item));
                }
                current_list_item = item_text;
            } else {
                // Start new list
                flush_list(&mut list_items, &mut blocks, list_ordered);
                list_ordered = this_ordered;
                current_list_item = item_text;
                in_list = true;
            }
            continue;
        } else if in_list {
            // Non-list line - check if it's a continuation of the current item
            let trimmed = line.trim_start();
            if !trimmed.is_empty() && !trimmed.starts_with("> ") {
                // Check if this looks like continuation (indented)
                let leading_spaces = line.len() - line.trim_start().len();
                if leading_spaces >= 2 {
                    current_list_item.push_str(" ");
                    current_list_item.push_str(trimmed);
                    continue;
                }
            }
            // End of list
            if !current_list_item.is_empty() {
                list_items.push(std::mem::take(&mut current_list_item));
            }
            flush_list(&mut list_items, &mut blocks, list_ordered);
            in_list = false;
        }

        text_lines.push(line);
    }

    // Flush remaining content
    if in_code {
        text_lines.push("```");
        text_lines.extend(code_lines);
    }
    if in_list {
        if !current_list_item.is_empty() {
            list_items.push(current_list_item);
        }
        flush_list(&mut list_items, &mut blocks, list_ordered);
    }
    if in_blockquote {
        flush_blockquote(&mut blockquote_lines, &mut blocks);
    }
    flush_text(&mut text_lines, &mut blocks);
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
