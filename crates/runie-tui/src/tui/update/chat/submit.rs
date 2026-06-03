use crate::components::MessageItem;
use crate::tui::state::{AppState, ThinkingState};
use super::ChatCmd;
use super::to_agent_messages;
use crate::tui::update::misc::current_timestamp;
use std::time::Instant;

pub fn handle_slash_submit(state: &mut AppState, text: &str) -> Vec<ChatCmd> {
    if let Some(cmd) = runie_core::slash_command::parse_slash_command(text) {
        let commands = crate::tui::update::slash::handle_slash(state, cmd);
        state.textarea.select_all();
        state.textarea.delete_line_by_end();
        commands.into_iter().map(ChatCmd::Ui).collect()
    } else {
        state.messages.push(MessageItem::Error {
            message: format!("Unknown command: {}. Type /help for available commands.", text),
            recoverable: true,
        });
        state.textarea.select_all();
        state.textarea.delete_line_by_end();
        vec![]
    }
}

pub fn handle_shell_submit(state: &mut AppState, text: &str) -> Vec<ChatCmd> {
    let cmd_text = text.strip_prefix('!').unwrap_or("").trim();
    if cmd_text.is_empty() {
        state.input_right_info = "Type a command after !".to_string();
        return vec![];
    }
    let output = execute_shell_command(cmd_text);
    state.messages.push(MessageItem::System {
        text: format!("$ {}\n{}", cmd_text, output),
    });
    state.textarea.select_all();
    state.textarea.delete_line_by_end();
    vec![]
}

pub fn handle_submit(state: &mut AppState) -> Vec<ChatCmd> {
    let text = state.textarea.lines().join("\n");
    tracing::debug!(
        "handle_submit called: agent_running={}, text_len={}, first_msg={:?}",
        state.agent_running,
        text.len(),
        state.messages.last().map(|m| format!("{:?}", m).chars().take(50).collect::<String>())
    );
    if text.chars().all(|c| c.is_whitespace()) {
        state.input_right_info = "Type a message first".to_string();
        return vec![];
    }
    if text.starts_with('/') { return handle_slash_submit(state, &text); }
    if text.starts_with('!') { return handle_shell_submit(state, &text); }
    if state.agent_running {
        tracing::debug!("handle_submit: agent_running=true, calling cancel_running_agent");
        cancel_running_agent(state);
    }
    // Guard: ensure agent_running is false before proceeding with new submission
    // This handles edge cases where cancellation didn't fully clear state
    if state.agent_running {
        tracing::debug!("handle_submit: agent_running still true after cancel, blocking");
        state.input_right_info = "Agent still running... Ctrl+C to stop".to_string();
        return vec![];
    }
    // Clear session_starting if present - stale indicator from prior session
    // should not block subsequent turns
    state.session_starting = None;
    if should_defer_submit(state) { return vec![]; }
    prepare_agent_messages(state, &text);
    finalize_submit(state, text)
}

fn prepare_agent_messages(state: &mut AppState, text: &str) {
    save_to_history(state, text);
    reset_history_nav(state);
    reset_scroll(state);
    state.agent_start_time = Some(Instant::now());
}

fn save_to_history(state: &mut AppState, text: &str) {
    if !text.trim().is_empty() {
        state.input_history.push(text.to_string());
        if state.input_history.len() > 100 {
            state.input_history.remove(0);
        }
    }
}

fn reset_history_nav(state: &mut AppState) {
    state.input_history_index = None;
    state.input_draft.clear();
}

fn cancel_running_agent(state: &mut AppState) {
    if !state.agent_running { return; }
    state.agent_running = false;
    state.thinking = None;
    state.status_header = None;
    state.status_details = None;
    state.status_start_time = None;
    if let Some(MessageItem::Assistant { text, .. }) = state.messages.last() {
        if text.is_empty() { state.messages.pop(); }
    }
}

fn reset_scroll(state: &mut AppState) {
    state.scroll.feed_offset = 0;
    state.scroll.user_scrolled_up = false;
}

fn should_defer_submit(state: &mut AppState) -> bool {
    if let Some(ref onboarding) = state.onboarding {
        if onboarding.is_fetching_models {
            state.input_right_info = "Loading models... retry after".to_string();
            return true;
        }
    }
    // session_starting should not block subsequent turns - clear it if present
    // It only indicates the transition from home screen to chat, not a blocking state
    if state.session_starting.is_some() {
        state.session_starting = None;
    }
    false
}

fn execute_shell_command(cmd: &str) -> String {
    let output = std::process::Command::new("sh").arg("-c").arg(cmd).output();
    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            if out.status.success() {
                if stdout.is_empty() && stderr.is_empty() { "(no output)".to_string() }
                else { format!("{}{}", stdout, stderr) }
            } else {
                format!("Error (exit {}): {}{}", out.status.code().unwrap_or(-1), stdout, stderr)
            }
        }
        Err(e) => format!("Failed to execute: {}", e),
    }
}

fn finalize_submit(state: &mut AppState, text: String) -> Vec<ChatCmd> {
    let model_missing = state.current_model.as_deref().map_or(true, |s| s.is_empty()) && state.onboarding.is_none();
    if model_missing {
        add_user_message_only(state, &text);
        state.messages.push(MessageItem::Error {
            message: "No model configured. Press Ctrl+O or type /onboard to set up a model.".to_string(),
            recoverable: true,
        });
        state.input_right_info = String::new();
        return vec![];
    }
    tracing::debug!("finalize_submit: setting agent_running = true");
    state.agent_running = true;
    state.session_starting = None;
    add_user_and_placeholder(state, &text);
    let agent_messages = to_agent_messages(&state.messages);
    tracing::debug!(
        "finalize_submit: returning SpawnAgent with {} messages, {} user, {} assistant",
        agent_messages.len(),
        agent_messages.iter().filter(|m| m.role == "user").count(),
        agent_messages.iter().filter(|m| m.role == "assistant").count()
    );
    vec![ChatCmd::SpawnAgent { messages: agent_messages }]
}

pub fn handle_interject(state: &mut AppState) -> Vec<ChatCmd> {
    let text = state.textarea.lines().join("\n");
    if text.chars().all(|c| c.is_whitespace()) {
        state.input_right_info = "Type a message to interject".to_string();
        return vec![];
    }
    state.input_right_info = String::new();
    state.textarea.select_all();
    state.textarea.delete_line_by_end();
    state.messages.push(MessageItem::User {
        text: text.to_string(),
        model: Some("You".to_string()),
        timestamp: current_timestamp(),
    });
    vec![]
}

fn add_user_message_only(state: &mut AppState, text: &str) {
    state.messages.push(MessageItem::User {
        text: text.to_string(),
        model: Some("You".to_string()),
        timestamp: current_timestamp(),
    });
    state.textarea.select_all();
    state.textarea.delete_line_by_end();
}

fn add_user_and_placeholder(state: &mut AppState, text: &str) {
    state.messages.push(MessageItem::User {
        text: text.to_string(),
        model: Some("You".to_string()),
        timestamp: current_timestamp(),
    });
    state.messages.push(MessageItem::Assistant {
        text: String::new(),
        model: state.current_model.clone(),
        timestamp: current_timestamp(),
        expanded: true,
        thought_duration: None,
        turn_duration: None,
    });
    state.thinking = Some(ThinkingState {
        start: Some(Instant::now()),
        text: String::new(),
        accrued_duration: None,
    });
    state.textarea.select_all();
    state.textarea.delete_line_by_end();
}
