//! Session commands using the new DSL

use crate::commands::{CommandCategory, CommandRegistry, CommandResult};
use crate::model::{now, AppState};

pub fn register(registry: &mut CommandRegistry) {
    // Form commands (always show dialog, pre-fill from args)
    registry.register(
        crate::cmd!("save")
            .desc("Save current session")
            .category(CommandCategory::Session)
            .sub()
            .form(
                "Save Session",
                |f| f.field("Name", "session-name", "name"),
                crate::Event::RunSaveCommand {
                    name: String::new(),
                },
            ),
    );

    registry.register(
        crate::cmd!("load")
            .desc("Load a saved session")
            .category(CommandCategory::Session)
            .sub()
            .form(
                "Load Session",
                |f| f.field("Name", "session-name", "name"),
                crate::Event::RunLoadCommand {
                    name: String::new(),
                },
            ),
    );

    registry.register(
        crate::cmd!("delete")
            .desc("Delete a saved session")
            .category(CommandCategory::Session)
            .sub()
            .form(
                "Delete Session",
                |f| f.field("Name", "session-name", "name"),
                crate::Event::RunDeleteCommand {
                    name: String::new(),
                },
            ),
    );

    registry.register(
        crate::cmd!("export")
            .desc("Export session to JSON")
            .category(CommandCategory::Session)
            .sub()
            .form(
                "Export Session",
                |f| f.field("Path", "session.json", "path"),
                crate::Event::RunExportCommand {
                    path: String::new(),
                },
            ),
    );

    registry.register(
        crate::cmd!("import")
            .desc("Import session from JSON")
            .category(CommandCategory::Session)
            .sub()
            .form(
                "Import Session",
                |f| f.field("Path", "session.json", "path"),
                crate::Event::RunImportCommand {
                    path: String::new(),
                },
            ),
    );

    // Immediate commands
    registry.register(
        crate::cmd!("sessions")
            .desc("List saved sessions")
            .category(CommandCategory::Session)
            .handler(handle_sessions),
    );

    registry.register(
        crate::cmd!("new")
            .desc("Start new session")
            .category(CommandCategory::Session)
            .handler(handle_new),
    );

    registry.register(
        crate::cmd!("reset")
            .desc("Clear all state")
            .category(CommandCategory::Session)
            .handler(handle_reset),
    );

    registry.register(
        crate::cmd!("history")
            .desc("Show recent history")
            .category(CommandCategory::Session)
            .handler(handle_history),
    );

    registry.register(
        crate::cmd!("session")
            .desc("Show session info")
            .category(CommandCategory::Session)
            .handler(handle_session),
    );

    registry.register(
        crate::cmd!("clone")
            .desc("Clone current session position")
            .category(CommandCategory::Session)
            .handler(handle_clone),
    );

    registry.register(
        crate::cmd!("tree")
            .desc("Open session tree dialog")
            .category(CommandCategory::Session)
            .sub()
            .handler(handle_tree),
    );

    registry.register(
        crate::cmd!("share")
            .desc("Share session as GitHub gist")
            .category(CommandCategory::Session)
            .handler(handle_share),
    );

    registry.register(
        crate::cmd!("resume")
            .desc("Resume most recent session")
            .category(CommandCategory::Session)
            .handler(handle_resume),
    );

    registry.register(
        crate::cmd!("compact")
            .desc("Compact context")
            .category(CommandCategory::Session)
            .sub()
            .form(
                "Compact Context",
                |f| {
                    f.field("Keep tokens", "2000", "keep").field(
                        "Focus",
                        "optional focus keyword",
                        "focus",
                    )
                },
                crate::Event::RunCompactCommand {
                    keep: String::new(),
                    focus: String::new(),
                },
            ),
    );

    registry.register(
        crate::cmd!("fork")
            .desc("Fork session from a message")
            .category(CommandCategory::Session)
            .sub()
            .form(
                "Fork Session",
                |f| f.field("Message index", "0", "index"),
                crate::Event::RunForkCommand {
                    message_index: String::new(),
                },
            ),
    );

    registry.register(
        crate::cmd!("name")
            .desc("Set session display name")
            .category(CommandCategory::Session)
            .sub()
            .form(
                "Set Session Name",
                |f| f.field("Name", "session-name", "name"),
                crate::Event::RunNameCommand {
                    name: String::new(),
                },
            ),
    );
}

// ============================================================================
// Handlers
// ============================================================================

fn handle_sessions(_: &mut AppState, _: &str) -> CommandResult {
    match crate::session::list() {
        Ok(sessions) if sessions.is_empty() => {
            CommandResult::Message("No saved sessions. Use /save name to create one.".into())
        }
        Ok(sessions) => CommandResult::Message(format!("Saved sessions:\n{}", sessions.join("\n"))),
        Err(e) => CommandResult::Message(format!("Could not list sessions: {}", e)),
    }
}

fn handle_new(state: &mut AppState, _: &str) -> CommandResult {
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

fn handle_reset(state: &mut AppState, _: &str) -> CommandResult {
    *state = AppState::default();
    CommandResult::Message("State cleared.".into())
}

fn handle_history(state: &mut AppState, _: &str) -> CommandResult {
    if state.input_history.is_empty() {
        return CommandResult::Message("No history.".into());
    }
    let count = state.input_history.len();
    let entries: Vec<_> = state
        .input_history
        .iter()
        .rev()
        .take(10)
        .enumerate()
        .map(|(i, e)| format!("{}: {}", i + 1, e))
        .collect();
    CommandResult::Message(format!(
        "History ({} total):\n{}",
        count,
        entries.join("\n")
    ))
}

fn handle_session(state: &mut AppState, _: &str) -> CommandResult {
    let tokens: usize = state
        .session
        .messages
        .iter()
        .map(|m| crate::tokens::estimate_tokens(&m.content))
        .sum();
    let (user, assistant, tool) = (
        state
            .session
            .messages
            .iter()
            .filter(|m| m.role == crate::model::Role::User)
            .count(),
        state
            .session
            .messages
            .iter()
            .filter(|m| m.role == crate::model::Role::Assistant)
            .count(),
        state
            .session
            .messages
            .iter()
            .filter(|m| m.role == crate::model::Role::Tool)
            .count(),
    );
    let info = format!(
        "Session: {}\nMessages: {} ({} user, {} assistant, {} tool)\nTokens: {} estimated\nProvider: {}\nModel: {}\nCreated: {}\nUpdated: {}",
        state.session.session_display_name.as_deref().unwrap_or("unnamed"),
        state.session.messages.len(), user, assistant, tool, tokens,
        state.config.current_provider, state.config.current_model,
        crate::labels::format_timestamp(state.session.session_created_at),
        crate::labels::format_timestamp(state.session.session_updated_at),
    );
    CommandResult::Message(info)
}

fn handle_clone(state: &mut AppState, _: &str) -> CommandResult {
    let tree = state.session.session_tree.clone().unwrap_or_else(|| {
        crate::session_tree::SessionTree::from_messages(&state.session.messages)
    });
    state.session.session_tree = Some(tree);
    CommandResult::Message("Session cloned.".into())
}

fn handle_tree(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Event(crate::Event::ToggleSessionTree)
}

fn handle_share(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Event(crate::Event::ShareSession)
}

fn handle_resume(state: &mut AppState, _: &str) -> CommandResult {
    match find_most_recent() {
        Some(name) => match crate::session::load(&name) {
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
                CommandResult::Message(format!("Loaded '{}'.", name))
            }
            Err(_) => CommandResult::Message("Could not load session.".into()),
        },
        None => CommandResult::Message("No sessions to resume.".into()),
    }
}

fn find_most_recent() -> Option<String> {
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

// Form handlers (called from update/mod.rs with form values)

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
        return CommandResult::Message("Usage: /load <session-name>".into());
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

pub fn handle_delete(_state: &mut AppState, name: &str) -> CommandResult {
    if name.is_empty() {
        return CommandResult::Message("Usage: /delete <session-name>".into());
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
        return CommandResult::Message("Usage: /import <path>".into());
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

pub fn handle_export(state: &mut AppState, path: &str) -> CommandResult {
    if path.is_empty() {
        return CommandResult::Message("Usage: /export <path>".into());
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

// ============================================================================
// Form-submit handlers (called from update/mod.rs with form values).
// These are direct side-effect functions (not CommandResult) because they run
// after the form has been validated.
// ============================================================================

/// Set or display the session display name. Empty name shows the current one.
pub(crate) fn run_name(state: &mut AppState, name: &str) {
    let name = name.trim();
    if name.is_empty() {
        let current = state
            .session
            .session_display_name
            .as_deref()
            .unwrap_or("(unset)");
        state.add_system_msg(format!("Current display name: {}", current));
        return;
    }
    let truncated = if name.chars().count() > 64 {
        format!("{}…", name.chars().take(64).collect::<String>())
    } else {
        name.to_string()
    };
    state.session.session_display_name = Some(truncated.clone());
    state.add_system_msg(format!("Session name set to '{}'", truncated));
}

/// Default fork index: the most recent user message. 0 if no user messages.
pub(crate) fn fallback_fork_index(state: &AppState) -> usize {
    state
        .session
        .messages
        .iter()
        .enumerate()
        .rfind(|(_, m)| m.role == crate::model::Role::User)
        .map(|(i, _)| i)
        .unwrap_or(0)
}

/// Fork the session at a message index. Empty/non-numeric out-of-range → error.
pub(crate) fn run_fork(state: &mut AppState, index_raw: &str) {
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
                return;
            }
        }
    };
    if message_index >= state.session.messages.len() {
        state.add_system_msg(format!(
            "Index {} out of range (0–{})",
            message_index,
            state.session.messages.len().saturating_sub(1)
        ));
        return;
    }
    let mut tree = state.session.session_tree.take().unwrap_or_else(|| {
        crate::session_tree::SessionTree::from_messages(&state.session.messages)
    });
    match tree.fork_at(message_index) {
        Some(path) => {
            tree.navigate_to(&path);
            state.session.session_tree = Some(tree);
            state.add_system_msg(format!("Forked at message {}.", message_index));
        }
        None => state.add_system_msg("Could not fork.".into()),
    }
}

/// Compact the context, keeping the last `keep_raw` tokens. Empty/non-numeric
/// keep → default 2000 (empty) or error (non-numeric). Empty focus means no
/// focus keyword in the resulting message.
pub(crate) fn run_compact(state: &mut AppState, keep_raw: &str, focus: &str) {
    let keep = if keep_raw.trim().is_empty() {
        2000
    } else {
        match keep_raw.trim().parse::<usize>() {
            Ok(n) if n > 0 => n,
            Ok(_) => 2000,
            Err(_) => {
                state.add_system_msg(format!(
                    "Invalid keep value '{}': expected a positive integer.",
                    keep_raw
                ));
                return;
            }
        }
    };
    let msg = state.compact(keep);
    let result = if focus.is_empty() {
        msg
    } else {
        format!("{} (focus: {})", msg, focus)
    };
    state.add_system_msg(result);
}
