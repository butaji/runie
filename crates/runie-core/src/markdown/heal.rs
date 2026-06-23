//! Markdown healing: close unclosed inline syntax for stable display.

/// Close unclosed inline markdown syntax in `text`.
///
/// Handles: bold (`**`/`__`), italic (`*`/`_`), inline code (`` ` ``), and links (`[`).
pub fn heal_markdown(text: &str) -> String {
    let mut state = ParseState::default();
    let out = heal_loop(text, &mut state);
    append_closers(out, &state)
}

struct ParseState {
    bold_open: bool,
    italic_open: Option<char>,
    italic_ever: bool,
    code_n: usize,
    link_open: bool,
    link_await_close: bool,
}

impl Default for ParseState {
    fn default() -> Self {
        Self {
            bold_open: false,
            italic_open: None,
            italic_ever: false,
            code_n: 0,
            link_open: false,
            link_await_close: false,
        }
    }
}

fn heal_loop(text: &str, state: &mut ParseState) -> String {
    let mut out = String::with_capacity(text.len() + 16);
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '`' => heal_backtick(&mut chars, &mut out, state),
            '*' | '_' => heal_star_underscore(c, &mut chars, &mut out, state),
            '[' => {
                out.push('[');
                state.link_open = true;
                state.link_await_close = false;
            }
            ']' => heal_close_bracket(&mut chars, &mut out, state),
            '(' | ')' => heal_paren(c, state, &mut out),
            _ => out.push(c),
        }
    }
    out
}

fn heal_star_underscore(c: char, chars: &mut std::iter::Peekable<std::str::Chars>, out: &mut String, state: &mut ParseState) {
    if chars.peek() == Some(&c) {
        chars.next();
        out.push(c);
        out.push(c);
        state.bold_open = !state.bold_open;
    } else {
        out.push(c);
        if state.italic_open.is_some() {
            state.italic_open = None;
        } else {
            state.italic_open = Some(c);
            state.italic_ever = true;
        }
    }
}

fn heal_paren(c: char, state: &mut ParseState, out: &mut String) {
    if state.link_await_close {
        state.link_await_close = false;
    }
    out.push(c);
}

fn heal_backtick(chars: &mut std::iter::Peekable<std::str::Chars>, out: &mut String, state: &mut ParseState) {
    let n = count_run(chars, '`', 1);
    out.push_str(&"`".repeat(n));
    state.code_n = if state.code_n == n { 0 } else { n };
}

fn heal_close_bracket(chars: &mut std::iter::Peekable<std::str::Chars>, out: &mut String, state: &mut ParseState) {
    out.push(']');
    if state.link_open {
        state.link_open = false;
        state.link_await_close = true;
        if chars.peek() != Some(&'(') {
            out.push_str("]()");
        }
    }
}

fn append_closers(mut out: String, state: &ParseState) -> String {
    if state.link_await_close || state.link_open {
        out.push_str("]()");
    }
    if state.bold_open {
        out.push_str("**");
    }
    if let Some(c) = state.italic_open {
        out.push(c);
    }
    if state.code_n > 0 {
        out.push_str(&"`".repeat(state.code_n));
    }
    out
}

fn count_run(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    target: char,
    min: usize,
) -> usize {
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
    }

    #[test]
    fn heal_markdown_closes_unclosed_italic() {
        assert_eq!(heal_markdown("hello *world"), "hello *world*");
        assert_eq!(heal_markdown("hello _world"), "hello _world_");
    }

    #[test]
    fn heal_markdown_closes_unclosed_inline_code() {
        assert_eq!(heal_markdown("hello `world"), "hello `world`");
        assert_eq!(heal_markdown("hello ``world"), "hello ``world``");
    }

    #[test]
    fn heal_markdown_closes_unclosed_link() {
        assert_eq!(heal_markdown("hello [world"), "hello [world]()");
    }

    #[test]
    fn heal_markdown_leaves_closed_syntax_unchanged() {
        assert_eq!(heal_markdown("hello **world**"), "hello **world**");
        assert_eq!(heal_markdown("hello *world*"), "hello *world*");
        assert_eq!(heal_markdown("hello `world`"), "hello `world`");
        assert_eq!(heal_markdown("hello [world](url)"), "hello [world](url)");
    }

    #[test]
    fn heal_markdown_leaves_plain_text_unchanged() {
        assert_eq!(heal_markdown("hello world"), "hello world");
        assert_eq!(heal_markdown(""), "");
    }

    #[test]
    fn heal_markdown_handles_multiple_unclosed_spans() {
        // All three spans are unclosed: bold (**), italic (*), code (`)
        // Healing closes all of them
        assert_eq!(
            heal_markdown("**bold and *italic and `code"),
            "**bold and *italic and `code***`"
        );
    }
}
