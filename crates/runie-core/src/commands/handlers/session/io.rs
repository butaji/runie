//! Session persistence handlers.

use crate::commands::CommandResult;
use crate::model::{now, AppState};

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
    let session = crate::session::Session {
        name: name.clone(),
        display_name: state.session.session_display_name.clone(),
        created_at: state.session.session_created_at,
        updated_at: now(),
        messages: state.session.messages.clone(),
        provider: state.config.current_provider.clone(),
        model: state.config.current_model.clone(),
        theme_name: state.config.theme_name.clone(),
        thinking_level: state.config.thinking_level,
        read_only: state.config.read_only,
        session_tree: state.session.session_tree.clone(),
    };
    match crate::session::save(&name, &session) {
        Ok(_) => CommandResult::Message(format!("Session '{}' saved.", name)),
        Err(e) => CommandResult::Message(format!("Could not save '{}': {}", name, e)),
    }
}

pub fn handle_load(state: &mut AppState, name: &str) -> CommandResult {
    if name.is_empty() {
        // Empty form submission: re-open the form so user can try again.
        return crate::commands::handlers::session::build_load_form();
    }
    match crate::session::load(name) {
        Ok(session) => {
            state.session.messages = session.messages;
            state.config.current_provider = session.provider;
            state.config.current_model = session.model;
            state.config.theme_name = session.theme_name;
            state.config.thinking_level = session.thinking_level;
            state.config.read_only = session.read_only;
            state.session.session_display_name = session.display_name.or(Some(session.name));
            state.session.session_created_at = session.created_at;
            state.session.session_updated_at = session.updated_at;
            state.session.session_tree = session.session_tree;
            state.configure_token_tracker();
            state.messages_changed();
            CommandResult::Message(format!("Session '{}' loaded.", name))
        }
        Err(_) => CommandResult::Message(format!(
            "Session '{}' not found. Use /sessions to list saved sessions.",
            name
        )),
    }
}

pub fn handle_delete(_state: &mut AppState, name: &str) -> CommandResult {
    if name.is_empty() {
        return crate::commands::handlers::session::build_delete_form();
    }
    match crate::session::delete(name) {
        Ok(_) => CommandResult::Message(format!("Session '{}' deleted.", name)),
        Err(_) => CommandResult::Message(format!(
            "Session '{}' not found. Use /sessions to list saved sessions.",
            name
        )),
    }
}

pub fn handle_import(state: &mut AppState, path: &str) -> CommandResult {
    if path.is_empty() {
        return crate::commands::handlers::session::build_import_form();
    }
    match std::fs::read_to_string(path) {
        Ok(json) => match serde_json::from_str::<crate::session::Session>(&json) {
            Ok(session) => {
                state.session.messages = session.messages;
                state.config.current_provider = session.provider;
                state.config.current_model = session.model;
                state.config.theme_name = session.theme_name;
                state.config.thinking_level = session.thinking_level;
                state.config.read_only = session.read_only;
                state.session.session_display_name = session.display_name.or(Some(session.name));
                state.session.session_created_at = session.created_at;
                state.session.session_updated_at = session.updated_at;
                state.session.session_tree = session.session_tree;
                state.configure_token_tracker();
                state.messages_changed();
                CommandResult::Message(format!("Session imported from '{}'", path))
            }
            Err(e) => CommandResult::Message(format!("Invalid session file: {}", e)),
        },
        Err(e) => CommandResult::Message(format!("Could not read file: {}", e)),
    }
}

pub fn handle_export(state: &mut AppState, path: &str) -> CommandResult {
    if path.is_empty() {
        return crate::commands::handlers::session::build_export_form();
    }
    let session = crate::session::Session {
        name: state
            .session
            .session_display_name
            .clone()
            .unwrap_or_else(|| "exported".into()),
        display_name: state.session.session_display_name.clone(),
        created_at: state.session.session_created_at,
        updated_at: now(),
        messages: state.session.messages.clone(),
        provider: state.config.current_provider.clone(),
        model: state.config.current_model.clone(),
        theme_name: state.config.theme_name.clone(),
        thinking_level: state.config.thinking_level,
        read_only: state.config.read_only,
        session_tree: state.session.session_tree.clone(),
    };
    match std::fs::write(
        path,
        serde_json::to_string_pretty(&session).unwrap_or_default(),
    ) {
        Ok(()) => CommandResult::Message(format!("Session exported to '{}'", path)),
        Err(e) => CommandResult::Message(format!("Could not export: {}", e)),
    }
}
