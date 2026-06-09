use crate::model::AppState;
use crate::event::Event;
use crate::commands::DialogState;

#[test]
fn slash_opens_command_palette_when_input_empty() {
    let mut state = AppState::default();
    assert!(state.open_dialog.is_none());
    assert!(state.input.input.is_empty());
    state.update(Event::Input('/'));
    assert!(matches!(state.open_dialog, Some(DialogState::CommandPalette { .. })), "Typing / with empty input should open command palette");
}

#[test]
fn slash_does_not_open_palette_when_input_not_empty() {
    let mut state = AppState::default();
    state.update(Event::Input('h'));
    state.update(Event::Input('i'));
    assert_eq!(state.input.input, "hi");
    state.update(Event::Input('/'));
    assert!(state.open_dialog.is_none(), "Typing / with non-empty input should NOT open palette");
    assert_eq!(state.input.input, "hi/", "Slash should be inserted normally");
}

#[test]
fn ctrl_p_opens_command_palette() {
    let mut state = AppState::default();
    assert!(state.open_dialog.is_none());
    state.update(Event::ToggleCommandPalette);
    assert!(matches!(state.open_dialog, Some(DialogState::CommandPalette { .. })), "Ctrl+P should open command palette");
}

#[test]
fn toggle_opens_palette() {
    let mut state = AppState::default();
    assert!(state.open_dialog.is_none());
    state.update(Event::ToggleCommandPalette);
    assert!(matches!(state.open_dialog, Some(DialogState::CommandPalette { .. })));
}

#[test]
fn select_closes_then_executes() {
    let mut state = AppState::default();
    state.update(Event::ToggleCommandPalette);
    // Select the first command
    state.update(Event::PaletteSelect);
    assert!(state.open_dialog.is_none(), "Palette should close after select");
}

#[test]
fn close_pops_dialog() {
    let mut state = AppState::default();
    state.update(Event::ToggleCommandPalette);
    assert!(state.open_dialog.is_some());
    state.update(Event::PaletteClose);
    assert!(state.open_dialog.is_none());
}

#[test]
fn esc_closes_palette() {
    let mut state = AppState::default();
    state.update(Event::ToggleCommandPalette);
    assert!(state.open_dialog.is_some());
    state.update(Event::Abort);
    assert!(state.open_dialog.is_none());
}

#[test]
fn filter_reduces_selection() {
    let mut state = AppState::default();
    state.update(Event::ToggleCommandPalette);
    // Type "q" to filter to quit
    state.update(Event::PaletteFilter('q'));
    if let Some(DialogState::CommandPalette { filter, selected }) = &state.open_dialog {
        assert_eq!(filter, "q");
        assert_eq!(*selected, 0, "Filter resets selection to 0");
    } else {
        panic!("Palette should be open");
    }
}
