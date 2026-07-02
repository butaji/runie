//! Session run helpers (name/fork/compact).
//!
//! Functions named `run_*` are canonical handlers matching
//! `fn(&mut AppState, &str) -> CommandResult` for use as
//! `CommandKind::Handler` in the command registry.
//!
//! These handlers emit intent events. The actual state mutation logic lives
//! in `handle_command_event` to avoid circular event loops.

use crate::actors::SessionMsg;
use super::COMPACT_DEFAULT_KEEP_TOKENS;
use crate::commands::CommandResult;
use crate::model::AppState;

/// Set the session display name.  Returns `CommandResult` for registry use.
pub fn run_name(_state: &mut AppState, name: &str) -> CommandResult {
    let name = name.trim();
    // Emit intent event; handle_command_event contains the actual mutation logic
    CommandResult::Event(crate::Event::RunNameCommand {
        name: name.to_owned(),
    })
}

/// Default fork index: the most recent user message. 0 if no user messages.
pub fn fallback_fork_index(state: &AppState) -> usize {
    state
        .session
        .messages
        .iter()
        .enumerate()
        .rfind(|(_, m)| m.role == crate::model::Role::User)
        .map(|(i, _)| i)
        .unwrap_or(0)
}

/// Fork the session at a message index.  Returns `CommandResult` for registry use.
pub fn run_fork(state: &mut AppState, index_raw: &str) -> CommandResult {
    let message_index = if index_raw.trim().is_empty() {
        fallback_fork_index(state)
    } else {
        match index_raw.trim().parse::<usize>() {
            Ok(n) => n,
            Err(_) => {
                // Emit event for invalid input; handler will show error
                return CommandResult::Event(crate::Event::RunForkCommand {
                    message_index: index_raw.to_string(),
                });
            }
        }
    };
    let msg_count = state.session().messages.len();
    if message_index >= msg_count {
        // Emit event; handler will show range error
        return CommandResult::Event(crate::Event::RunForkCommand {
            message_index: index_raw.to_string(),
        });
    }
    // Emit ForkSession intent; owning actor handles the mutation
    CommandResult::Event(crate::Event::ForkSession { message_index })
}

/// Compact the context, keeping the last `keep_raw` tokens.
/// Returns `CommandResult` for registry use.  Accepts args as "keep focus".
pub fn run_compact(_state: &mut AppState, args: &str) -> CommandResult {
    let parts: Vec<&str> = args.split_whitespace().collect();
    let keep = parts.first().unwrap_or(&COMPACT_DEFAULT_KEEP_TOKENS).to_string();
    let focus = parts.get(1).unwrap_or(&"").to_string();
    // Emit intent event; owning actor handles the mutation
    CommandResult::Event(crate::Event::RunCompactCommand { keep, focus })
}

// ── Session IO handlers ──────────────────────────────────────────────────────
// These handle save/load/delete/import/export with async support.
// Form submissions pass args as positional values (e.g. "session-name" or
// "2000 focus-value").

/// Save the current session.  Args: session name.
pub fn run_save(state: &mut AppState, name: &str) -> CommandResult {
    let name_owned = name.trim().to_owned();
    if name_owned.is_empty() {
        return CommandResult::Message("Usage: /save name".into());
    }
    let session = crate::session::Session::from_state(state, name_owned.clone());
    // Route through SessionActor in production; fall back to direct save in tests.
    if let Some(h) = state.actor_handles() {
        let _ = h.session.try_send(SessionMsg::Save {
            name: name_owned,
            session,
        });
        CommandResult::Message(format!("Saving session '{}'…", name.trim()))
    } else {
        match crate::session::replay::save_session(&name_owned, state) {
            Ok(_) => CommandResult::Message(format!("Session '{}' saved.", name_owned)),
            Err(e) => CommandResult::Message(format!("Could not save session: {}", e)),
        }
    }
}

/// Load a saved session.  Args: session name.
pub fn run_load(state: &mut AppState, name: &str) -> CommandResult {
    let name = name.trim();
    if name.is_empty() {
        return CommandResult::Message("Usage: /load name".into());
    }
    if send_session_msg(
        state,
        SessionMsg::Load {
            name: name.to_owned(),
        },
    ) {
        return CommandResult::None;
    }
    match crate::session::replay::load_session(name, state) {
        Ok(_) => CommandResult::Message(format!("Session '{}' loaded.", name)),
        Err(_) => CommandResult::Message(format!(
            "Session '{}' not found. Use /sessions to list saved sessions.",
            name
        )),
    }
}

/// Delete a saved session.  Args: session name.
pub fn run_delete(state: &mut AppState, name: &str) -> CommandResult {
    let name = name.trim();
    if name.is_empty() {
        return CommandResult::Message("Usage: /delete name".into());
    }
    if send_session_msg(
        state,
        SessionMsg::Delete {
            name: name.to_owned(),
        },
    ) {
        return CommandResult::None;
    }
    match crate::session::replay::delete_session(name) {
        Ok(_) => CommandResult::Message(format!("Session '{}' deleted.", name)),
        Err(_) => CommandResult::Message(format!(
            "Session '{}' not found. Use /sessions to list saved sessions.",
            name
        )),
    }
}

/// Import a session from a JSON file.  Args: file path.
pub fn run_import(state: &mut AppState, path: &str) -> CommandResult {
    let path = path.trim();
    if path.is_empty() {
        return CommandResult::Message("Usage: /import path/to/session.json".into());
    }
    let path_buf = std::path::PathBuf::from(path);
    // Route through SessionActor in production; fall back to direct import in tests.
    if let Some(h) = state.actor_handles() {
        let _ = h.session.try_send(SessionMsg::Import { path: path_buf });
        return CommandResult::Message(format!("Importing session from '{}'…", path));
    }
    let json = match std::fs::read_to_string(&path_buf) {
        Ok(s) => s,
        Err(_) => return CommandResult::Message(format!("Could not read '{}'", path)),
    };
    match serde_json::from_str::<crate::session::Session>(&json) {
        Ok(session) => {
            state.restore_session(&session);
            CommandResult::Message(format!("Session imported from '{}'", path))
        }
        Err(_) => CommandResult::Message(format!("Could not import session from '{}'", path)),
    }
}

/// Export the current session to a JSON file.  Args: file path.
pub fn run_export(state: &mut AppState, path: &str) -> CommandResult {
    let path = path.trim();
    if path.is_empty() {
        return CommandResult::Message("Usage: /export path/to/session.json".into());
    }
    let name = state
        .session()
        .session_display_name
        .clone()
        .unwrap_or_else(|| "exported".into());
    let session = crate::session::Session::from_state(state, name);
    let path_buf = std::path::PathBuf::from(path);
    // Route through SessionActor in production; fall back to direct export in tests.
    if let Some(h) = state.actor_handles() {
        let _ = h.session.try_send(SessionMsg::Export {
            path: path_buf,
            session,
        });
        return CommandResult::Message(format!("Exporting session to '{}'…", path));
    }
    let json = serde_json::to_string_pretty(&session).unwrap_or_default();
    match std::fs::write(&path_buf, json) {
        Ok(_) => CommandResult::Message(format!("Session exported to '{}'", path)),
        Err(e) => CommandResult::Message(format!("Could not export: {}", e)),
    }
}

/// Helper: send a session message to SessionActor if the actor is available.
fn send_session_msg(state: &AppState, msg: SessionMsg) -> bool {
    if let Some(handles) = state.actor_handles() {
        let _ = handles.session.try_send(msg);
        return true;
    }
    false
}
