//! Support helpers (from input_support.rs).

use unicode_segmentation::UnicodeSegmentation;

pub fn is_quit_command(content: &str) -> bool {
    content.eq_ignore_ascii_case("quit")
        || content.eq_ignore_ascii_case("exit")
        || content.eq_ignore_ascii_case(":q")
}

pub fn prev_grapheme_boundary(s: &str, pos: usize) -> usize {
    let mut last = 0;
    for (i, _) in s.grapheme_indices(true) {
        if i >= pos {
            break;
        }
        last = i;
    }
    last
}

pub fn next_grapheme_boundary(s: &str, pos: usize) -> usize {
    for (i, _) in s.grapheme_indices(true) {
        if i > pos {
            return i;
        }
    }
    s.len()
}

pub fn find_word_boundary_left(s: &str, pos: usize) -> usize {
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

pub fn find_word_boundary_right(s: &str, pos: usize) -> usize {
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

pub fn vim_nav_hints() -> Vec<String> {
    vec![
        "j/k".to_owned(),
        "g/G".to_owned(),
        "enter expand".to_owned(),
        "q quit".to_owned(),
        "space/i".to_owned(),
        "esc".to_owned(),
    ]
}

pub fn at_suggestion_hints() -> Vec<String> {
    vec![
        "tab cycle".to_owned(),
        "enter insert".to_owned(),
        "esc close".to_owned(),
    ]
}

pub fn input_active_hints() -> Vec<String> {
    vec![
        "enter send".to_owned(),
        "alt+enter follow-up".to_owned(),
        "esc clear".to_owned(),
    ]
}

pub fn empty_input_hints() -> Vec<String> {
    vec!["alt+enter follow-up".to_owned(), "esc clear".to_owned()]
}

/// Modal navigation hints (command palette, model selector, settings, etc.).
pub fn modal_hints() -> Vec<String> {
    vec![
        "↑/↓ select".to_owned(),
        "enter confirm".to_owned(),
        "esc close".to_owned(),
    ]
}
