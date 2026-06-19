//! Session commands.
//!
//! Session IO (save/load/delete/import/export) is handled via
//! `CommandEvent::Run*Command` in `update::command`.
//! Session run helpers (name/fork/compact) live in `run.rs`.

pub mod run;

pub use run::{run_compact, run_fork, run_name};

use crate::commands::{CommandCategory, CommandRegistry, CommandResult};
use crate::model::AppState;

use std::collections::HashMap;

use super::spec::{CommandKind, CommandSpec};

/// Build a `CommandEvent` from a single form field.
fn make_submit<F>(values: &HashMap<String, String>, key: &str, f: F) -> crate::Event
where
    F: FnOnce(String) -> crate::Event,
{
    f(crate::dialog::dsl::get_field(values, key))
}

fn save_submit(values: &HashMap<String, String>) -> crate::Event {
    make_submit(values, "name", |name| {
        crate::event::CommandEvent::RunSaveCommand { name }
    })
}
fn load_submit(values: &HashMap<String, String>) -> crate::Event {
    make_submit(values, "name", |name| {
        crate::event::CommandEvent::RunLoadCommand { name }
    })
}
fn delete_submit(values: &HashMap<String, String>) -> crate::Event {
    make_submit(values, "name", |name| {
        crate::event::CommandEvent::RunDeleteCommand { name }
    })
}
fn export_submit(values: &HashMap<String, String>) -> crate::Event {
    make_submit(values, "path", |path| {
        crate::event::CommandEvent::RunExportCommand { path }
    })
}
fn import_submit(values: &HashMap<String, String>) -> crate::Event {
    make_submit(values, "path", |path| {
        crate::event::CommandEvent::RunImportCommand { path }
    })
}
fn compact_submit(values: &HashMap<String, String>) -> crate::Event {
    crate::event::CommandEvent::RunCompactCommand {
        keep: crate::dialog::dsl::get_field(values, "keep"),
        focus: crate::dialog::dsl::get_field(values, "focus"),
    }
}
fn fork_submit(values: &HashMap<String, String>) -> crate::Event {
    crate::event::CommandEvent::RunForkCommand {
        message_index: crate::dialog::dsl::get_field(values, "index"),
    }
}
fn name_submit(values: &HashMap<String, String>) -> crate::Event {
    make_submit(values, "name", |name| {
        crate::event::CommandEvent::RunNameCommand { name }
    })
}

static SESSION_COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        name: "save",
        desc: "Save current session",
        aliases: &[],
        category: CommandCategory::Session,
        sub: true,
        kind: CommandKind::Form {
            title: "Save Session",
            fields: &[("Name", "session-name", "name")],
            submit: save_submit,
        },
    },
    CommandSpec {
        name: "load",
        desc: "Load a saved session",
        aliases: &[],
        category: CommandCategory::Session,
        sub: true,
        kind: CommandKind::Form {
            title: "Load Session",
            fields: &[("Name", "session-name", "name")],
            submit: load_submit,
        },
    },
    CommandSpec {
        name: "delete",
        desc: "Delete a saved session",
        aliases: &[],
        category: CommandCategory::Session,
        sub: true,
        kind: CommandKind::Form {
            title: "Delete Session",
            fields: &[("Name", "session-name", "name")],
            submit: delete_submit,
        },
    },
    CommandSpec {
        name: "export",
        desc: "Export session to JSON",
        aliases: &[],
        category: CommandCategory::Session,
        sub: true,
        kind: CommandKind::Form {
            title: "Export Session",
            fields: &[("Path", "session.json", "path")],
            submit: export_submit,
        },
    },
    CommandSpec {
        name: "import",
        desc: "Import session from JSON",
        aliases: &[],
        category: CommandCategory::Session,
        sub: true,
        kind: CommandKind::Form {
            title: "Import Session",
            fields: &[("Path", "session.json", "path")],
            submit: import_submit,
        },
    },
    CommandSpec {
        name: "sessions",
        desc: "List saved sessions",
        aliases: &[],
        category: CommandCategory::Session,
        sub: false,
        kind: CommandKind::Handler(handle_sessions),
    },
    CommandSpec {
        name: "new",
        desc: "Start new session",
        aliases: &[],
        category: CommandCategory::Session,
        sub: false,
        kind: CommandKind::Handler(handle_new),
    },
    CommandSpec {
        name: "reset",
        desc: "Clear all state",
        aliases: &[],
        category: CommandCategory::Session,
        sub: false,
        kind: CommandKind::Handler(handle_reset),
    },
    CommandSpec {
        name: "history",
        desc: "Show recent history",
        aliases: &[],
        category: CommandCategory::Session,
        sub: false,
        kind: CommandKind::Handler(handle_history),
    },
    CommandSpec {
        name: "session",
        desc: "Show session info",
        aliases: &[],
        category: CommandCategory::Session,
        sub: false,
        kind: CommandKind::Handler(handle_session),
    },
    CommandSpec {
        name: "tree",
        desc: "Open session tree dialog",
        aliases: &[],
        category: CommandCategory::Session,
        sub: true,
        kind: CommandKind::Handler(handle_tree),
    },
    CommandSpec {
        name: "share",
        desc: "Share session as GitHub gist",
        aliases: &[],
        category: CommandCategory::Session,
        sub: false,
        kind: CommandKind::Handler(handle_share),
    },
    CommandSpec {
        name: "resume",
        desc: "Resume most recent session",
        aliases: &[],
        category: CommandCategory::Session,
        sub: false,
        kind: CommandKind::Handler(handle_resume),
    },
    CommandSpec {
        name: "compact",
        desc: "Compact context",
        aliases: &[],
        category: CommandCategory::Session,
        sub: true,
        kind: CommandKind::Form {
            title: "Compact Context",
            fields: &[
                ("Keep tokens", "2000", "keep"),
                ("Focus", "optional focus keyword", "focus"),
            ],
            submit: compact_submit,
        },
    },
    CommandSpec {
        name: "fork",
        desc: "Fork session from a message",
        aliases: &[],
        category: CommandCategory::Session,
        sub: true,
        kind: CommandKind::Form {
            title: "Fork Session",
            fields: &[("Message index", "0", "index")],
            submit: fork_submit,
        },
    },
    CommandSpec {
        name: "name",
        desc: "Set session display name",
        aliases: &[],
        category: CommandCategory::Session,
        sub: true,
        kind: CommandKind::Form {
            title: "Set Session Name",
            fields: &[("Name", "session-name", "name")],
            submit: name_submit,
        },
    },
];

pub fn register(registry: &mut CommandRegistry) {
    super::spec::register_commands(registry, SESSION_COMMANDS);
}

// ── Command handlers ──────────────────────────────────────────────────────────

fn handle_sessions(_: &mut AppState, _: &str) -> CommandResult {
    match crate::session_replay::list_sessions() {
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
    state.configure_token_tracker();
    state.session.session_display_name = None;
    state.open_dialog = None;
    state.dialog_back_stack.clear();
    state.login_flow = None;
    state.permission_request = None;
    let now = crate::update::now();
    state.session.session_created_at = now;
    state.session.session_updated_at = now;
    state.messages_changed();
    CommandResult::Message("New session started".into())
}

fn handle_reset(state: &mut AppState, _: &str) -> CommandResult {
    state.reset_session();
    CommandResult::Message("State cleared.".into())
}

fn handle_history(state: &mut AppState, _: &str) -> CommandResult {
    if state.input.input_history.is_empty() {
        return CommandResult::Message("No history.".into());
    }
    let count = state.input.input_history.len();
    let entries: Vec<_> = state
        .input
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
    let tokens = session_token_count(state);
    let (user, assistant, tool) = count_messages_by_role(state);
    let info = build_session_info(state, tokens, (user, assistant, tool));
    CommandResult::Message(info)
}

fn session_token_count(state: &AppState) -> usize {
    state
        .session
        .messages
        .iter()
        .map(|m| state.agent.token_tracker.estimate_input(&m.content))
        .sum()
}

fn count_messages_by_role(state: &AppState) -> (usize, usize, usize) {
    (
        count_role(state, crate::model::Role::User),
        count_role(state, crate::model::Role::Assistant),
        count_role(state, crate::model::Role::Tool),
    )
}

fn count_role(state: &AppState, role: crate::model::Role) -> usize {
    state
        .session
        .messages
        .iter()
        .filter(|m| m.role == role)
        .count()
}

fn build_session_info(
    state: &AppState,
    tokens: usize,
    (user, assistant, tool): (usize, usize, usize),
) -> String {
    let prompt = if state.input.current_prompt.is_empty() {
        "default"
    } else {
        &state.input.current_prompt
    };
    let read_only = if state.config.read_only { "on" } else { "off" };
    let trust = project_trust_status(state);
    format!(
        "Session: {}\nMessages: {} ({} user, {} assistant, {} tool)\nTokens: {} estimated\nProvider: {}\nModel: {}\nPrompt: {}\nThinking: {}\nRead-only: {}\nTrust: {}\nCreated: {}\nUpdated: {}",
        state.session.session_display_name.as_deref().unwrap_or("unnamed"),
        state.session.messages.len(),
        user,
        assistant,
        tool,
        tokens,
        state.config.current_provider,
        state.config.current_model,
        prompt,
        state.config.thinking_level.as_str(),
        read_only,
        trust,
        crate::labels::format_timestamp(state.session.session_created_at),
        crate::labels::format_timestamp(state.session.session_updated_at),
    )
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
    CommandResult::Event(crate::event::SessionEvent::ToggleSessionTree)
}

fn handle_share(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Event(crate::event::ControlEvent::ShareSession)
}

fn handle_resume(state: &mut AppState, _: &str) -> CommandResult {
    match find_most_recent() {
        Some(name) => match crate::session_replay::load_session(&name, state) {
            Ok(_) => CommandResult::Message(format!("Loaded '{}'.", name)),
            Err(_) => CommandResult::Message("Could not load session.".into()),
        },
        None => CommandResult::Message("No sessions to resume.".into()),
    }
}

fn find_most_recent() -> Option<String> {
    let names = crate::session_replay::list_sessions().ok()?;
    let store = crate::session_store::SessionStore::default_store()?;
    let mut most_recent = None;
    let mut most_recent_time = 0.0f64;
    for name in names {
        if let Ok(meta) = load_session_metadata(&store, &name) {
            if meta.updated_at > most_recent_time {
                most_recent_time = meta.updated_at;
                most_recent = Some(name);
            }
        }
    }
    most_recent
}

fn load_session_metadata(
    store: &crate::session_store::SessionStore,
    name: &str,
) -> anyhow::Result<crate::session_index::SessionMetadata> {
    let data_dir = store.dir().parent().unwrap_or(store.dir()).to_path_buf();
    let index = crate::session_index::SessionIndex::load(&data_dir)?;
    index
        .get(name)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("not found"))
}
