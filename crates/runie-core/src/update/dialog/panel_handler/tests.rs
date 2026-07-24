#![allow(clippy::useless_conversion)]
use super::*;
use crate::commands::DialogState;
use crate::dialog::{ItemAction, Panel, PanelItem};

#[test]
fn space_toggles_checkbox_item_value() {
    let mut state = AppState::default();
    state.config.read_only = false;
    let mut panel = Panel::new("test", "Test").toggle("Read-Only", false, ItemAction::Toggle("read_only".into()));

    assert!(toggle_selected_checkbox(&mut state, &mut panel));
    assert!(
        matches!(
            panel.selected_item(),
            Some(PanelItem::Toggle { value: true, .. })
        ),
        "checkbox value should flip to true"
    );
    assert!(
        state.config.read_only,
        "read_only setting should be applied"
    );
}

#[test]
fn space_on_non_toggle_does_nothing() {
    let mut state = AppState::default();
    let mut panel = Panel::new("test", "Test").item("Do", ItemAction::Close);
    assert!(!toggle_selected_checkbox(&mut state, &mut panel));
}

#[test]
fn space_on_emit_checkbox_updates_state() {
    let mut state = AppState::default();
    let mut flow = crate::login_flow::LoginFlowState::new()
        .with_provider("minimax".into())
        .with_key("sk".into())
        .with_validation_success(vec!["m1".into()]);
    flow.selected_models.clear();
    state.login_flow = Some(flow);

    let mut panel = Panel::new("models", "Models").toggle(
        "m1",
        false,
        ItemAction::Emit(crate::Event::from(crate::Event::ToggleModel {
            model: "m1".into(),
        })),
    );

    assert!(toggle_selected_checkbox(&mut state, &mut panel));
    let flow = state.login_flow.as_ref().expect("login flow");
    assert!(flow.selected_models.contains("m1"));
}

#[test]
fn space_in_list_panel_keeps_dialog_open() {
    let mut state = AppState::default();
    let panel = Panel::new("settings", "Settings").toggle("Read-Only", false, ItemAction::Toggle("read_only".into()));
    let mut stack = PanelStack::new(panel);
    state.open_dialog = Some(DialogState::Active { kind: DialogKind::Generic, panels: stack.clone() });

    let result = update_panel_stack(&mut state, crate::Event::Input(' ').into(), &mut stack);
    assert_eq!(
        result,
        PanelUpdateResult::Consumed,
        "space should be consumed by the toggle"
    );
    assert!(
        matches!(
            state.open_dialog,
            Some(DialogState::Active { kind: DialogKind::Generic, panels: _ })
        ),
        "dialog should stay open after toggling"
    );
    assert!(state.config.read_only);
}

/// Regression (live-test #6): pressing Enter on a settings/model toggle row must
/// toggle the checkbox and KEEP the dialog open (same as Space). It previously
/// toggled and then closed the dialog, so Enter looked like it just dismissed it.
#[test]
fn enter_on_toggle_row_toggles_and_keeps_dialog_open() {
    let mut state = AppState::default();
    state.config.read_only = false;
    let panel = Panel::new("settings", "Settings").toggle("Read-Only", false, ItemAction::Toggle("read_only".into()));
    let mut stack = PanelStack::new(panel);
    state.open_dialog = Some(DialogState::Active { kind: DialogKind::Generic, panels: stack.clone() });

    let result = update_panel_stack(&mut state, crate::Event::Submit.into(), &mut stack);

    assert_eq!(
        result,
        PanelUpdateResult::Consumed,
        "Enter on a toggle row should toggle (consumed), not close the dialog"
    );
    assert!(
        matches!(
            state.open_dialog,
            Some(DialogState::Active { kind: DialogKind::Generic, .. })
        ),
        "dialog should stay open after Enter toggles a row"
    );
    assert!(
        state.config.read_only,
        "the setting should have been toggled by Enter"
    );
}

/// Regression (live-test ISSUE B): pressing Enter on a settings *cycle/select* row
/// (e.g. Thinking Level, Theme, Steering/Follow-Up Mode) must advance the option,
/// apply the setting, and KEEP the dialog open — same as a toggle row. It
/// previously cycled and then closed the dialog.
#[test]
fn enter_on_cycle_row_cycles_and_keeps_dialog_open() {
    use crate::model::ThinkingLevel;

    let mut state = AppState::default();
    let mut panel = Panel::new("settings", "Settings");
    panel.items.push(PanelItem::Select {
        label: "Thinking Level".into(),
        current: "Off".into(),
        options: vec!["Off".into(), "Low".into(), "Medium".into(), "High".into()],
        key: "thinking_level".into(),
    });
    let mut stack = PanelStack::new(panel);
    state.open_dialog = Some(DialogState::Active { kind: DialogKind::Generic, panels: stack.clone() });

    let result = update_panel_stack(&mut state, crate::Event::Submit.into(), &mut stack);

    assert_eq!(
        result,
        PanelUpdateResult::Consumed,
        "Enter on a cycle/select row should cycle (consumed), not close the dialog"
    );
    assert!(
        matches!(
            state.open_dialog,
            Some(DialogState::Active { kind: DialogKind::Generic, .. })
        ),
        "dialog should stay open after Enter cycles a settings row"
    );
    assert_eq!(
        state.config.thinking_level,
        ThinkingLevel::Low,
        "the setting should have advanced Off -> Low and been applied"
    );
}

#[test]
fn palette_args_only_from_exact_command_prefix() {
    // Exact command name typed in full -> no args.
    assert_eq!(activation::extract_palette_args("theme", "theme"), "");
    // Full command line typed into the palette -> args after the space.
    assert_eq!(
        activation::extract_palette_args("theme", "theme nord"),
        "nord"
    );
    assert_eq!(
        activation::extract_palette_args("theme", "theme  nord  "),
        "nord"
    );
    // Fuzzy search fragments are queries, never arguments — even when they
    // merely prefix-match the command name.
    assert_eq!(activation::extract_palette_args("theme", "the"), "");
    assert_eq!(activation::extract_palette_args("theme", "themes"), "");
    assert_eq!(activation::extract_palette_args("theme", "the nord"), "");
    assert_eq!(activation::extract_palette_args("theme", ""), "");
}

/// Regression: pressing Esc (DialogBack) on the @-file picker must close the dialog
/// AND restore the typed prefix from the file_picker_backup.
#[test]
fn dialog_back_closes_file_picker_and_restores_prefix() {
    use crate::dialog::builders::file_picker;
    use crate::update::dialog::update_dialog;

    let mut state = AppState::default();
    // Simulate typing "read @" before opening the picker
    state.input.input = "read @".to_string();
    state.input.cursor_pos = 6;

    // Open the file picker — this saves the backup
    let entries = vec![
        ("foo.rs".into(), false, crate::Event::InsertAtRef("foo.rs".into())),
        ("bar.rs".into(), false, crate::Event::InsertAtRef("bar.rs".into())),
    ];
    let stack = file_picker(entries);
    *state.open_dialog_mut() = Some(DialogState::Active { kind: DialogKind::Generic, panels: stack });

    assert!(
        state.open_dialog().is_some(),
        "file picker should be open"
    );
    assert!(
        state.input.file_picker_backup.is_some(),
        "picker backup should be set"
    );

    // Press Esc (DialogBack) — should close the picker
    update_dialog(&mut state, crate::Event::DialogBack);

    assert!(
        state.open_dialog().is_none(),
        "file picker should be closed after DialogBack"
    );
    // The typed prefix should be restored
    assert_eq!(
        state.input.input, "read @",
        "input should be restored to the typed prefix"
    );
}

/// DialogBack on a file picker selected on the header (non-navigable) should
/// still close the dialog (not return Consumed and keep it open).
#[test]
fn dialog_back_closes_even_without_selection() {
    use crate::dialog::builders::file_picker;
    use crate::update::dialog::update_dialog;

    let mut state = AppState::default();
    state.input.input = "read @".to_string();

    // File picker with zero entries — the header is the only item
    let stack = file_picker(vec![]);
    *state.open_dialog_mut() = Some(DialogState::Active { kind: DialogKind::Generic, panels: stack });

    assert!(state.open_dialog().is_some());
    update_dialog(&mut state, crate::Event::DialogBack);
    assert!(
        state.open_dialog().is_none(),
        "file picker should close on DialogBack even with no items"
    );
    assert_eq!(state.input.input, "read @");
}

/// Enter on a file picker item should close the dialog and insert the path.
#[test]
fn enter_on_file_picker_inserts_path() {
    use crate::dialog::builders::file_picker;
    use crate::update::dialog::update_dialog;

    let mut state = AppState::default();
    state.input.input = "read @".to_string();
    state.input.cursor_pos = 6;

    let entries = vec![("foo.rs".into(), false, crate::Event::InsertAtRef("foo.rs".into()))];
    let stack = file_picker(entries);
    *state.open_dialog_mut() = Some(DialogState::Active { kind: DialogKind::Generic, panels: stack });

    assert!(state.open_dialog().is_some());

    // Enter — should close the picker and insert the path
    update_dialog(&mut state, crate::Event::Submit);

    assert!(
        state.open_dialog().is_none(),
        "file picker should close on Enter"
    );
    assert!(
        state.input.input.contains("foo.rs"),
        "inserted path should appear in input"
    );
}
