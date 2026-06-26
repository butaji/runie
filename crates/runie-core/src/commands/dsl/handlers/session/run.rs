//! Session run helpers (name/fork/compact).
//!
//! Functions named `run_*` are canonical handlers matching
//! `fn(&mut AppState, &str) -> CommandResult` for use as
//! `CommandKind::Handler` in the command registry.

use crate::commands::CommandResult;
use crate::model::AppState;

/// Set the session display name.  Returns `CommandResult` for registry use.
pub fn run_name(state: &mut AppState, name: &str) -> CommandResult {
    let name = name.trim();
    if name.is_empty() {
        let current = state
            .session
            .session_display_name
            .as_deref()
            .unwrap_or("(unset)");
        state.add_system_msg(format!("Current display name: {}", current));
        return CommandResult::None;
    }
    let truncated = if name.chars().count() > 64 {
        format!("{}…", name.chars().take(64).collect::<String>())
    } else {
        name.to_owned()
    };
    state.session_mut().session_display_name = Some(truncated.clone());
    state.add_system_msg(format!("Session name set to '{}'", truncated));
    CommandResult::None
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
                state.add_system_msg(format!(
                    "Invalid message index '{}': expected a non-negative integer.",
                    index_raw
                ));
                return CommandResult::None;
            }
        }
    };
    let msg_count = state.session().messages.len();
    if message_index >= msg_count {
        state.add_system_msg(format!(
            "Index {} out of range (0–{})",
            message_index,
            msg_count.saturating_sub(1)
        ));
        return CommandResult::None;
    }
    let mut tree = state.session_mut().session_tree.take().unwrap_or_else(|| {
        crate::session::tree::SessionTree::from_messages(&state.session().messages)
    });
    match tree.fork_at(message_index) {
        Some(path) => {
            tree.navigate_to(&path);
            state.session_mut().session_tree = Some(tree);
            state.add_system_msg(format!("Forked at message {}.", message_index));
        }
        None => state.add_system_msg("Could not fork.".into()),
    }
    CommandResult::None
}

/// Compact the context, keeping the last `keep_raw` tokens.
/// Returns `CommandResult` for registry use.  Accepts args as "keep focus".
pub fn run_compact(state: &mut AppState, args: &str) -> CommandResult {
    let parts: Vec<&str> = args.split_whitespace().collect();
    let keep_raw = parts.first().unwrap_or(&"2000");
    let focus = parts.get(1).unwrap_or(&"");
    let keep = match keep_raw.parse::<usize>() {
        Ok(n) if n > 0 => n,
        _ => {
            state.add_system_msg(format!(
                "Invalid keep value '{}': expected a positive integer.",
                keep_raw
            ));
            return CommandResult::None;
        }
    };
    let msg = state.compact(keep);
    let result = if focus.is_empty() {
        msg
    } else {
        format!("{} (focus: {})", msg, focus)
    };
    state.add_system_msg(result);
    CommandResult::None
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
    let handles = state.actor_handles().cloned();
    let can_spawn = handles.as_ref().is_some() && tokio::runtime::Handle::try_current().is_ok();
    if can_spawn {
        let handles = handles.unwrap();
        tokio::spawn(async move {
            handles.send_save_session(name_owned, session).await;
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
    if send_to_session_store(state, |tx, n| async move { tx.load(n).await }, name) {
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
    if send_to_session_store(state, |tx, n| async move { tx.delete(n).await }, name) {
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
    let handles = state.actor_handles().cloned();
    let can_spawn = handles.as_ref().is_some() && tokio::runtime::Handle::try_current().is_ok();
    if can_spawn {
        let handles = handles.unwrap();
        tokio::spawn(async move {
            handles.send_import_session(path_buf).await;
        });
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
    let name = state.session().session_display_name.clone().unwrap_or_else(|| "exported".into());
    let session = crate::session::Session::from_state(state, name);
    let path_buf = std::path::PathBuf::from(path);
    let handles = state.actor_handles().cloned();
    let can_spawn = handles.as_ref().is_some() && tokio::runtime::Handle::try_current().is_ok();
    if can_spawn {
        let handles = handles.unwrap();
        tokio::spawn(async move {
            handles.send_export_session(path_buf, session).await;
        });
        return CommandResult::Message(format!("Exporting session to '{}'…", path));
    }
    let json = serde_json::to_string_pretty(&session).unwrap_or_default();
    match std::fs::write(&path_buf, json) {
        Ok(_) => CommandResult::Message(format!("Session exported to '{}'", path)),
        Err(e) => CommandResult::Message(format!("Could not export: {}", e)),
    }
}

/// Helper: send an async message to the session store if the actor is available.
fn send_to_session_store<F, Fut>(state: &AppState, f: F, name: &str) -> bool
where
    F: FnOnce(crate::actors::SessionActorHandle, String) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    if let Some(handles) = state.actor_handles() {
        if tokio::runtime::Handle::try_current().is_ok() {
            if let Some(ref session) = handles.session {
                let name = name.to_owned();
                let session = session.clone();
                tokio::spawn(async move {
                    f(session, name).await;
                });
                return true;
            }
        }
    }
    false
}
