//! Free helper functions for input text editing.

use unicode_segmentation::UnicodeSegmentation;

pub(crate) fn is_quit_command(content: &str) -> bool {
    content.eq_ignore_ascii_case("quit")
        || content.eq_ignore_ascii_case("exit")
        || content.eq_ignore_ascii_case(":q")
}

pub(crate) fn prev_grapheme_boundary(s: &str, pos: usize) -> usize {
    let mut last = 0;
    for (i, _) in s.grapheme_indices(true) {
        if i >= pos {
            break;
        }
        last = i;
    }
    last
}

pub(crate) fn next_grapheme_boundary(s: &str, pos: usize) -> usize {
    for (i, _) in s.grapheme_indices(true) {
        if i > pos {
            return i;
        }
    }
    s.len()
}

pub(crate) fn find_word_boundary_left(s: &str, pos: usize) -> usize {
    let mut pos = pos;
    while pos > 0 {
        let prev = prev_grapheme_boundary(s, pos);
        if &s[prev..pos] != " " {
            break;
        }
        pos = prev;
    }
    while pos > 0 {
        let prev = prev_grapheme_boundary(s, pos);
        if &s[prev..pos] == " " {
            break;
        }
        pos = prev;
    }
    pos
}

pub(crate) fn find_word_boundary_right(s: &str, pos: usize) -> usize {
    let mut pos = pos;
    let len = s.len();
    while pos < len {
        let next = next_grapheme_boundary(s, pos);
        if &s[pos..next] == " " {
            break;
        }
        pos = next;
    }
    while pos < len {
        let next = next_grapheme_boundary(s, pos);
        if &s[pos..next] != " " {
            break;
        }
        pos = next;
    }
    pos
}

pub(crate) fn vim_nav_hints() -> Vec<String> {
    vec![
        "j down · k up".to_string(),
        "g/G top/bottom".to_string(),
        "space/i input".to_string(),
        "esc input".to_string(),
    ]
}

pub(crate) fn at_suggestion_hints() -> Vec<String> {
    vec![
        "tab cycle".to_string(),
        "enter insert".to_string(),
        "esc close".to_string(),
    ]
}

pub(crate) fn input_active_hints() -> Vec<String> {
    vec![
        "enter send".to_string(),
        "alt+enter follow-up".to_string(),
        "esc clear".to_string(),
    ]
}

pub(crate) fn empty_input_hints() -> Vec<String> {
    vec!["alt+enter follow-up".to_string(), "esc clear".to_string()]
}
