//! Markdown healing: close unclosed inline syntax for stable display.
//!
//! Two-pass approach:
//! 1. Parse with `pulldown_cmark` → build a style stack to track which
//!    inline markers are open at the end of the document.
//!    (pulldown_cmark auto-closes some markers, so we can't infer openers
//!    from the event stream alone.)
//! 2. Scan the original text character-by-character → track openers that
//!    were explicitly opened but never closed.
//! 3. Output the original text + closers for the unclosed openers from (2).
#![allow(clippy::too_many_lines)]

/// Close unclosed inline markdown syntax in `text`.
///
/// Handles: bold (`**`/`__`), italic (`*`/`_`), strikethrough (`~~`),
/// inline code (`` ` ``), and links (`[` without matching `](url)`).
pub fn heal_markdown(text: &str) -> String {
    // Pass 1: use pulldown_cmark to determine which style markers are open.
    let style_stack = parse_style_stack(text);

    // Pass 2: scan original text to find openers that were not closed.
    let raw_openers = scan_raw_openers(text);

    // Combine: the unclosed markers are those from the raw scan.
    // The pulldown_cmark style stack tells us the nesting order.
    let mut result = text.to_string();
    append_closers(&mut result, &raw_openers, &style_stack);
    result
}

/// Track which inline markers are explicitly opened in the original text.
#[derive(Default)]
struct RawOpeners {
    style_stack: Vec<StyleMarker>,
    code_n: usize,
    link_open: bool,
}

/// Style marker with the actual delimiter character used.
#[derive(Clone, Debug, PartialEq, Eq)]
enum StyleMarker {
    Bold,
    Italic(char), // Track which delimiter: '*' or '_'
    Strike,
}

/// Scan the original text to track which markers were opened but never closed.
#[allow(clippy::cognitive_complexity)]
fn scan_raw_openers(text: &str) -> RawOpeners {
    let mut openers = RawOpeners::default();
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '`' => {
                let n = count_run(&mut chars, '`', 1);
                if n == 3 {
                    // Triple backtick: code block delimiter, not inline
                    if openers.code_n > 0 {
                        openers.code_n = 0;
                    }
                } else if openers.code_n > 0 && openers.code_n == n {
                    // Closing backtick(s) matching opener: clear
                    openers.code_n = 0;
                } else {
                    // Opening or mismatch: set new count
                    openers.code_n = n;
                }
            }
            '*' | '_' => {
                if chars.peek() == Some(&c) {
                    chars.next();
                    // `**` or `__` = Bold
                    if matches!(openers.style_stack.last(), Some(StyleMarker::Bold)) {
                        openers.style_stack.pop();
                    } else {
                        openers.style_stack.push(StyleMarker::Bold);
                    }
                } else {
                    // `*` or `_` = Italic
                    if matches!(openers.style_stack.last(), Some(StyleMarker::Italic(_))) {
                        openers.style_stack.pop();
                    } else {
                        openers.style_stack.push(StyleMarker::Italic(c));
                    }
                }
            }
            '~' => {
                if chars.peek() == Some(&'~') {
                    chars.next();
                    if matches!(openers.style_stack.last(), Some(StyleMarker::Strike)) {
                        openers.style_stack.pop();
                    } else {
                        openers.style_stack.push(StyleMarker::Strike);
                    }
                }
            }
            '[' => openers.link_open = true,
            ']' => {
                if openers.link_open {
                    openers.link_open = false;
                }
            }
            '(' => {}
            ')' => {}
            _ => {}
        }
    }

    openers
}

/// Parse with pulldown_cmark to build the current style stack at end-of-document.
fn parse_style_stack(text: &str) -> Vec<StyleMarker> {
    use pulldown_cmark::{Event, Parser, Tag};

    let mut style_stack = Vec::new();
    for event in Parser::new(text) {
        match event {
            Event::Start(Tag::Strong) => style_stack.push(StyleMarker::Bold),
            Event::Start(Tag::Emphasis) => style_stack.push(StyleMarker::Italic('*')),
            Event::Start(Tag::Strikethrough) => style_stack.push(StyleMarker::Strike),
            Event::End(
                pulldown_cmark::TagEnd::Strong
                | pulldown_cmark::TagEnd::Emphasis
                | pulldown_cmark::TagEnd::Strikethrough,
            ) => {
                style_stack.pop();
            }
            _ => {}
        }
    }
    style_stack
}

/// Append closing markers for unclosed openers.
fn append_closers(result: &mut String, openers: &RawOpeners, _style_stack: &[StyleMarker]) {
    // Close code spans first (innermost).
    if openers.code_n > 0 {
        result.push_str(&"`".repeat(openers.code_n));
    }
    // Close styles in reverse order (outermost first).
    for marker in openers.style_stack.iter().rev() {
        match marker {
            StyleMarker::Bold => result.push_str("**"),
            StyleMarker::Italic(c) => result.push(*c),
            StyleMarker::Strike => result.push_str("~~"),
        }
    }
    // Close link.
    if openers.link_open {
        result.push_str("]()");
    }
}

fn count_run(chars: &mut std::iter::Peekable<std::str::Chars>, target: char, min: usize) -> usize {
    let mut n = min;
    while chars.peek() == Some(&target) {
        chars.next();
        n += 1;
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heal_markdown_closes_unclosed_bold() {
        assert_eq!(heal_markdown("hello **world"), "hello **world**");
        assert_eq!(heal_markdown("**bold start"), "**bold start**");
    }

    #[test]
    fn heal_markdown_closes_unclosed_italic() {
        assert_eq!(heal_markdown("hello *world"), "hello *world*");
        assert_eq!(heal_markdown("hello _world"), "hello _world_");
        assert_eq!(heal_markdown("italic _start"), "italic _start_");
    }

    #[test]
    fn heal_markdown_closes_unclosed_inline_code() {
        assert_eq!(heal_markdown("use `rust"), "use `rust`");
        assert_eq!(heal_markdown("code `snippet"), "code `snippet`");
        assert_eq!(heal_markdown("hello ``world"), "hello ``world``");
    }

    #[test]
    fn heal_markdown_closes_unclosed_link() {
        assert_eq!(heal_markdown("see [docs"), "see [docs]()");
        assert_eq!(heal_markdown("[link"), "[link]()");
    }

    #[test]
    fn heal_markdown_leaves_closed_syntax_unchanged() {
        assert_eq!(
            heal_markdown("hello **world** and `code`"),
            "hello **world** and `code`"
        );
        assert_eq!(
            heal_markdown("**bold** and *italic* and `code`"),
            "**bold** and *italic* and `code`"
        );
    }

    #[test]
    fn heal_markdown_leaves_plain_text_unchanged() {
        assert_eq!(heal_markdown("just plain text"), "just plain text");
        assert_eq!(heal_markdown(""), "");
    }

    #[test]
    fn heal_markdown_handles_multiple_unclosed_spans() {
        assert_eq!(heal_markdown("**bold and `code"), "**bold and `code`**");
    }

    #[test]
    fn heal_markdown_strikethrough_unclosed() {
        assert_eq!(
            heal_markdown("hello ~~strikethrough"),
            "hello ~~strikethrough~~"
        );
    }

    #[test]
    fn heal_markdown_mixed_content() {
        // Mixed: some closed, some unclosed
        assert_eq!(
            heal_markdown("plain **bold *italic* text"),
            "plain **bold *italic* text**"
        );
    }

    #[test]
    fn heal_markdown_balanced_stays_unchanged() {
        assert_eq!(
            heal_markdown("**bold** and *italic* and ~~strike~~"),
            "**bold** and *italic* and ~~strike~~"
        );
    }
}
