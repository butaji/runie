//! Session commands using the new DSL

use crate::commands::{CommandCategory, CommandRegistry, CommandResult};
use crate::model::AppState;

pub mod io;
pub mod run;

pub(crate) use run::{run_compact, run_fork, run_name};

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
    let prompt = if state.input.current_prompt.is_empty() {
        "default"
    } else {
        &state.input.current_prompt
    };
    let read_only = if state.config.read_only { "on" } else { "off" };
    let trust = project_trust_status(state);
    let info = format!(
        "Session: {}\nMessages: {} ({} user, {} assistant, {} tool)\nTokens: {} estimated\nProvider: {}\nModel: {}\nPrompt: {}\nThinking: {}\nRead-only: {}\nTrust: {}\nCreated: {}\nUpdated: {}",
        state.session.session_display_name.as_deref().unwrap_or("unnamed"),
        state.session.messages.len(), user, assistant, tool, tokens,
        state.config.current_provider, state.config.current_model,
        prompt,
        state.config.thinking_level.as_str(),
        read_only,
        trust,
        crate::labels::format_timestamp(state.session.session_created_at),
        crate::labels::format_timestamp(state.session.session_updated_at),
    );
    CommandResult::Message(info)
}

fn project_trust_status(_state: &AppState) -> &'static str {
    let cwd = std::env::current_dir().unwrap_or_default();
    let tm = crate::trust::TrustManager::load();
    match tm.decision_for(&cwd) {
        Some(crate::trust::TrustDecision::Trusted) => "trusted",
        Some(crate::trust::TrustDecision::Untrusted) => "untrusted",
        None => "default",
    }
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

