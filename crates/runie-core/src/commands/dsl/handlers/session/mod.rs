//! Session commands.

pub mod run;

pub use run::{run_compact, run_fork, run_name};

use crate::actors::{PermissionMsg, SessionMsg};
use crate::commands::CommandResult;
use crate::commands::dsl::handlers::NamedHandler;
use crate::model::AppState;

// ── Form field defaults ──────────────────────────────────────────────────────

/// Default token count for the `/compact` command form.
pub const COMPACT_DEFAULT_KEEP_TOKENS: &str = "2000";
/// Default message index for the `/fork` command form.
pub const FORK_DEFAULT_MESSAGE_INDEX: &str = "0";

/// Register all session handlers with the handler registry (for YAML-based commands).
pub fn register_handlers(registry: &mut crate::commands::dsl::handlers::registry::HandlerRegistry) {
    register_session_form_handlers(registry);
    register_session_simple_handlers(registry);
}

/// Register form-based session commands.
fn register_session_form_handlers(
    registry: &mut crate::commands::dsl::handlers::registry::HandlerRegistry,
) {
    registry.register("save", NamedHandler::FormWithHandler {
        title: "Save Session",
        fields: &[("Name", "session-name", "name")],
        handler: run::run_save,
    });
    registry.register("load", NamedHandler::FormWithHandler {
        title: "Load Session",
        fields: &[("Name", "session-name", "name")],
        handler: run::run_load,
    });
    registry.register("delete", NamedHandler::FormWithHandler {
        title: "Delete Session",
        fields: &[("Name", "session-name", "name")],
        handler: run::run_delete,
    });
    registry.register("export", NamedHandler::FormWithHandler {
        title: "Export Session",
        fields: &[("Path", "session.json", "path")],
        handler: run::run_export,
    });
    registry.register("import", NamedHandler::FormWithHandler {
        title: "Import Session",
        fields: &[("Path", "session.json", "path")],
        handler: run::run_import,
    });
    registry.register("compact", NamedHandler::FormWithHandler {
        title: "Compact Context",
        fields: &[
            ("Keep tokens", COMPACT_DEFAULT_KEEP_TOKENS, "keep"),
            ("Focus", "optional focus keyword", "focus"),
        ],
        handler: run::run_compact,
    });
    registry.register("fork", NamedHandler::FormWithHandler {
        title: "Fork Session",
        fields: &[("Message index", FORK_DEFAULT_MESSAGE_INDEX, "index")],
        handler: run::run_fork,
    });
    registry.register("name", NamedHandler::FormWithHandler {
        title: "Set Session Name",
        fields: &[("Name", "session-name", "name")],
        handler: run::run_name,
    });
}

/// Register simple session commands.
fn register_session_simple_handlers(
    registry: &mut crate::commands::dsl::handlers::registry::HandlerRegistry,
) {
    registry.register("sessions", NamedHandler::Handler(handle_sessions));
    registry.register("new", NamedHandler::Handler(handle_new));
    registry.register("reset", NamedHandler::Handler(handle_reset));
    registry.register("history", NamedHandler::Handler(handle_history));
    registry.register("session_info", NamedHandler::Handler(handle_session));
    registry.register("tree", NamedHandler::Handler(handle_tree));
    registry.register("share", NamedHandler::Handler(handle_share));
    registry.register("resume", NamedHandler::Handler(handle_resume));
}

pub fn handle_sessions(state: &mut AppState, _: &str) -> CommandResult {
    // Route through SessionActor in production; fall back to direct listing in tests.
    if let Some(handles) = state.actor_handles() {
        let _ = handles.session.try_send(SessionMsg::List);
        return CommandResult::None;
    }
    match crate::session::replay::list_sessions() {
        Ok(sessions) if sessions.is_empty() => {
            CommandResult::Message(crate::ui_strings::session::NO_SAVED_SESSIONS.into())
        }
        Ok(sessions) => CommandResult::Message(crate::ui_strings::session::saved_sessions(&sessions)),
        Err(e) => CommandResult::Message(crate::ui_strings::session::session_list_error(&e.to_string())),
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
        // Abort any in-flight turn so the agent stops streaming and agent_running is cleared.
        let _ = handles.turn.try_send(crate::actors::TurnMsg::AbortTurn);
    } else {
        *state.permission_request_mut() = None;
    }
    let now = crate::update::now();
    state.session_mut().session_created_at = now;
    state.session_mut().session_updated_at = now;
    state.messages_changed();
    state.add_system_msg(crate::ui_strings::session::NEW_SESSION_STARTED.into());
    // Emit Abort so UiActor clears agent_running; ClearQueues clears the queue.
    CommandResult::Events(vec![crate::Event::Abort, crate::Event::ClearQueues])
}

pub fn handle_reset(state: &mut AppState, _: &str) -> CommandResult {
    state.reset_session();
    CommandResult::Message(crate::ui_strings::session::STATE_CLEARED.into())
}

pub fn handle_history(state: &mut AppState, _: &str) -> CommandResult {
    let messages = state.session().messages();
    if messages.is_empty() {
        return CommandResult::Message(crate::ui_strings::session::NO_HISTORY.into());
    }

    let role_label = |role: &crate::message::Role| match role {
        crate::message::Role::User => "You",
        crate::message::Role::Assistant => "Assistant",
        crate::message::Role::Tool => "Tool",
        crate::message::Role::System => "System",
        crate::message::Role::Thought => "Thought",
        crate::message::Role::TurnComplete => "TurnComplete",
    };

    let total = messages.len();
    let entries: Vec<_> = messages
        .iter()
        .rev()
        .take(10)
        .enumerate()
        .map(|(i, msg)| {
            let label = role_label(&msg.role);
            let content = msg.content();
            let preview = if content.len() > 80 {
                format!("{}...", &content[..80])
            } else {
                content
            };
            let content_lines: Vec<&str> = preview.lines().take(3).collect();
            let content_str = content_lines.join(" ");
            format!("{}: [{}] {}", i + 1, label, content_str)
        })
        .collect();

    CommandResult::Message(crate::ui_strings::session::history(total, &entries))
}

pub fn handle_session(state: &mut AppState, _: &str) -> CommandResult {
    let tokens = session_token_count(state);
    let (user, assistant, tool) = count_messages_by_role(state);
    let info = build_session_info(state, tokens, (user, assistant, tool));
    CommandResult::Message(crate::ui_strings::session::session_info(&info))
}

fn session_token_count(state: &AppState) -> usize {
    state
        .session
        .messages
        .iter()
        .map(|m| {
            state
                .agent_state()
                .token_tracker
                .estimate_input(&m.content())
        })
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
        .session()
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
    let prompt = if state.input().current_prompt.is_empty() {
        "default"
    } else {
        &state.input().current_prompt
    };
    let read_only = if state.config().read_only {
        "on"
    } else {
        "off"
    };
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

fn project_trust_status(state: &AppState) -> &'static str {
    let cwd = std::env::current_dir().unwrap_or_default();
    let cwd_utf8 = camino::Utf8PathBuf::from_path_buf(cwd).unwrap_or_else(|_| camino::Utf8PathBuf::from("."));
    // Read from the AppState projection populated via Event::TrustLoaded.
    // In test mode (no actor handles), trust_decisions may be empty;
    // fall back to the default trust status.
    match state.trust_decisions().get(&cwd_utf8) {
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
    use crate::ui_strings::session as s;
    if let Some(handles) = state.actor_handles().cloned() {
        let _ = handles.session.try_send(SessionMsg::ResumeMostRecent);
        return CommandResult::None;
    }
    // Fallback for test mode without actor handles.
    let store = match crate::session::store::SessionStore::default_store() {
        Some(s) => s,
        None => return CommandResult::Message(s::NO_SESSION_STORE.into()),
    };
    match find_most_recent_from_store(&store) {
        Some(name) => match crate::session::replay::load_session_from_store(&name, state, &store) {
            Ok(_) => CommandResult::Message(s::session_resumed(&name)),
            Err(_) => CommandResult::Message(s::RESUME_LOAD_FAILED.into()),
        },
        None => CommandResult::Message(s::NO_SESSIONS_TO_RESUME.into()),
    }
}

fn find_most_recent_from_store(
    store: &crate::session::store::SessionStore,
) -> Option<String> {
    let names = store.list().ok()?;
    let mut most_recent = None;
    let mut most_recent_time = 0.0f64;
    for name in names {
        if let Ok(Some(meta)) = store.load_metadata(&name) {
            if meta.updated_at > most_recent_time {
                most_recent_time = meta.updated_at;
                most_recent = Some(name);
            }
        }
    }
    most_recent
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::AppState;
    use crate::trust::TrustDecision;
    use indexmap::indexmap;

    #[test]
    fn project_trust_status_reads_from_trust_decisions() {
        let mut state = AppState::default();
        // Pre-populate trust decisions (as Event::TrustLoaded would).
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/tmp"));
        let cwd_utf8 = camino::Utf8PathBuf::from_path_buf(cwd)
            .unwrap_or_else(|_| camino::Utf8PathBuf::from("/tmp"));
        state.trust_decisions = indexmap! { cwd_utf8.clone() => TrustDecision::Trusted };

        let status = project_trust_status(&state);
        assert_eq!(status, "trusted");
    }

    #[test]
    fn project_trust_status_default_when_no_decision() {
        let state = AppState::default();
        let status = project_trust_status(&state);
        assert_eq!(status, "default");
    }

    #[test]
    fn project_trust_status_untrusted() {
        let mut state = AppState::default();
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/tmp"));
        let cwd_utf8 = camino::Utf8PathBuf::from_path_buf(cwd)
            .unwrap_or_else(|_| camino::Utf8PathBuf::from("/tmp"));
        state.trust_decisions = indexmap! { cwd_utf8 => TrustDecision::Untrusted };

        let status = project_trust_status(&state);
        assert_eq!(status, "untrusted");
    }
}
