use crate::commands::filter_commands;
use crate::event::{DialogEvent, Event};
use crate::model::AppState;

use super::palette_stack;

#[test]
fn filter_empty_shows_all() {
    let state = AppState::default();
    let all = state.registry.list();
    let filtered = filter_commands(&state.registry, "");
    assert_eq!(filtered.len(), all.len());
}

#[test]
fn filter_matches_name() {
    let state = AppState::default();
    let filtered = filter_commands(&state.registry, "comp");
    assert!(
        filtered.iter().any(|c| c.name == "compact"),
        "'comp' should match 'compact'"
    );
}

#[test]
fn filter_matches_description() {
    let state = AppState::default();
    let filtered = filter_commands(&state.registry, "copy");
    assert!(
        filtered.iter().any(|c| c.name == "copy"),
        "'copy' should match 'copy' command description"
    );
}

#[test]
fn filter_case_insensitive() {
    let state = AppState::default();
    let lower = filter_commands(&state.registry, "comp");
    let upper = filter_commands(&state.registry, "COMP");
    assert_eq!(lower.len(), upper.len());
    assert!(upper.iter().any(|c| c.name == "compact"));
}

#[test]
fn select_wraps_up() {
    let mut state = AppState::default();
    state.update(Event::Dialog(DialogEvent::ToggleCommandPalette));
    state.update(Event::Dialog(DialogEvent::PaletteUp));
    let count = filter_commands(&state.registry, "").len();
    let stack = palette_stack(&state).expect("Palette should be open");
    assert_eq!(
        stack.current().unwrap().selected,
        count - 1,
        "Up at first should wrap to last"
    );
}

#[test]
fn select_wraps_down() {
    let mut state = AppState::default();
    state.update(Event::Dialog(DialogEvent::ToggleCommandPalette));
    let count = filter_commands(&state.registry, "").len();
    for _ in 0..count {
        state.update(Event::Dialog(DialogEvent::PaletteDown));
    }
    let stack = palette_stack(&state).expect("Palette should be open");
    assert_eq!(
        stack.current().unwrap().selected,
        0,
        "Down at last should wrap to first"
    );
}
