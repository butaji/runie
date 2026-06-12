use crate::commands::DialogState;
use crate::event::Event;
use crate::model::AppState;

#[test]
fn at_ref_opens_file_picker_dialog() {
    let mut state = AppState::default();
    state.update(Event::Input('@'));
    assert!(
        matches!(state.open_dialog, Some(DialogState::PanelStack(_))),
        "@ should open a PanelStack file picker dialog"
    );
}

#[test]
fn at_ref_dialog_has_file_items() {
    let mut state = AppState::default();
    state.update(Event::Input('@'));
    let stack = match &state.open_dialog {
        Some(DialogState::PanelStack(s)) => s,
        _ => panic!("Expected PanelStack dialog"),
    };
    let panel = stack.current().expect("PanelStack should have a panel");
    let nav_count = panel.navigable_count();
    assert!(
        nav_count > 0,
        "File picker should have at least one file item, got {}",
        nav_count
    );
}

#[test]
fn at_ref_dialog_is_filterable() {
    let mut state = AppState::default();
    state.update(Event::Input('@'));
    let stack = match &state.open_dialog {
        Some(DialogState::PanelStack(s)) => s,
        _ => panic!("Expected PanelStack dialog"),
    };
    let panel = stack.current().expect("PanelStack should have a panel");
    assert!(panel.filterable, "File picker panel should be filterable");
}

#[test]
fn at_ref_select_inserts_file_path() {
    let mut state = AppState::default();
    state.update(Event::Input('@'));
    // Find a file item and select it
    let path = {
        let stack = match &state.open_dialog {
            Some(DialogState::PanelStack(s)) => s,
            _ => panic!("Expected PanelStack dialog"),
        };
        let panel = stack.current().expect("Panel should exist");
        panel
            .items
            .iter()
            .find_map(|item| match item {
                crate::dialog::PanelItem::Action { label, .. } => Some(label.clone()),
                _ => None,
            })
            .expect("Should have at least one Action item")
    };
    state.update(Event::Submit);
    assert_eq!(
        state.input.input, path,
        "Should insert filepath after selection"
    );
    assert!(
        state.open_dialog.is_none(),
        "Dialog should close after selection"
    );
}

#[test]
fn at_ref_escape_closes_dialog() {
    let mut state = AppState::default();
    state.update(Event::Input('@'));
    assert!(state.open_dialog.is_some(), "Dialog should be open");
    state.update(Event::Abort);
    assert!(state.open_dialog.is_none(), "Escape should close dialog");
}

#[test]
fn no_at_ref_no_dialog() {
    let state = AppState::default();
    assert!(state.open_dialog.is_none());
}
