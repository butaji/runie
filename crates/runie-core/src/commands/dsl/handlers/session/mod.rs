//! Session commands.

pub mod run;

pub use run::{run_compact, run_fork, run_name};

use crate::actors::{PermissionMsg, SessionMsg};
use crate::commands::CommandResult;
use crate::model::AppState;

/// Register all session handlers with the handler registry (for YAML-based commands).
pub fn register_handlers(registry: &mut crate::commands::dsl::handlers::registry::HandlerRegistry) {
    use crate::register_handler;
    // Form handlers
    register_handler!(
        registry,
        "save",
        FormWithHandler("Save Session", &[("Name", "session-name", "name")], run::run_save)
    );
    register_handler!(
        registry,
        "load",
        FormWithHandler("Load Session", &[("Name", "session-name", "name")], run::run_load)
    );
    register_handler!(
        registry,
        "delete",
        FormWithHandler("Delete Session", &[("Name", "session-name", "name")], run::run_delete)
    );
    register_handler!(
        registry,
        "export",
        FormWithHandler("Export Session", &[("Path", "session.json", "path")], run::run_export)
    );
    register_handler!(
        registry,
        "import",
        FormWithHandler("Import Session", &[("Path", "session.json", "path")], run::run_import)
    );
    register_handler!(
        registry,
        "compact",
        FormWithHandler(
            "Compact Context",
            &[("Keep tokens", "2000", "keep"), ("Focus", "optional focus keyword", "focus")],
            run::run_compact
        )
    );
    register_handler!(
        registry,
        "fork",
        FormWithHandler("Fork Session", &[("Message index", "0", "index")], run::run_fork)
    );
    register_handler!(
        registry,
        "name",
        FormWithHandler("Set Session Name", &[("Name", "session-name", "name")], run::run_name)
    );
    // Simple handlers
    register_handler!(registry, "sessions", Handler(handle_sessions));
    register_handler!(registry, "new", Handler(handle_new));
    register_handler!(registry, "reset", Handler(handle_reset));
    register_handler!(registry, "history", Handler(handle_history));
    register_handler!(registry, "session_info", Handler(handle_session));
    register_handler!(registry, "tree", Handler(handle_tree));
    register_handler!(registry, "share", Handler(handle_share));
    register_handler!(registry, "resume", Handler(handle_resume));
}

pub fn handle_sessions(state: &mut AppState, _: &str) -> CommandResult {
    if let Some(handles) = state.actor_handles().cloned() {
        if tokio::runtime::Handle::try_current().is_ok() {
            let _ = handles.session.try_send(SessionMsg::List);
            return CommandResult::None;
        }
    }
    match crate::session::replay::list_sessions() {
        Ok(sessions) if sessions.is_empty() => {
            CommandResult::Message("No saved sessions. Use /save name to create one.".into())
        }
        Ok(sessions) => CommandResult::Message(format!("Saved sessions:\n{}", sessions.join("\n"))),
        Err(e) => CommandResult::Message(format!("Could not list sessions: {}", e)),
    }
}

pub fn handle_new(state: &mut AppState, _: &str) -> CommandResult {
    state.session_mut().messages.clear();
    state.input_mut().input.clear();
    state.input_mut().cursor_pos = 0;
    state.configure_token_tracker();
    state.session_mut().session_display_name = None;
    *state.open_dialog_mut() = None;
    state.view_mut().input_receiver = crate::model::InputReceiver::ChatInput;
    state.dialog_back_stack_mut().clear();
    *state.login_flow_mut() = None;
    if let Some(handles) = state.actor_handles() {
        let _ = handles.permission.try_send(PermissionMsg::DismissRequest);
    } else {
        *state.permission_request_mut() = None;
    }
    let now = crate::update::now();
    state.session_mut().session_created_at = now;
    state.session_mut().session_updated_at = now;
    state.messages_changed();
    state.add_system_msg("New session started".into());
    CommandResult::Event(crate::Event::ClearQueues)
}

pub fn handle_reset(state: &mut AppState, _: &str) -> CommandResult {
    state.reset_session();
    CommandResult::Message("State cleared.".into())
}

pub fn handle_history(state: &mut AppState, _: &str) -> CommandResult {
    if state.input().input_history.is_empty() {
        return CommandResult::Message("No history.".into());
    }
    let count = state.input().input_history.len();
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

pub fn handle_session(state: &mut AppState, _: &str) -> CommandResult {
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
        .map(|m| state.agent_state().token_tracker.estimate_input(&m.content()))
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
    state.session().messages.iter().filter(|m| m.role == role).count()
}

fn build_session_info(
    state: &AppState,
    tokens: usize,
    (user, assistant, tool): (usize, usize, usize),
) -> String {
    let prompt = if state.input().current_prompt.is_empty() {
        "default"
    } else {
        &state.input().current_prompt
    };
    let read_only = if state.config().read_only { "on" } else { "off" };
    let trust = project_trust_status(state);
    let session = state.session();
    format!(
        "Session: {}\nMessages: {} ({} user, {} assistant, {} tool)\nTokens: {} estimated\nProvider: {}\nModel: {}\nPrompt: {}\nThinking: {}\nRead-only: {}\nTrust: {}\nCreated: {}\nUpdated: {}",
        session.session_display_name.as_deref().unwrap_or("unnamed"),
        session.messages.len(),
        user,
        assistant,
        tool,
        tokens,
        state.config().current_provider,
        state.config().current_model,
        prompt,
        state.config().thinking_level.as_str(),
        read_only,
        trust,
        crate::labels::format_timestamp(session.session_created_at),
        crate::labels::format_timestamp(session.session_updated_at),
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

pub fn handle_tree(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Event(crate::Event::ToggleSessionTree)
}

pub fn handle_share(_: &mut AppState, _: &str) -> CommandResult {
    CommandResult::Event(crate::Event::ShareSession)
}

pub fn handle_resume(state: &mut AppState, _: &str) -> CommandResult {
    match find_most_recent() {
        Some(name) => match crate::session::replay::load_session(&name, state) {
            Ok(_) => CommandResult::Message(format!("Loaded '{}'.", name)),
            Err(_) => CommandResult::Message("Could not load session.".into()),
        },
        None => CommandResult::Message("No sessions to resume.".into()),
    }
}

fn find_most_recent() -> Option<String> {
    let names = crate::session::replay::list_sessions().ok()?;
    let store = crate::session::store::SessionStore::default_store()?;
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
    store: &crate::session::store::SessionStore,
    name: &str,
) -> anyhow::Result<crate::session::index::SessionMetadata> {
    let data_dir = store.dir().parent().unwrap_or(store.dir()).to_path_buf();
    let index = crate::session::index::SessionIndex::load(&data_dir)?;
    index.get(name).cloned().ok_or_else(|| anyhow::anyhow!("not found"))
}
