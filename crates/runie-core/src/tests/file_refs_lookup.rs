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
        matches!(
            state.open_dialog,
            Some(DialogState::Active {
                kind: DialogKind::Generic,
                panels: _
            })
        ),
        "@ should open a PanelStack file picker dialog"
    );
}

#[test]
fn at_ref_dialog_has_file_items() {
    let mut state = AppState::default();
    inject_mock_files(&mut state);
    state.update(crate::Event::Input('@'));
    let stack = match &state.open_dialog {
        Some(DialogState::Active {
            kind: DialogKind::Generic,
            panels: s,
        }) => s,
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
        Some(DialogState::Active {
            kind: DialogKind::Generic,
            panels: s,
        }) => s,
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
            Some(DialogState::Active {
                kind: DialogKind::Generic,
                panels: s,
            }) => s,
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

/// Create a mock file entry whose display name differs from its relative path
/// (duplicate basenames in different directories).
fn mock_file_entry_at(name: &str, path: &str) -> FffFileEntry {
    FffFileEntry {
        name: name.to_owned(),
        path: path.to_owned(),
        is_dir: false,
        score: 1.0,
        git_status: None,
    }
}

/// Inject entries with duplicate basenames, like a workspace full of Cargo.tomls.
fn inject_duplicate_basename_files(state: &mut AppState) {
    state.fff_file_results = vec![
        mock_file_entry_at("Cargo.toml", "Cargo.toml"),
        mock_file_entry_at("Cargo.toml", "crates/runie-core/Cargo.toml"),
        mock_file_entry_at("Cargo.toml", "crates/runie-tui/Cargo.toml"),
    ];
}

/// Collect the Action labels of the currently open picker panel.
fn picker_action_labels(state: &AppState) -> Vec<String> {
    match &state.open_dialog {
        Some(DialogState::Active {
            kind: DialogKind::Generic,
            panels,
        }) => panels
            .current()
            .expect("Picker panel should exist")
            .items
            .iter()
            .filter_map(|item| match item {
                crate::dialog::PanelItem::Action { label, .. } => Some(label.clone()),
                _ => None,
            })
            .collect(),
        _ => panic!("Expected file picker dialog"),
    }
}

/// Picker rows for duplicate basenames must carry path context so the rows
/// are distinguishable (a workspace can have many Cargo.toml files).
#[test]
fn at_ref_picker_disambiguates_duplicate_basenames() {
    let mut state = AppState::default();
    inject_duplicate_basename_files(&mut state);
    state.update(crate::Event::Input('@'));
    let labels = picker_action_labels(&state);
    assert!(
        labels
            .iter()
            .any(|l| l.contains("crates/runie-core/Cargo.toml")),
        "duplicate basenames should display their relative path, got: {labels:?}"
    );
    let mut unique = labels.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(
        unique.len(),
        labels.len(),
        "picker labels must be unique, got: {labels:?}"
    );
}

/// Selecting a duplicate-basename row inserts the disambiguated relative path,
/// not the bare basename.
#[test]
fn at_ref_select_inserts_relative_path() {
    let mut state = AppState::default();
    inject_duplicate_basename_files(&mut state);
    state.update(crate::Event::Input('@'));
    for c in "runie-core".chars() {
        state.update(crate::Event::Input(c));
    }
    state.update(crate::Event::Submit);
    assert_eq!(
        state.input.input, "@crates/runie-core/Cargo.toml ",
        "selection should insert the full relative path with '@' and trailing space"
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

/// Regression (production desync): when '@' opens the picker, the UiActor
/// clears the authoritative InputActor, whose `InputChanged` echo then
/// wholesale-replaces the projection `InputState`. The file-picker backup
/// and range suffix are projection-only — the echo must not wipe them, or
/// picking a file rewrites the whole input with the bare filename.
#[test]
fn input_changed_preserves_file_picker_backup() {
    let mut state = AppState::default();
    inject_mock_files(&mut state);
    // UiActor '@' trigger state: typed prefix + '@', backup saved, picker open.
    state.input.input = "read @".to_string();
    state.input.cursor_pos = 6;
    state.input.file_picker_backup = Some(("read @".to_string(), 6, 6, false));
    state.update(crate::Event::AtFilePicker);
    assert!(state.open_dialog.is_some());

    // The InputActor's Clear echo: an empty InputState with no backup.
    state.update(crate::Event::InputChanged {
        state: Box::new(crate::model::InputState::default()),
    });

    // The backup must survive the echo; picking keeps the typed prefix.
    assert_eq!(
        state.input.file_picker_backup,
        Some(("read @".to_string(), 6, 6, false)),
        "InputChanged must not wipe the file-picker backup"
    );
    state.update(crate::Event::Submit);
    assert!(
        state.input.input.starts_with("read @"),
        "pick must preserve the typed prefix, got: {:?}",
        state.input.input
    );
    assert!(state.open_dialog.is_none());
}

/// The range suffix (`@path:10-50`) is projection-only too: it must survive
/// the InputActor's InputChanged echo while the picker is open.
#[test]
fn input_changed_preserves_file_picker_range_suffix() {
    let mut state = AppState::default();
    state.input.file_picker_backup = Some(("read @".to_string(), 6, 6, false));
    state.input.file_picker_range_suffix = Some(":10-50".to_string());

    state.update(crate::Event::InputChanged {
        state: Box::new(crate::model::InputState::default()),
    });

    assert_eq!(
        state.input.file_picker_range_suffix,
        Some(":10-50".to_string()),
        "InputChanged must not wipe the range suffix"
    );
    assert!(state.input.file_picker_backup.is_some());
}

/// Esc maps to `DialogBack` (not `Abort`): closing the picker with Esc must
/// restore the typed prefix from the backup exactly like Abort does.
#[test]
fn dialog_back_restores_file_picker_backup() {
    let mut state = AppState::default();
    inject_mock_files(&mut state);
    // UiActor '@' trigger: backup holds the typed prefix; the InputActor's
    // Clear echo then empties the projection input (production ordering).
    state.input.file_picker_backup = Some(("read @".to_string(), 6, 6, false));
    state.update(crate::Event::AtFilePicker);
    assert!(state.open_dialog.is_some());
    state.update(crate::Event::InputChanged {
        state: Box::new(crate::model::InputState::default()),
    });
    assert!(state.input.input.is_empty());

    state.update(crate::Event::DialogBack);
    assert!(state.open_dialog.is_none(), "Esc should close the picker");
    assert_eq!(
        state.input.input, "read @",
        "Esc must restore the typed prefix from the picker backup"
    );
}

#[test]
fn no_at_ref_no_dialog() {
    let state = AppState::default();
    assert!(state.open_dialog.is_none());
}
