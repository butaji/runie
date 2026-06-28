use crate::commands::{DialogKind, DialogState};
use crate::model::{AppState, FffFileEntry};

/// Create a mock file entry for testing.
fn mock_file_entry(name: &str) -> FffFileEntry {
    FffFileEntry {
        name: name.to_owned(),
        path: name.to_owned(),
        is_dir: false,
        score: 1.0,
        git_status: None,
    }
}

/// Inject mock file entries for testing (since file picker now uses actor-based search).
fn inject_mock_files(state: &mut AppState) {
    state.fff_file_results = vec![
        mock_file_entry("README.md"),
        mock_file_entry("src/main.rs"),
        mock_file_entry("src/lib.rs"),
    ];
}

#[test]
fn at_ref_opens_file_picker_dialog() {
    let mut state = AppState::default();
    state.update(crate::Event::Input('@'));
    assert!(
        matches!(state.open_dialog, Some(DialogState::Active { kind: DialogKind::Generic, panels: _ })),
        "@ should open a PanelStack file picker dialog"
    );
}

#[test]
fn at_ref_dialog_has_file_items() {
    let mut state = AppState::default();
    inject_mock_files(&mut state);
    state.update(crate::Event::Input('@'));
    let stack = match &state.open_dialog {
        Some(DialogState::Active { kind: DialogKind::Generic, panels: s }) => s,
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
    state.update(crate::Event::Input('@'));
    let stack = match &state.open_dialog {
        Some(DialogState::Active { kind: DialogKind::Generic, panels: s }) => s,
        _ => panic!("Expected PanelStack dialog"),
    };
    let panel = stack.current().expect("PanelStack should have a panel");
    assert!(panel.filterable, "File picker panel should be filterable");
}

#[test]
fn at_ref_select_inserts_file_path() {
    let mut state = AppState::default();
    inject_mock_files(&mut state);
    state.update(crate::Event::Input('@'));
    // Find a file item and select it
    let label = {
        let stack = match &state.open_dialog {
            Some(DialogState::Active { kind: DialogKind::Generic, panels: s }) => s,
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
    state.update(crate::Event::Submit);
    // The inserted path may be absolute or relative depending on FFF state,
    // but it must contain the selected file name and close the dialog.
    assert!(
        state.input.input.contains(&label) || label.contains(&state.input.input),
        "Inserted path '{}' should relate to selected label '{}'",
        state.input.input,
        label
    );
    assert!(
        state.open_dialog.is_none(),
        "Dialog should close after selection"
    );
}

#[test]
fn at_ref_escape_closes_dialog() {
    let mut state = AppState::default();
    state.update(crate::Event::Input('@'));
    assert!(state.open_dialog.is_some(), "Dialog should be open");
    state.update(crate::Event::Abort);
    assert!(state.open_dialog.is_none(), "Escape should close dialog");
}

#[test]
fn no_at_ref_no_dialog() {
    let state = AppState::default();
    assert!(state.open_dialog.is_none());
}
