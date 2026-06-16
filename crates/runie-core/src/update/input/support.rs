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
        "j/k".to_string(),
        "g/G".to_string(),
        "enter expand".to_string(),
        "q quit".to_string(),
        "space/i".to_string(),
        "esc".to_string(),
    ]
}

pub fn feed_focused_hints() -> Vec<String> {
    vec![
        "j/k".to_string(),
        "enter expand".to_string(),
        "q quit".to_string(),
    ]
}

pub fn at_suggestion_hints() -> Vec<String> {
    vec![
        "tab cycle".to_string(),
        "enter insert".to_string(),
        "esc close".to_string(),
    ]
}

pub fn input_active_hints() -> Vec<String> {
    vec![
        "enter send".to_string(),
        "alt+enter follow-up".to_string(),
        "esc clear".to_string(),
    ]
}

pub fn empty_input_hints() -> Vec<String> {
    vec!["alt+enter follow-up".to_string(), "esc clear".to_string()]
}

/// Team mode subagent sidebar hotkeys.
pub fn team_mode_hints() -> Vec<String> {
    vec![
        "ctrl+0 orchestrator".to_string(),
        "ctrl+1..9 agents".to_string(),
    ]
}

/// Modal navigation hints (command palette, model selector, settings, etc.).
pub fn modal_hints() -> Vec<String> {
    vec![
        "↑/↓ select".to_string(),
        "enter confirm".to_string(),
        "esc close".to_string(),
    ]
}
