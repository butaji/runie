use pulldown_cmark::{Event, Options, Parser, Tag};

use super::MdInline;

/// Parse inline markdown spans (bold, italic, code, breaks) from plain text.
pub fn parse_inline_spans(text: &str) -> Vec<MdInline> {
    let parser = Parser::new_ext(text, md_options());
    parse_spans_from_events(parser)
}

/// Parser options: standard markdown plus tables, strikethrough, tasklists.
pub fn md_options() -> Options {
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
pub fn inlines_to_text(inlines: &[MdInline]) -> String {
    use std::fmt::Write;

    let mut out = String::new();
    // Track active style and the outer style (style before entering nested).
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
                // Text: just output the text. Markers are emitted by styled spans.
                out.push_str(s);
                // Text is unstyled, so reset current
                current = Style::None;
            }
            MdInline::Bold(s) | MdInline::Italic(s) | MdInline::Strike(s) => {
                // Check if we're re-entering the outer style (nested completed).
                // If new_style == outer, we're returning from a nested span.
                if new_style == outer {
                    // Re-entering outer: just output content.
                    out.push_str(s);
                    current = new_style;
                } else {
                    // Entering new (possibly nested) style.
                    // Save current as outer before entering.
                    outer = current;
                    out.write_str(new_style.marker()).unwrap();
                    out.push_str(s);
                    out.write_str(new_style.marker()).unwrap();
                    current = new_style;
                }
            }
            MdInline::Code(s) => {
                // Code: temporarily exit style context.
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

    // Close remaining style
    if current != Style::None {
        out.write_str(current.marker()).unwrap();
    }

    out
}
