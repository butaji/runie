use pulldown_cmark::{Event, Options, Parser, Tag};

use super::MdInline;

/// Parse inline markdown spans (bold, italic, code, breaks) from plain text.
pub fn parse_inline_spans(text: &str) -> Vec<MdInline> {
    let parser = Parser::new_ext(text, md_options());
    parse_spans_from_events(parser)
}

pub(crate) fn md_options() -> Options {
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
            // intentionally ignored: other events fall through
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
