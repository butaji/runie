#![allow(clippy::all)]
//! Tests for command form dialogs.

use crate::Event;

use crate::commands::{DialogKind, DialogState};
use crate::message::Part;
use crate::tests::{fresh_state, tmp_store, type_str};
use runie_testing::with_env;

/// Open palette and select a command by name
fn palette_select(state: &mut crate::model::AppState, cmd: &str) {
    // Open palette with '/'
    state.update(crate::Event::Input('/'));
    // Filter to the command
    for c in cmd.chars() {
        state.update(crate::Event::PaletteFilter(c));
    }
    // Select the command
    state.update(crate::Event::PaletteSelect);
}

#[test]
fn save_no_args_opens_form() {
    let mut state = fresh_state();
    palette_select(&mut state, "save");

    // Should open form dialog
    assert!(state.open_dialog.is_some(), "should open dialog");
    if let Some(DialogState::Active {
        kind: DialogKind::Generic,
        panels: stack,
    }) = &state.open_dialog
    {
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
    if let Some(DialogState::Active {
        kind: DialogKind::Generic,
        panels: stack,
    }) = &state.open_dialog
    {
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
    if let Some(DialogState::Active {
        kind: DialogKind::Generic,
        panels: stack,
    }) = &state.open_dialog
    {
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
    if let Some(DialogState::Active {
        kind: DialogKind::Generic,
        panels: stack,
    }) = &state.open_dialog
    {
        let panel = stack.current().expect("panel");
        assert!(panel.is_form(), "should be form");
    }
}

#[test]
fn form_submit_executes_command() {
    with_env(|env| {
        let store = tmp_store();
        env.set("RUNIE_SESSIONS_DIR", store.dir().to_path_buf().to_str().unwrap_or("/tmp"));
        let mut state = fresh_state();
        palette_select(&mut state, "save");
        // Verify form is open
        assert!(
            state.open_dialog.is_some(),
            "form should be open after submit"
        );
        // Check the dialog type - take dialog to inspect, then restore
        let form_is_form = if let Some(crate::commands::DialogState::Active {
            kind: DialogKind::Generic,
            panels: stack,
        }) = &state.open_dialog
        {
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
        state.update(crate::Event::Input('m'));
        state.update(crate::Event::Input('y'));
        state.update(crate::Event::Input('s'));
        state.update(crate::Event::Input('e'));
        state.update(crate::Event::Input('s'));
        // Submit the form
        state.update(Event::submit());
        // Should close dialog and execute save
        assert!(state.open_dialog.is_none(), "dialog should close");
        let jsonl_path =
            crate::session::store::SessionStore::new(store.dir().to_path_buf()).path("myses");
        assert!(jsonl_path.exists(), "session should be saved");
    });
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
    use crate::commands::{DialogKind, DialogState};
    use crate::dialog::{Panel, PanelStack};
    let mut panel = Panel::new("unknown_command_xyz", "T").form_field("Field", "ph", "name");
    panel
        .form_values
        .insert("name".into(), "should-not-fire".into());
    state.open_dialog = Some(DialogState::Active {
        kind: DialogKind::Generic,
        panels: PanelStack::new(panel),
    });

    state.update(Event::CommandFormSubmit);

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
    let reg = CommandRegistry::new();
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
    assert!(form_commands.len() >= 8, "expected at least 8 form commands, got {:?} (this is a sanity check; if a new form is added, ensure it uses FormWithHandler in the registry)", form_commands);
}

#[test]
fn invalid_fork_index_shows_error_for_out_of_range() {
    // /fork with an out-of-range index must surface a clear error, not
    // silently dispatch with a default. Form routes through command registry.
    use crate::commands::{DialogKind, DialogState};
    use crate::dialog::{Panel, PanelStack};
    use crate::model::Role;
    let mut state = fresh_state();
    state.session.messages.push(crate::model::ChatMessage {
        role: Role::User,
        timestamp: 0.0,
        id: "u0".into(),
        parts: vec![Part::Text {
            content: "hi".into(),
        }],
        ..Default::default()
    });
    let mut panel = Panel::new("fork", "Fork Session").form_field("Message index", "0", "index");
    panel.cmd_name = Some("fork".into());
    panel.field_keys.push("index".into());
    panel.form_values.insert("index".into(), "999".into());
    state.open_dialog = Some(DialogState::Active {
        kind: DialogKind::Generic,
        panels: PanelStack::new(panel),
    });

    state.update(Event::CommandFormSubmit);

    assert!(state.open_dialog.is_none(), "dialog should close");
    let sys: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    assert!(!sys.is_empty(), "expected a system error message");
    assert!(
        sys.last().unwrap().content().contains("out of range"),
        "expected out-of-range error, got: {:?}",
        sys.last().unwrap().content()
    );
}

#[test]
fn compact_with_invalid_keep_shows_error() {
    // /compact with a non-numeric keep value must surface a clear error,
    // not silently fall back to 2000. Form routes through command registry.
    use crate::commands::{DialogKind, DialogState};
    use crate::dialog::{Panel, PanelStack};
    use crate::model::Role;
    let mut state = fresh_state();
    let mut panel = Panel::new("compact", "Compact Context")
        .form_field("Keep tokens", "2000", "keep")
        .form_field("Focus", "f", "focus");
    panel.cmd_name = Some("compact".into());
    panel.field_keys.push("keep".into());
    panel.field_keys.push("focus".into());
    panel
        .form_values
        .insert("keep".into(), "not-a-number".into());
    panel.form_values.insert("focus".into(), "".into());
    state.open_dialog = Some(DialogState::Active {
        kind: DialogKind::Generic,
        panels: PanelStack::new(panel),
    });

    state.update(Event::CommandFormSubmit);

    let sys: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    assert!(!sys.is_empty(), "expected a system error message");
    let last = sys.last().unwrap().content().clone();
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
    state.update(crate::Event::Abort);

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
    state.update(crate::Event::CommandFormDown);

    // Navigate up
    state.update(crate::Event::CommandFormUp);
}

#[test]
fn form_backspace_deletes() {
    let mut state = fresh_state();
    type_str(&mut state, "/save");
    state.update(Event::submit());

    // Type some characters
    state.update(crate::Event::Input('a'));
    state.update(crate::Event::Input('b'));
    state.update(crate::Event::Input('c'));

    // Backspace
    state.update(crate::Event::Backspace);

    // Dialog should still be open
    assert!(state.open_dialog.is_some());
}

#[test]
fn form_button_activated_by_enter() {
    use crate::dialog::{ItemAction, Panel};
    let mut panel = Panel::new("test", "Test")
        .form_field("Name", "", "name")
        .item("_Submit", ItemAction::Emit(Event::Save))
        .item("_Cancel", ItemAction::Emit(Event::Cancel));
    // Navigate to the first button (index 1, after form field at index 0)
    panel.selected = 1;
    let mut state = crate::model::AppState::default();
    let action =
        crate::update::dialog::form_panel_action(&mut state, &mut panel, crate::Event::submit());
    assert!(matches!(
        action,
        crate::update::dialog::FormAction::Submit(Some(Event::Save))
    ));
}

#[test]
fn form_button_activated_by_accelerator() {
    use crate::dialog::{ItemAction, Panel};
    let mut panel = Panel::new("test", "Test")
        .form_field("Name", "", "name")
        .item("_Submit", ItemAction::Emit(Event::Save))
        .item("_Cancel", ItemAction::Emit(Event::Cancel));
    // On a form field, typing 'c' should type into the field
    panel.selected = 0;
    let mut state = crate::model::AppState::default();
    let action =
        crate::update::dialog::form_panel_action(&mut state, &mut panel, Event::Input('c'));
    assert!(matches!(
        action,
        crate::update::dialog::FormAction::KeepOpen
    ));
    assert_eq!(panel.form_values.get("name"), Some(&"c".to_string()));

    // On a button, typing 'c' should activate Cancel
    panel.selected = 2;
    let action =
        crate::update::dialog::form_panel_action(&mut state, &mut panel, Event::Input('c'));
    assert!(matches!(
        action,
        crate::update::dialog::FormAction::Submit(Some(Event::Cancel))
    ));
}

#[test]
fn form_field_submits_via_command_registry() {
    // A form with cmd_name set routes through the command registry.
    use crate::dialog::{ItemAction, Panel};
    let mut panel = Panel::new("save", "Save")
        .form_field("Name", "my-session", "name")
        .item("_Submit", ItemAction::Emit(Event::Save));
    panel.cmd_name = Some("save".into());
    panel.field_keys.push("name".into());
    panel.form_values.insert("name".into(), "myses".into());
    // On the form field, Enter should submit the form via command registry
    panel.selected = 0;
    let mut state = crate::model::AppState::default();
    let action =
        crate::update::dialog::form_panel_action(&mut state, &mut panel, crate::Event::submit());
    assert!(matches!(
        action,
        crate::update::dialog::FormAction::SubmitCommand { name, .. } if name == "save"
    ));
}

#[test]
fn form_submit_button_routes_via_command_registry() {
    // A form with cmd_name set uses SubmitCommand, not legacy submit factory.
    use crate::dialog::Panel;
    let mut panel = Panel::new("save", "Save")
        .form_field("Name", "my-session", "name")
        .form_submit();
    panel.cmd_name = Some("save".into());
    panel.field_keys.push("name".into());
    panel.form_values.insert("name".into(), "myses".into());
    // Move selection from the field to the FormSubmit button.
    panel.selected = 1;

    let mut state = crate::model::AppState::default();
    let action =
        crate::update::dialog::form_panel_action(&mut state, &mut panel, crate::Event::submit());

    assert!(
        matches!(
            action,
            crate::update::dialog::FormAction::SubmitCommand { name, .. } if name == "save"
        ),
        "Enter on FormSubmit button should route via command registry"
    );
}

/// Test that `handle_form_dialog(Event::CommandFormSubmit)` dispatches the form
/// submit when a form dialog is open. This is the path the UiActor uses when
/// the user presses Enter with an empty chat input and a form is open.
#[test]
fn handle_form_dialog_submit_dispatches_save_command() {
    let mut state = crate::model::AppState::default();
    // Open the save form via the command palette.
    palette_select(&mut state, "save");
    assert!(
        state.open_dialog.is_some(),
        "form dialog should be open"
    );
    // Verify the root panel is a form.
    let dialog = state.open_dialog.as_ref().unwrap();
    let DialogState::Active { panels, .. } = dialog else {
        panic!("expected Active dialog");
    };
    let root_panel = panels.root().expect("root panel should exist");
    assert!(
        root_panel.is_form(),
        "root panel should be a form"
    );
    // Clone and modify the panel: set a form field value and move to submit button.
    let mut panel = root_panel.clone();
    // Set form values (how the form stores user input).
    panel.form_values.insert("name".to_string(), "test-session".to_string());
    // Move to submit button (the second item in a save form: field + submit button).
    panel.selected = 1;
    // Apply CommandFormSubmit (the event the UiActor sends on Enter with empty input).
    let action =
        crate::update::dialog::form_panel_action(&mut state, &mut panel, crate::Event::CommandFormSubmit);
    // Should produce a SubmitCommand action with the form values.
    match action {
        crate::update::dialog::FormAction::SubmitCommand { name, keys, values } => {
            assert_eq!(name, "save", "should route to save command");
            assert_eq!(keys, &["name"], "should have name field key");
            assert_eq!(
                values.get("name").cloned(),
                Some("test-session".to_string()),
                "should preserve form field value"
            );
        }
        other => panic!("expected SubmitCommand, got {:?}", other),
    }
}
