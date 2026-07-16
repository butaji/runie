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
        "J/K".to_owned(),
        "G/Shift+G".to_owned(),
        "h/l turn".to_owned(),
        "K/J anchor".to_owned(),
        "Enter detail".to_owned(),
        "Q quit".to_owned(),
        "Space/I".to_owned(),
        "Esc".to_owned(),
    ]
}

pub fn at_suggestion_hints() -> Vec<String> {
    vec![
        "Tab cycle".to_owned(),
        "Enter insert".to_owned(),
        "Esc close".to_owned(),
    ]
}

pub fn input_active_hints() -> Vec<String> {
    vec![
        "Enter send".to_owned(),
        "Alt+Enter follow-up".to_owned(),
        "Esc clear".to_owned(),
    ]
}

pub fn empty_input_hints() -> Vec<String> {
    vec!["Alt+Enter follow-up".to_owned(), "Esc clear".to_owned()]
}

/// Modal navigation hints (command palette, model selector, settings, etc.).
pub fn modal_hints() -> Vec<String> {
    vec![
        "↑/↓ select".to_owned(),
        "Enter confirm".to_owned(),
        "Esc close".to_owned(),
    ]
}
