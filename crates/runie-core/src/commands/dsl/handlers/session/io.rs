//! Session persistence handlers (save/load/delete/import/export).

use crate::commands::CommandResult;
use crate::model::AppState;

use super::build_delete_form;
use super::build_export_form;
use super::build_import_form;
use super::build_load_form;

/// Save the current session to disk.
pub fn handle_save(state: &mut AppState, name: &str) -> CommandResult {
    let name = if name.is_empty() {
        state
            .session
            .session_display_name
            .clone()
            .unwrap_or_else(|| "unnamed".into())
    } else {
        name.into()
    };
    match crate::session_replay::save_session(&name, state) {
        Ok(_) => CommandResult::Message(format!("Session '{}' saved.", name)),
        Err(e) => CommandResult::Message(format!("Could not save '{}': {}", name, e)),
    }
}

/// Load a saved session from disk.
pub fn handle_load(state: &mut AppState, name: &str) -> CommandResult {
    if name.is_empty() {
        return build_load_form();
    }
    match crate::session_replay::load_session(name, state) {
        Ok(_) => CommandResult::Message(format!("Session '{}' loaded.", name)),
        Err(_) => CommandResult::Message(format!(
            "Session '{}' not found. Use /sessions to list saved sessions.",
            name
        )),
    }
}

/// Delete a saved session from disk.
pub fn handle_delete(_state: &mut AppState, name: &str) -> CommandResult {
    if name.is_empty() {
        return build_delete_form();
    }
    match crate::session_replay::delete_session(name) {
        Ok(_) => CommandResult::Message(format!("Session '{}' deleted.", name)),
        Err(_) => CommandResult::Message(format!(
            "Session '{}' not found. Use /sessions to list saved sessions.",
            name
        )),
    }
}

/// Import a session from a JSON file.
pub fn handle_import(state: &mut AppState, path: &str) -> CommandResult {
    if path.is_empty() {
        return build_import_form();
    }
    match std::fs::read_to_string(path) {
        Ok(json) => match serde_json::from_str::<crate::session::Session>(&json) {
            Ok(session) => {
                let msg = format!("Session imported from '{}'", path);
                state.restore_session(&session);
                CommandResult::Message(msg)
            }
            Err(e) => CommandResult::Message(format!("Invalid session file: {}", e)),
        },
        Err(e) => CommandResult::Message(format!("Could not read file: {}", e)),
    }
}

/// Export the current session to a JSON file.
pub fn handle_export(state: &mut AppState, path: &str) -> CommandResult {
    if path.is_empty() {
        return build_export_form();
    }
    let name = state
        .session
        .session_display_name
        .clone()
        .unwrap_or_else(|| "exported".into());
    let session = crate::session::Session::from_state(state, name);
    match std::fs::write(
        path,
        serde_json::to_string_pretty(&session).unwrap_or_default(),
    ) {
        Ok(()) => CommandResult::Message(format!("Session exported to '{}'", path)),
        Err(e) => CommandResult::Message(format!("Could not export: {}", e)),
    }
}
