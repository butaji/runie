//! Markdown parsing for agent messages.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

pub use runie_core::markdown::{extract_code_blocks, CodeBlock};

use crate::theme::{color_accent, color_code_bg, color_fg_bright};

/// Parsed inline markdown span for styling.
#[derive(Debug, Clone, PartialEq)]
pub struct MdSpan {
    pub content: String,
    pub style: Style,
}

/// Parse inline markdown bold (**text**), italic (*text*), and code (`text`) into styled spans.
pub fn parse_inline_markdown(text: &str) -> Vec<MdSpan> {
    parse_inline_markdown_with_color(text, color_fg_bright())
}

fn md_options() -> pulldown_cmark::Options {
    pulldown_cmark::Options::ENABLE_STRIKETHROUGH
        | pulldown_cmark::Options::ENABLE_TABLES
        | pulldown_cmark::Options::ENABLE_TASKLISTS
}

/// Parse inline markdown with a custom base foreground color.
pub fn parse_inline_markdown_with_color(text: &str, base_color: Color) -> Vec<MdSpan> {
    let parser = pulldown_cmark::Parser::new_ext(text, md_options());
    let base = Style::default().fg(base_color);
    let code_style = Style::default().fg(color_accent()).bg(color_code_bg());
    let mut spans: Vec<MdSpan> = Vec::new();
    let mut style_stack: Vec<Style> = vec![base];

    let mut writer = SpanWriter::new(&mut spans);

    for event in parser {
        handle_inline_event(event, &mut style_stack, &mut writer, code_style);
    }
    spans
}

fn handle_inline_event(
    event: pulldown_cmark::Event<'_>,
    style_stack: &mut Vec<Style>,
    writer: &mut SpanWriter<'_>,
    code_style: Style,
) {
    match event {
        pulldown_cmark::Event::Text(text) => writer.push(&text, current_style(style_stack)),
        pulldown_cmark::Event::Code(code) => writer.push(&code, code_style),
        pulldown_cmark::Event::SoftBreak | pulldown_cmark::Event::HardBreak => {
            writer.push("\n", current_style(style_stack))
        }
        pulldown_cmark::Event::Start(tag) => {
            style_stack.push(style_for_tag(tag, current_style(style_stack)));
        }
        pulldown_cmark::Event::End(_) => {
            style_stack.pop();
        }
        _ => {}
    }
}

fn current_style(stack: &[Style]) -> Style {
    *stack.last().unwrap_or(&Style::default())
}

fn style_for_tag(tag: pulldown_cmark::Tag<'_>, current: Style) -> Style {
    match tag {
        pulldown_cmark::Tag::Strong => current.add_modifier(Modifier::BOLD),
        pulldown_cmark::Tag::Emphasis => current.add_modifier(Modifier::ITALIC),
        pulldown_cmark::Tag::Strikethrough => current.add_modifier(Modifier::CROSSED_OUT),
        _ => current,
    }
}

struct SpanWriter<'a> {
    spans: &'a mut Vec<MdSpan>,
}

impl<'a> SpanWriter<'a> {
    fn new(spans: &'a mut Vec<MdSpan>) -> Self {
        Self { spans }
    }

    fn push(&mut self, text: &str, style: Style) {
        if text.is_empty() {
            return;
        }
        if let Some(last) = self.spans.last_mut() {
            if last.style == style {
                last.content.push_str(text);
                return;
            }
        }
        self.spans.push(MdSpan {
            content: text.to_string(),
            style,
        });
    }
}

/// Convert MdSpan slices to ratatui Spans.
pub fn md_to_spans(md_spans: &[MdSpan]) -> Vec<Span<'static>> {
    md_spans
        .iter()
        .map(|s| Span::styled(s.content.clone(), s.style))
        .collect()
}
