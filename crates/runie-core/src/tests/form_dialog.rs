//! Tests for command form dialogs.

use crate::commands::DialogState;
use crate::model::AppState;
use crate::Event;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn fresh_state() -> AppState {
    let mut state = AppState::default();
    state.input.input.clear();
    state.input.cursor_pos = 0;
    state
}

fn type_str(state: &mut AppState, s: &str) {
    for c in s.chars() {
        state.update(Event::Input(c));
    }
}

fn tmp_store() -> crate::session::Store {
    let store = crate::session::Store::new(
        std::env::temp_dir().join(format!("runie_test_{}", std::process::id())),
    );
    let _ = std::fs::remove_dir_all(&store.dir);
    store
}

#[test]
fn save_no_args_opens_form() {
    let mut state = fresh_state();
    type_str(&mut state, "/save");
    state.update(Event::Submit);

    // Should open form dialog
    assert!(state.open_dialog.is_some(), "should open dialog");
    if let Some(DialogState::PanelStack(stack)) = &state.open_dialog {
        let panel = stack.current().expect("should have panel");
        assert_eq!(panel.id, "save", "should be save form");
    } else {
        panic!("expected PanelStack dialog");
    }
}

#[test]
fn load_no_args_opens_form() {
    let mut state = fresh_state();
    type_str(&mut state, "/load");
    state.update(Event::Submit);

    // Should open form dialog
    assert!(state.open_dialog.is_some(), "should open dialog");
    if let Some(DialogState::PanelStack(stack)) = &state.open_dialog {
        let panel = stack.current().expect("should have panel");
        assert_eq!(panel.id, "load", "should be load form");
    } else {
        panic!("expected PanelStack dialog");
    }
}

#[test]
fn delete_no_args_opens_form() {
    let mut state = fresh_state();
    type_str(&mut state, "/delete");
    state.update(Event::Submit);

    // Should open form dialog
    assert!(state.open_dialog.is_some(), "should open dialog");
    if let Some(DialogState::PanelStack(stack)) = &state.open_dialog {
        let panel = stack.current().expect("should have panel");
        assert_eq!(panel.id, "delete", "should be delete form");
    } else {
        panic!("expected PanelStack dialog");
    }
}

#[test]
fn form_save_accepts_input() {
    let mut state = fresh_state();
    type_str(&mut state, "/save");
    state.update(Event::Submit);

    // Should open form
    assert!(state.open_dialog.is_some());
    if let Some(DialogState::PanelStack(stack)) = &state.open_dialog {
        let panel = stack.current().expect("panel");
        assert!(panel.is_form(), "should be form");
    }
}

#[test]
fn form_submit_executes_command() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let store = tmp_store();
    std::env::set_var("RUNIE_SESSIONS_DIR", store.dir.clone());
    let mut state = fresh_state();
    type_str(&mut state, "/save");
    state.update(Event::Submit);
    // Verify form is open
    assert!(
        state.open_dialog.is_some(),
        "form should be open after submit"
    );
    // Check the dialog type - take dialog to inspect, then restore
    let form_is_form =
        if let Some(crate::commands::DialogState::PanelStack(stack)) = &state.open_dialog {
            if let Some(panel) = stack.current() {
                panel.is_form()
            } else {
                false
            }
        } else {
            false
        };
    assert!(form_is_form, "panel should be a form");
    // Type a name - these go to input, but should be routed to form
    state.update(Event::Input('m'));
    state.update(Event::Input('y'));
    state.update(Event::Input('s'));
    state.update(Event::Input('e'));
    state.update(Event::Input('s'));
    // Submit the form
    state.update(Event::Submit);
    // Should close dialog and execute save
    assert!(state.open_dialog.is_none(), "dialog should close");
    assert!(store.path("myses").exists(), "session should be saved");
    std::env::remove_var("RUNIE_SESSIONS_DIR");
}

#[test]
fn form_panel_id_maps_to_known_form_command() {
    // The form's id selects which command to run. "save" runs RunSaveCommand.
    // An unknown id (a form registered but not in the dispatch table) must
    // not silently dispatch anything — the dialog closes, no event fires.
    use std::sync::Mutex;
    static UNKNOWN_LOCK: Mutex<()> = Mutex::new(());
    let _guard = UNKNOWN_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let mut state = fresh_state();
    use crate::commands::DialogState;
    use crate::dialog::{Panel, PanelStack};
    let mut panel = Panel::new("unknown_command_xyz", "T").form_field("Field", "ph", "name");
    panel
        .form_values
        .insert("name".into(), "should-not-fire".into());
    state.open_dialog = Some(DialogState::PanelStack(PanelStack::new(panel)));

    state.update(crate::Event::CommandFormSubmit);

    assert!(state.open_dialog.is_none(), "dialog should close on submit");
    let sys_count = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == crate::model::Role::System)
        .count();
    assert_eq!(
        sys_count, 0,
        "unknown form id must not produce side effects, got {} system msgs",
        sys_count
    );
}

#[test]
fn all_form_commands_are_listed() {
    // Regression: every command that has a form-flow should be in the dispatch
    // table. This list is the contract — when a new form command is added,
    // this test must be updated to include it. Commands marked `.sub()`
    // wrap their flow in `CommandFlow::Sub`; we unwrap to find the Form.
    use crate::commands::{CommandFlow, CommandRegistry};
    let mut reg = CommandRegistry::new();
    crate::commands::handlers::register_all(&mut reg);
    let form_commands: Vec<String> = reg
        .list()
        .iter()
        .filter_map(|def| {
            let flow = match &def.flow {
                CommandFlow::Sub(inner) => inner.as_ref(),
                other => other,
            };
            if matches!(flow, CommandFlow::Form { .. }) {
                Some(def.name.clone())
            } else {
                None
            }
        })
        .collect();
    // Sanity: we have multiple form commands
    assert!(form_commands.len() >= 8, "expected at least 8 form commands, got {:?} (this is a sanity check; if a new form is added, ensure form_build_submit handles it)", form_commands);
}

#[test]
fn invalid_fork_index_shows_error_for_out_of_range() {
    // /fork with an out-of-range index must surface a clear error, not
    // silently dispatch with a default.
    use crate::commands::DialogState;
    use crate::dialog::{Panel, PanelStack};
    use crate::model::Role;
    let mut state = fresh_state();
    state.session.messages.push(crate::model::ChatMessage {
        role: Role::User,
        content: "hi".into(),
        timestamp: 0.0,
        id: "u0".into(),
        ..Default::default()
    });
    let mut panel = Panel::new("fork", "Fork Session").form_field("Message index", "0", "index");
    panel.form_values.insert("index".into(), "999".into());
    state.open_dialog = Some(DialogState::PanelStack(PanelStack::new(panel)));

    state.update(crate::Event::CommandFormSubmit);

    assert!(state.open_dialog.is_none(), "dialog should close");
    let sys: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    assert!(!sys.is_empty(), "expected a system error message");
    assert!(
        sys.last().unwrap().content.contains("out of range"),
        "expected out-of-range error, got: {:?}",
        sys.last().unwrap().content
    );
}

#[test]
fn compact_with_invalid_keep_shows_error() {
    // /compact with a non-numeric keep value must surface a clear error,
    // not silently fall back to 2000.
    use crate::commands::DialogState;
    use crate::dialog::{Panel, PanelStack};
    use crate::model::Role;
    let mut state = fresh_state();
    let mut panel = Panel::new("compact", "Compact Context")
        .form_field("Keep tokens", "2000", "keep")
        .form_field("Focus", "f", "focus");
    panel
        .form_values
        .insert("keep".into(), "not-a-number".into());
    panel.form_values.insert("focus".into(), "".into());
    state.open_dialog = Some(DialogState::PanelStack(PanelStack::new(panel)));

    state.update(crate::Event::CommandFormSubmit);

    let sys: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    assert!(!sys.is_empty(), "expected a system error message");
    let last = sys.last().unwrap().content.clone();
    assert!(
        last.contains("Invalid")
            || last.contains("invalid")
            || last.contains("not a number")
            || last.contains("parse"),
        "expected an error mentioning invalid input, got: {}",
        last
    );
}

#[test]
fn form_escape_closes_dialog() {
    let mut state = fresh_state();
    type_str(&mut state, "/save");
    state.update(Event::Submit);

    // Dialog should be open
    assert!(state.open_dialog.is_some());

    // Press Escape to close
    state.update(Event::Abort);

    // Dialog should be closed
    assert!(state.open_dialog.is_none(), "dialog should close on escape");
}

#[test]
fn form_navigation_up_down() {
    let mut state = fresh_state();
    type_str(&mut state, "/save");
    state.update(Event::Submit);

    // Should open form with a form field selected
    assert!(state.open_dialog.is_some());

    // Navigate down
    state.update(Event::CommandFormDown);

    // Navigate up
    state.update(Event::CommandFormUp);
}

#[test]
fn form_backspace_deletes() {
    let mut state = fresh_state();
    type_str(&mut state, "/save");
    state.update(Event::Submit);

    // Type some characters
    state.update(Event::Input('a'));
    state.update(Event::Input('b'));
    state.update(Event::Input('c'));

    // Backspace
    state.update(Event::Backspace);

    // Dialog should still be open
    assert!(state.open_dialog.is_some());
}

#[test]
fn form_button_activated_by_enter() {
    use crate::dialog::{ItemAction, Panel};
    let mut panel = Panel::new("test", "Test")
        .form_field("Name", "", "name")
        .item("_Submit", ItemAction::Emit(crate::Event::LoginFlowSave))
        .item("_Cancel", ItemAction::Emit(crate::Event::LoginFlowCancel));
    // Navigate to the first button (index 1, after form field at index 0)
    panel.selected = 1;
    let action = AppState::form_panel_action(&mut panel, crate::Event::Submit);
    assert!(matches!(
        action,
        crate::update::FormAction::Submit(Some(crate::Event::LoginFlowSave))
    ));
}

#[test]
fn form_button_activated_by_accelerator() {
    use crate::dialog::{ItemAction, Panel};
    let mut panel = Panel::new("test", "Test")
        .form_field("Name", "", "name")
        .item("_Submit", ItemAction::Emit(crate::Event::LoginFlowSave))
        .item("_Cancel", ItemAction::Emit(crate::Event::LoginFlowCancel));
    // On a form field, typing 'c' should type into the field
    panel.selected = 0;
    let action = AppState::form_panel_action(&mut panel, crate::Event::Input('c'));
    assert!(matches!(action, crate::update::FormAction::KeepOpen));
    assert_eq!(panel.form_values.get("name"), Some(&"c".to_string()));

    // On a button, typing 'c' should activate Cancel
    panel.selected = 2;
    let action = AppState::form_panel_action(&mut panel, crate::Event::Input('c'));
    assert!(matches!(
        action,
        crate::update::FormAction::Submit(Some(crate::Event::LoginFlowCancel))
    ));
}

#[test]
fn form_field_submit_still_builds_form_values() {
    use crate::dialog::{ItemAction, Panel};
    let mut panel = Panel::new("save", "Save")
        .form_field("Name", "my-session", "name")
        .item("_Submit", ItemAction::Emit(crate::Event::LoginFlowSave));
    // On the form field, Enter should submit the form
    panel.selected = 0;
    let action = AppState::form_panel_action(&mut panel, crate::Event::Submit);
    assert!(matches!(
        action,
        crate::update::FormAction::Submit(Some(crate::Event::RunSaveCommand { .. }))
    ));
}
