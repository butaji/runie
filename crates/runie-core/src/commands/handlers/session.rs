use crate::commands::{command_form_dialog, CommandCategory, CommandDef, CommandHandler, CommandRegistry, CommandResult};
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
    registry.register(cmd("fork", "Fork session from a message", &[], CommandCategory::Session, handle_fork));
    registry.register(cmd("clone", "Clone current session position", &[], CommandCategory::Session, handle_clone));
    registry.register(cmd("tree", "Open session tree dialog", &[], CommandCategory::Session, handle_tree));
    registry.register(cmd("share", "Share session as GitHub gist", &[], CommandCategory::Session, handle_share));
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

pub fn handle_save(state: &mut AppState, args: &str) -> CommandResult {
    let name = args.trim();
    if name.is_empty() {
        // Open form dialog
        let form = command_form_dialog(
            "save",
            "Save Session",
            vec![("Name", "session-name", "name")],
            crate::Event::RunSaveCommand,
        );
        return CommandResult::OpenPanelStack(form);
    }
    let now = crate::update::now();
    let session = crate::session::Session {
        name: name.to_string(),
        display_name: state.session.session_display_name.clone(),
        created_at: state.session.session_created_at,
        updated_at: now,
        messages: state.session.messages.clone(),
        provider: state.config.current_provider.clone(),
        model: state.config.current_model.clone(),
        theme_name: state.config.theme_name.clone(),
        thinking_level: state.config.thinking_level,
        read_only: state.config.read_only,
        session_tree: state.session.session_tree.clone(),
    };
    match crate::session::save(name, &session) {
        Ok(()) => {
            state.session.session_updated_at = now;
            CommandResult::Message(format!("Session '{}' saved.", name))
        }
        Err(e) => CommandResult::Message(format!("Could not save '{}': {}", name, e)),
    }
}

pub fn handle_load(state: &mut AppState, args: &str) -> CommandResult {
    let name = args.trim();
    if name.is_empty() {
        // Open sessions list form
        let form = command_form_dialog(
            "load",
            "Load Session",
            vec![("Name", "session-name", "name")],
            crate::Event::RunLoadCommand { name: String::new() },
        );
        return CommandResult::OpenPanelStack(form);
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

pub fn handle_delete(_state: &mut AppState, args: &str) -> CommandResult {
    let name = args.trim();
    if name.is_empty() {
        let form = command_form_dialog(
            "delete",
            "Delete Session",
            vec![("Name", "session-name", "name")],
            crate::Event::RunDeleteCommand { name: String::new() },
        );
        return CommandResult::OpenPanelStack(form);
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
        let current = state.session.session_display_name.as_deref().unwrap_or("(unset)");
        return CommandResult::Message(format!("Current display name: {}", current));
    }
    let truncated = if name.chars().count() > 64 {
        format!("{}…", name.chars().take(64).collect::<String>())
    } else {
        name.to_string()
    };
    state.session.session_display_name = Some(truncated.clone());
    CommandResult::Message(format!("Session name set to '{}'", truncated))
}

pub fn handle_export(state: &mut AppState, args: &str) -> CommandResult {
    let path = args.trim();
    if path.is_empty() {
        let form = command_form_dialog(
            "export",
            "Export Session",
            vec![("Path", "session.json", "path")],
            crate::Event::RunExportCommand { path: String::new() },
        );
        return CommandResult::OpenPanelStack(form);
    }
    let session = crate::session::Session {
        name: state.session.session_display_name.clone().unwrap_or_else(|| "exported".into()),
        display_name: state.session.session_display_name.clone(),
        created_at: state.session.session_created_at,
        updated_at: crate::update::now(),
        messages: state.session.messages.clone(),
        provider: state.config.current_provider.clone(),
        model: state.config.current_model.clone(),
        theme_name: state.config.theme_name.clone(),
        thinking_level: state.config.thinking_level,
        read_only: state.config.read_only,
        session_tree: state.session.session_tree.clone(),
    };
    match std::fs::write(&path, serde_json::to_string_pretty(&session).unwrap_or_default()) {
        Ok(()) => CommandResult::Message(format!("Session exported to '{}'", path)),
        Err(e) => CommandResult::Message(format!("Could not export: {}", e)),
    }
}

pub fn handle_import(state: &mut AppState, args: &str) -> CommandResult {
    let path = args.trim();
    if path.is_empty() {
        let form = command_form_dialog(
            "import",
            "Import Session",
            vec![("Path", "session.json", "path")],
            crate::Event::RunImportCommand { path: String::new() },
        );
        return CommandResult::OpenPanelStack(form);
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
                state.messages_changed();
                CommandResult::Message(format!("Session imported from '{}'", path))
            }
            Err(e) => CommandResult::Message(format!("Invalid session file: {}", e)),
        },
        Err(e) => CommandResult::Message(format!("Could not read file: {}", e)),
    }
}

fn handle_new(state: &mut AppState, _args: &str) -> CommandResult {
    state.session.messages.clear();
    state.input.input.clear();
    state.input.cursor_pos = 0;
    state.agent.message_queue.clear();
    state.agent.request_queue.clear();
    state.config.current_provider = state.config.config_provider.clone();
    state.config.current_model = state.config.config_model.clone();
    state.session.session_display_name = None;
    let now = crate::update::now();
    state.session.session_created_at = now;
    state.session.session_updated_at = now;
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
    let total_tokens: usize = state.session.messages.iter().map(|m| crate::tokens::estimate_tokens(&m.content)).sum();
    let msg_count = state.session.messages.len();
    let user_msgs = state.session.messages.iter().filter(|m| m.role == crate::model::Role::User).count();
    let assistant_msgs = state.session.messages.iter().filter(|m| m.role == crate::model::Role::Assistant).count();
    let tool_msgs = state.session.messages.iter().filter(|m| m.role == crate::model::Role::Tool).count();

    let prompt_info = if state.current_prompt.is_empty() {
        "default".to_string()
    } else {
        state.current_prompt.clone()
    };
    let info = format!(
        "Session: {}\n\
         Messages: {} total ({} user, {} assistant, {} tool)\n\
         Tokens: {} estimated\n\
         Provider: {}\n\
         Model: {}\n\
         Prompt: {}\n\
         Created: {}\n\
         Updated: {}",
        state.session.session_display_name.as_deref().unwrap_or("unnamed"),
        msg_count, user_msgs, assistant_msgs, tool_msgs,
        total_tokens,
        state.config.current_provider,
        state.config.current_model,
        prompt_info,
        crate::labels::format_timestamp(state.session.session_created_at),
        crate::labels::format_timestamp(state.session.session_updated_at),
    );
    CommandResult::Message(info)
}

fn handle_fork(state: &mut AppState, args: &str) -> CommandResult {
    let index = args.trim().parse::<usize>().unwrap_or_else(|_| {
        // Default to last user message if no index given
        state.session.messages.iter().enumerate().rfind(|(_, m)| m.role == crate::model::Role::User).map(|(i, _)| i).unwrap_or(0)
    });
    if index >= state.session.messages.len() {
        return CommandResult::Message(format!("Message index {} out of range (0–{})", index, state.session.messages.len().saturating_sub(1)));
    }
    // Initialize or update session tree
    let mut tree = state.session.session_tree.take().unwrap_or_else(|| {
        crate::session_tree::SessionTree::from_messages(&state.session.messages)
    });
    match tree.fork_at(index) {
        Some(path) => {
            tree.navigate_to(&path);
            state.session.session_tree = Some(tree);
            CommandResult::Message(format!("Forked at message {}. New branch created.", index))
        }
        None => CommandResult::Message("Could not fork at that message.".into()),
    }
}

fn handle_clone(state: &mut AppState, _args: &str) -> CommandResult {
    let tree = state.session.session_tree.clone().unwrap_or_else(|| {
        crate::session_tree::SessionTree::from_messages(&state.session.messages)
    });
    state.session.session_tree = Some(tree);
    CommandResult::Message("Session cloned at current position.".into())
}

fn handle_tree(_state: &mut AppState, _args: &str) -> CommandResult {
    CommandResult::Event(crate::Event::ToggleSessionTree)
}

fn handle_share(_state: &mut AppState, _args: &str) -> CommandResult {
    CommandResult::Event(crate::Event::ShareSession)
}
