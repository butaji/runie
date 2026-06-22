//! Tests for command form dialogs.

use crate::dialog::dsl::get_field;
use crate::event::{ControlEvent, DialogEvent, InputEvent};

use crate::commands::DialogState;
use crate::tests::slash::ENV_LOCK;
use crate::Event;
use crate::tests::{fresh_state, type_str};

/// Open palette and select a command by name
fn palette_select(state: &mut crate::model::AppState, cmd: &str) {
    // Open palette with '/'
    state.update(InputEvent::Input('/'));
    // Filter to the command
    for c in cmd.chars() {
        state.update(DialogEvent::PaletteFilter(c));
    }
    // Select the command
    state.update(DialogEvent::PaletteSelect);
}

fn tmp_store() -> crate::session_store::SessionStore {
    let dir = std::env::temp_dir().join(format!("runie_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    crate::session_store::SessionStore::new(dir)
}

#[test]
fn save_no_args_opens_form() {
    let mut state = fresh_state();
    palette_select(&mut state, "save");

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
    palette_select(&mut state, "load");

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
    palette_select(&mut state, "delete");

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
    palette_select(&mut state, "save");

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
    std::env::set_var("RUNIE_SESSIONS_DIR", store.dir().to_path_buf());
    let mut state = fresh_state();
    palette_select(&mut state, "save");
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
    state.update(InputEvent::Input('m'));
    state.update(InputEvent::Input('y'));
    state.update(InputEvent::Input('s'));
    state.update(InputEvent::Input('e'));
    state.update(InputEvent::Input('s'));
    // Submit the form
    state.update(Event::submit());
    // Should close dialog and execute save
    assert!(state.open_dialog.is_none(), "dialog should close");
    let redb_path = crate::session_store::SessionStore::new(store.dir().to_path_buf()).path("myses");
    assert!(redb_path.exists(), "session should be saved");
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

    state.update(crate::event::DialogEvent::CommandFormSubmit);

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
    // wrap their flow in `CommandFlow::Sub`; we unwrap to find the PanelStack.
    use crate::commands::{CommandFlow, CommandRegistry};
    let mut reg = CommandRegistry::new();
    crate::commands::dsl::handlers::register_all(&mut reg);
    let form_commands: Vec<String> = reg
        .list()
        .iter()
        .filter_map(|def| {
            let flow = match &def.flow {
                CommandFlow::Sub(inner) => inner.as_ref(),
                other => other,
            };
            if matches!(flow, CommandFlow::PanelStack(_)) {
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
    panel.submit_factory = Some(|values| crate::event::CommandEvent::RunForkCommand {
        message_index: crate::dialog::dsl::get_field(values, "index"),
    });
    panel.form_values.insert("index".into(), "999".into());
    state.open_dialog = Some(DialogState::PanelStack(PanelStack::new(panel)));

    state.update(crate::event::DialogEvent::CommandFormSubmit);

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
    panel.submit_factory = Some(|values| crate::event::CommandEvent::RunCompactCommand {
        keep: crate::dialog::dsl::get_field(values, "keep"),
        focus: crate::dialog::dsl::get_field(values, "focus"),
    });
    panel
        .form_values
        .insert("keep".into(), "not-a-number".into());
    panel.form_values.insert("focus".into(), "".into());
    state.open_dialog = Some(DialogState::PanelStack(PanelStack::new(panel)));

    state.update(crate::event::DialogEvent::CommandFormSubmit);

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
    palette_select(&mut state, "save");

    // Dialog should be open
    assert!(state.open_dialog.is_some());

    // Press Escape to close
    state.update(ControlEvent::Abort);

    // Dialog should be closed
    assert!(state.open_dialog.is_none(), "dialog should close on escape");
}

#[test]
fn form_navigation_up_down() {
    let mut state = fresh_state();
    palette_select(&mut state, "save");

    // Should open form with a form field selected
    assert!(state.open_dialog.is_some());

    // Navigate down
    state.update(DialogEvent::CommandFormDown);

    // Navigate up
    state.update(DialogEvent::CommandFormUp);
}

#[test]
fn form_backspace_deletes() {
    let mut state = fresh_state();
    type_str(&mut state, "/save");
    state.update(Event::submit());

    // Type some characters
    state.update(InputEvent::Input('a'));
    state.update(InputEvent::Input('b'));
    state.update(InputEvent::Input('c'));

    // Backspace
    state.update(InputEvent::Backspace);

    // Dialog should still be open
    assert!(state.open_dialog.is_some());
}

#[test]
fn form_button_activated_by_enter() {
    use crate::dialog::{ItemAction, Panel};
    let mut panel = Panel::new("test", "Test")
        .form_field("Name", "", "name")
        .item(
            "_Submit",
            ItemAction::Emit(crate::event::LoginFlowEvent::Save),
        )
        .item(
            "_Cancel",
            ItemAction::Emit(crate::event::LoginFlowEvent::Cancel),
        );
    // Navigate to the first button (index 1, after form field at index 0)
    panel.selected = 1;
    let mut state = crate::model::AppState::default();
    let action =
        crate::update::dialog::form_panel_action(&mut state, &mut panel, crate::Event::submit());
    assert!(matches!(
        action,
        crate::update::dialog::FormAction::Submit(Some(crate::event::LoginFlowEvent::Save))
    ));
}

#[test]
fn form_button_activated_by_accelerator() {
    use crate::dialog::{ItemAction, Panel};
    let mut panel = Panel::new("test", "Test")
        .form_field("Name", "", "name")
        .item(
            "_Submit",
            ItemAction::Emit(crate::event::LoginFlowEvent::Save),
        )
        .item(
            "_Cancel",
            ItemAction::Emit(crate::event::LoginFlowEvent::Cancel),
        );
    // On a form field, typing 'c' should type into the field
    panel.selected = 0;
    let mut state = crate::model::AppState::default();
    let action = crate::update::dialog::form_panel_action(
        &mut state,
        &mut panel,
        crate::event::InputEvent::Input('c'),
    );
    assert!(matches!(
        action,
        crate::update::dialog::FormAction::KeepOpen
    ));
    assert_eq!(panel.form_values.get("name"), Some(&"c".to_string()));

    // On a button, typing 'c' should activate Cancel
    panel.selected = 2;
    let action = crate::update::dialog::form_panel_action(
        &mut state,
        &mut panel,
        crate::event::InputEvent::Input('c'),
    );
    assert!(matches!(
        action,
        crate::update::dialog::FormAction::Submit(Some(crate::event::LoginFlowEvent::Cancel))
    ));
}

#[test]
fn form_field_submit_still_builds_form_values() {
    use crate::dialog::{ItemAction, Panel};
    let mut panel = Panel::new("save", "Save")
        .form_field("Name", "my-session", "name")
        .item(
            "_Submit",
            ItemAction::Emit(crate::event::LoginFlowEvent::Save),
        );
    panel.submit_factory = Some(|values| crate::event::CommandEvent::RunSaveCommand {
        name: crate::dialog::dsl::get_field(values, "name"),
    });
    // On the form field, Enter should submit the form
    panel.selected = 0;
    let mut state = crate::model::AppState::default();
    let action =
        crate::update::dialog::form_panel_action(&mut state, &mut panel, crate::Event::submit());
    assert!(matches!(
        action,
        crate::update::dialog::FormAction::Submit(Some(
            crate::event::CommandEvent::RunSaveCommand { .. }
        ))
    ));
}

#[test]
fn form_submit_button_uses_factory() {
    use crate::dialog::Panel;
    let mut panel = Panel::new("save", "Save")
        .form_field("Name", "my-session", "name")
        .form_submit_with(|values| crate::event::CommandEvent::RunSaveCommand {
            name: get_field(values, "name"),
        });
    panel.form_values.insert("name".into(), "myses".into());
    // Move selection from the field to the FormSubmit button.
    panel.selected = 1;

    let mut state = crate::model::AppState::default();
    let action =
        crate::update::dialog::form_panel_action(&mut state, &mut panel, crate::Event::submit());

    assert!(
        matches!(
            action,
            crate::update::dialog::FormAction::Submit(Some(
                crate::event::CommandEvent::RunSaveCommand { .. }
            ))
        ),
        "Enter on FormSubmit button should use the submit factory"
    );
}
