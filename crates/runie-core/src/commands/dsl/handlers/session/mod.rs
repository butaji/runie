//! Session commands.
//!
//! Session IO (save/load/delete/import/export) lives in `io.rs`.
//! Session run helpers (name/fork/compact) live in `run.rs`.

pub mod io;
pub mod run;

pub use run::{run_compact, run_fork, run_name};

use crate::commands::{CommandCategory, CommandRegistry, CommandResult};
use crate::dialog::PanelStack;
use crate::model::AppState;

use super::spec::{CommandKind, CommandSpec};

/// Build the /load form panel (used to re-open when submitted empty).
pub fn build_load_form() -> CommandResult {
    let stack = build_form_stack(
        "load",
        "Load Session",
        &[("Name", "session-name", "name")],
        load_submit,
    );
    CommandResult::OpenPanelStack(stack)
}

/// Build the /delete form panel.
pub fn build_delete_form() -> CommandResult {
    let stack = build_form_stack(
        "delete",
        "Delete Session",
        &[("Name", "session-name", "name")],
        delete_submit,
    );
    CommandResult::OpenPanelStack(stack)
}

/// Build the /import form panel.
pub fn build_import_form() -> CommandResult {
    let stack = build_form_stack(
        "import",
        "Import Session",
        &[("Path", "session.json", "path")],
        import_submit,
    );
    CommandResult::OpenPanelStack(stack)
}

/// Build the /export form panel.
pub fn build_export_form() -> CommandResult {
    let stack = build_form_stack(
        "export",
        "Export Session",
        &[("Path", "session.json", "path")],
        export_submit,
    );
    CommandResult::OpenPanelStack(stack)
}

fn build_form_stack(
    id: &str,
    title: &str,
    fields: &[(&str, &str, &str)],
    submit: super::spec::FormSubmitFn,
) -> PanelStack {
    use crate::dialog::dsl::form;
    let mut builder = form(id, title);
    for (label, placeholder, key) in fields {
        builder = builder.field(*label, *placeholder, *key);
    }
    builder.on_submit(submit).into_stack()
}

fn save_submit(values: &std::collections::HashMap<String, String>) -> crate::Event {
    crate::event::CommandEvent::RunSaveCommand {
        name: values.get("name").cloned().unwrap_or_default(),
    }
}
fn load_submit(values: &std::collections::HashMap<String, String>) -> crate::Event {
    crate::event::CommandEvent::RunLoadCommand {
        name: values.get("name").cloned().unwrap_or_default(),
    }
}
fn delete_submit(values: &std::collections::HashMap<String, String>) -> crate::Event {
    crate::event::CommandEvent::RunDeleteCommand {
        name: values.get("name").cloned().unwrap_or_default(),
    }
}
fn export_submit(values: &std::collections::HashMap<String, String>) -> crate::Event {
    crate::event::CommandEvent::RunExportCommand {
        path: values.get("path").cloned().unwrap_or_default(),
    }
}
fn import_submit(values: &std::collections::HashMap<String, String>) -> crate::Event {
    crate::event::CommandEvent::RunImportCommand {
        path: values.get("path").cloned().unwrap_or_default(),
    }
}
fn compact_submit(values: &std::collections::HashMap<String, String>) -> crate::Event {
    crate::event::CommandEvent::RunCompactCommand {
        keep: values.get("keep").cloned().unwrap_or_default(),
        focus: values.get("focus").cloned().unwrap_or_default(),
    }
}
fn fork_submit(values: &std::collections::HashMap<String, String>) -> crate::Event {
    crate::event::CommandEvent::RunForkCommand {
        message_index: values.get("index").cloned().unwrap_or_default(),
    }
}
fn name_submit(values: &std::collections::HashMap<String, String>) -> crate::Event {
    crate::event::CommandEvent::RunNameCommand {
        name: values.get("name").cloned().unwrap_or_default(),
    }
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
    state.configure_token_tracker();
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
    CommandResult::Message(format!("History ({} total):\n{}", count, entries.join("\n")))
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
    state.session.messages.iter().filter(|m| m.role == role).count()
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
        Some(name) => match crate::session::load(&name) {
            Ok(session) => {
                state.session.messages = session.messages;
                state.config.current_provider = session.provider;
                state.config.current_model = session.model;
                state.config.theme_name = session.theme_name;
                state.config.thinking_level = session.thinking_level;
                state.config.read_only = session.read_only;
                state.session.session_display_name =
                    session.display_name.or(Some(session.name));
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
