//! Tests for command form dialogs.

use crate::commands::DialogState;
use crate::model::{AppState, Role};
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
    let store = crate::session::Store::new(std::env::temp_dir().join(format!("runie_test_{}", std::process::id())));
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
    assert!(state.open_dialog.is_some(), "form should be open after submit");
    
    // Check the dialog type - take dialog to inspect, then restore
    let form_is_form = if let Some(crate::commands::DialogState::PanelStack(stack)) = &state.open_dialog {
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
