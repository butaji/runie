use crate::commands::{CommandCategory, CommandDef, CommandHandler, CommandRegistry, CommandResult};
use crate::model::AppState;

pub fn register(registry: &mut CommandRegistry) {
    registry.register(cmd("save", "Save current session", &[], CommandCategory::Session, handle_save));
    registry.register(cmd("load", "Load a saved session", &[], CommandCategory::Session, handle_load));
    registry.register(cmd("sessions", "List saved sessions", &[], CommandCategory::Session, handle_sessions));
    registry.register(cmd("delete", "Delete a saved session", &[], CommandCategory::Session, handle_delete));
    registry.register(cmd("name", "Set session display name", &[], CommandCategory::Session, handle_name));
    registry.register(cmd("export", "Export session to JSON", &[], CommandCategory::Session, handle_export));
    registry.register(cmd("import", "Import session from JSON", &[], CommandCategory::Session, handle_import));
    registry.register(cmd("new", "Start new session", &[], CommandCategory::Session, handle_new));
    registry.register(cmd("resume", "Resume most recent session", &[], CommandCategory::Session, handle_resume));
    registry.register(cmd("compact", "Compact context", &[], CommandCategory::Session, handle_compact));
    registry.register(cmd("reset", "Clear all state", &[], CommandCategory::Session, handle_reset));
    registry.register(cmd("history", "Show recent history", &[], CommandCategory::Session, handle_history));
    registry.register(cmd("session", "Show session info", &[], CommandCategory::Session, handle_session));
}

fn cmd(name: &str, desc: &str, aliases: &[&str], category: CommandCategory, handler: CommandHandler) -> CommandDef {
    CommandDef {
        name: name.into(),
        description: desc.into(),
        aliases: aliases.iter().map(|s| s.to_string()).collect(),
        category,
        handler,
        completer: None,
    }
}

fn handle_save(state: &mut AppState, args: &str) -> CommandResult {
    let name = args;
    if name.is_empty() {
        return CommandResult::Message("Usage: /save name".into());
    }
    let now = crate::update::now();
    let session = crate::session::Session {
        name: name.to_string(),
        display_name: state.session_display_name.clone(),
        created_at: state.session_created_at,
        updated_at: now,
        messages: state.messages.clone(),
        provider: state.current_provider.clone(),
        model: state.current_model.clone(),
        theme_name: state.theme_name.clone(),
        thinking_level: state.thinking_level,
        read_only: state.read_only,
    };
    match crate::session::save(name, &session) {
        Ok(()) => {
            state.session_updated_at = now;
            CommandResult::Message(format!("Session '{}' saved.", name))
        }
        Err(e) => CommandResult::Message(format!("Could not save '{}': {}", name, e)),
    }
}

fn handle_load(state: &mut AppState, args: &str) -> CommandResult {
    let name = args;
    if name.is_empty() {
        return CommandResult::Message("Usage: /load name".into());
    }
    match crate::session::load(name) {
        Ok(session) => {
            state.messages = session.messages;
            state.current_provider = session.provider;
            state.current_model = session.model;
            state.theme_name = session.theme_name;
            state.thinking_level = session.thinking_level;
            state.read_only = session.read_only;
            state.session_display_name = session.display_name.or(Some(session.name));
            state.session_created_at = session.created_at;
            state.session_updated_at = session.updated_at;
            state.messages_changed();
            CommandResult::Message(format!("Session '{}' loaded.", name))
        }
        Err(_) => CommandResult::Message(format!(
            "Session '{}' not found. Use /sessions to list saved sessions.",
            name
        )),
    }
}

fn handle_sessions(_state: &mut AppState, _args: &str) -> CommandResult {
    match crate::session::list() {
        Ok(sessions) => {
            if sessions.is_empty() {
                CommandResult::Message("No saved sessions. Use /save name to create one.".into())
            } else {
                CommandResult::Message(format!("Saved sessions:\n{}", sessions.join("\n")))
            }
        }
        Err(e) => CommandResult::Message(format!("Could not list sessions: {}", e)),
    }
}

fn handle_delete(_state: &mut AppState, args: &str) -> CommandResult {
    let name = args;
    if name.is_empty() {
        return CommandResult::Message("Usage: /delete name".into());
    }
    match crate::session::delete(name) {
        Ok(()) => CommandResult::Message(format!("Session '{}' deleted.", name)),
        Err(_) => CommandResult::Message(format!(
            "Session '{}' not found. Use /sessions to list saved sessions.",
            name
        )),
    }
}

fn handle_name(state: &mut AppState, args: &str) -> CommandResult {
    let name = args.trim();
    if name.is_empty() {
        let current = state.session_display_name.as_deref().unwrap_or("(unset)");
        return CommandResult::Message(format!("Current display name: {}", current));
    }
    let truncated = if name.chars().count() > 64 {
        format!("{}…", name.chars().take(64).collect::<String>())
    } else {
        name.to_string()
    };
    state.session_display_name = Some(truncated.clone());
    CommandResult::Message(format!("Session name set to '{}'", truncated))
}

fn handle_export(state: &mut AppState, args: &str) -> CommandResult {
    let path = args.trim();
    let path = if path.is_empty() {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        format!("{}_{}.json", state.session_display_name.as_deref().unwrap_or("session"), timestamp)
    } else {
        path.to_string()
    };
    let session = crate::session::Session {
        name: state.session_display_name.clone().unwrap_or_else(|| "exported".into()),
        display_name: state.session_display_name.clone(),
        created_at: state.session_created_at,
        updated_at: crate::update::now(),
        messages: state.messages.clone(),
        provider: state.current_provider.clone(),
        model: state.current_model.clone(),
        theme_name: state.theme_name.clone(),
        thinking_level: state.thinking_level,
        read_only: state.read_only,
    };
    match std::fs::write(&path, serde_json::to_string_pretty(&session).unwrap_or_default()) {
        Ok(()) => CommandResult::Message(format!("Session exported to '{}'", path)),
        Err(e) => CommandResult::Message(format!("Could not export: {}", e)),
    }
}

fn handle_import(state: &mut AppState, args: &str) -> CommandResult {
    let path = args.trim();
    if path.is_empty() {
        return CommandResult::Message("Usage: /import filename.json".into());
    }
    match std::fs::read_to_string(path) {
        Ok(json) => match serde_json::from_str::<crate::session::Session>(&json) {
            Ok(session) => {
                state.messages = session.messages;
                state.current_provider = session.provider;
                state.current_model = session.model;
                state.theme_name = session.theme_name;
                state.thinking_level = session.thinking_level;
                state.read_only = session.read_only;
                state.session_display_name = session.display_name.or(Some(session.name));
                state.session_created_at = session.created_at;
                state.session_updated_at = session.updated_at;
                state.messages_changed();
                CommandResult::Message(format!("Session imported from '{}'", path))
            }
            Err(e) => CommandResult::Message(format!("Invalid session file: {}", e)),
        },
        Err(e) => CommandResult::Message(format!("Could not read file: {}", e)),
    }
}

fn handle_new(state: &mut AppState, _args: &str) -> CommandResult {
    state.messages.clear();
    state.input.clear();
    state.cursor_pos = 0;
    state.message_queue.clear();
    state.request_queue.clear();
    state.current_provider = state.config_provider.clone();
    state.current_model = state.config_model.clone();
    state.session_display_name = None;
    let now = crate::update::now();
    state.session_created_at = now;
    state.session_updated_at = now;
    state.messages_changed();
    CommandResult::Message("New session started".into())
}

fn handle_resume(state: &mut AppState, _args: &str) -> CommandResult {
    match find_most_recent_session() {
        Some(name) => handle_load(state, &name),
        None => CommandResult::Message("No sessions to resume".into()),
    }
}

fn find_most_recent_session() -> Option<String> {
    let store = crate::session::default_store()?;
    let names = store.list().ok()?;
    let mut most_recent = None;
    let mut most_recent_time = 0.0f64;
    for name in names {
        if let Ok(session) = store.load(&name) {
            if session.updated_at > most_recent_time {
                most_recent_time = session.updated_at;
                most_recent = Some(name);
            }
        }
    }
    most_recent
}

fn handle_compact(state: &mut AppState, args: &str) -> CommandResult {
    let keep = 2000usize;
    let msg = state.compact(keep);
    let result = if args.trim().is_empty() {
        msg
    } else {
        format!("{} (focus: {})", msg, args.trim())
    };
    CommandResult::Message(result)
}

fn handle_reset(state: &mut AppState, _args: &str) -> CommandResult {
    *state = AppState::default();
    CommandResult::Message("State cleared.".into())
}

fn handle_history(state: &mut AppState, _args: &str) -> CommandResult {
    if state.input_history.is_empty() {
        return CommandResult::Message("No history. Commands you send become part of your history.".into());
    }
    let count = state.input_history.len();
    let last = state.input_history.iter().rev().take(10).collect::<Vec<_>>();
    let entries: Vec<String> = last
        .iter()
        .enumerate()
        .map(|(i, e)| format!("{}: {}", i + 1, e))
        .collect();
    CommandResult::Message(format!(
        "History ({} total, showing last 10). Use ↑/↓ to navigate.\n{}",
        count,
        entries.join("\n")
    ))
}

fn handle_session(state: &mut AppState, _args: &str) -> CommandResult {
    let total_tokens: usize = state.messages.iter().map(|m| crate::tokens::estimate_tokens(&m.content)).sum();
    let msg_count = state.messages.len();
    let user_msgs = state.messages.iter().filter(|m| m.role == crate::model::Role::User).count();
    let assistant_msgs = state.messages.iter().filter(|m| m.role == crate::model::Role::Assistant).count();
    let tool_msgs = state.messages.iter().filter(|m| m.role == crate::model::Role::Tool).count();

    let info = format!(
        "Session: {}\n\
         Messages: {} total ({} user, {} assistant, {} tool)\n\
         Tokens: {} estimated\n\
         Provider: {}\n\
         Model: {}\n\
         Created: {}\n\
         Updated: {}",
        state.session_display_name.as_deref().unwrap_or("unnamed"),
        msg_count, user_msgs, assistant_msgs, tool_msgs,
        total_tokens,
        state.current_provider,
        state.current_model,
        crate::labels::format_timestamp(state.session_created_at),
        crate::labels::format_timestamp(state.session_updated_at),
    );
    CommandResult::Message(info)
}
