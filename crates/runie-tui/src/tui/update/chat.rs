//! Chat domain update functions.
//! Handles: messages, textarea input, scroll, submit, clear.

use crate::components::MessageItem;
use crate::tui::state::AppState;
use crate::tui::key_to_textarea_input;
use std::time::Instant;

/// Chat-specific commands returned by update functions.
#[derive(Debug, Clone)]
pub enum ChatCmd {
    SpawnAgent { messages: Vec<runie_agent::AgentMessage> },
}

impl From<ChatCmd> for crate::tui::state::Cmd {
    fn from(cmd: ChatCmd) -> Self {
        match cmd {
            ChatCmd::SpawnAgent { messages } => crate::tui::state::Cmd::SpawnAgent { messages },
        }
    }
}

/// Update chat domain: messages, textarea, scroll, submit, clear.
pub fn update(state: &mut AppState, msg: crate::tui::state::Msg) -> Vec<ChatCmd> {
    match msg {
        crate::tui::state::Msg::Submit => handle_submit(state),
        crate::tui::state::Msg::TextareaKey(key) => { state.textarea.input(key_to_textarea_input(key)); vec![] }
        crate::tui::state::Msg::InsertNewline => { state.textarea.insert_newline(); vec![] }
        crate::tui::state::Msg::ScrollUp => { state.scroll.feed_offset = state.scroll.feed_offset.saturating_sub(1); vec![] }
        crate::tui::state::Msg::ScrollDown => { state.scroll.feed_offset = (state.scroll.feed_offset + 1).min(state.messages.len().saturating_sub(1)); vec![] }
        crate::tui::state::Msg::ScrollPageUp => { state.scroll.feed_offset = state.scroll.feed_offset.saturating_sub(10); vec![] }
        crate::tui::state::Msg::ScrollPageDown => { state.scroll.feed_offset = (state.scroll.feed_offset + 10).min(state.messages.len().saturating_sub(1)); vec![] }
        crate::tui::state::Msg::ClearInputConfirm => { handle_clear_input_confirm(state); vec![] }
        crate::tui::state::Msg::ClearInput => { state.textarea.select_all(); state.textarea.delete_line_by_end(); vec![] }
        crate::tui::state::Msg::ClearChat => { state.messages.clear(); vec![] }
        crate::tui::state::Msg::Paste(text) => { for c in text.chars() { state.textarea.input(ratatui_textarea::Input { key: ratatui_textarea::Key::Char(c), ctrl: false, alt: false, shift: false }); } vec![] }
        _ => vec![],
    }
}

// ─── Submit ─────────────────────────────────────────────────────────────────────

fn handle_submit(state: &mut AppState) -> Vec<ChatCmd> {
    let text = state.textarea.lines().join("\n");
    if text.is_empty() {
        state.input_right_info = "Type a message first".to_string();
        return vec![];
    }
    // BUG-10 FIX: Set agent_running immediately to prevent race condition
    // where rapid submissions could spawn multiple agents
    if state.agent_running {
        state.input_right_info = "Agent running (blocked)... Ctrl+C to stop".to_string();
        return vec![];
    }
    state.agent_running = true;
    // P0-AGENT-TIMEOUT: Track when agent started for watchdog timeout
    state.agent_start_time = Some(Instant::now());
    if let Some(ref onboarding) = state.onboarding {
        if onboarding.is_fetching_models {
            state.input_right_info = "Loading models...".to_string();
            return vec![];
        }
    }
    let model_missing = state.current_model.as_deref().map_or(true, |s| s.is_empty()) && state.onboarding.is_none();
    state.messages.push(MessageItem::User { text: text.clone(), model: Some("You".to_string()), timestamp: None });
    state.textarea.select_all();
    state.textarea.delete_line_by_end();
    if model_missing {
        state.messages.push(MessageItem::System { text: "No model configured. Press Ctrl+O or type /onboard to set up a model.".to_string() });
        return vec![];
    }
    let agent_messages = to_agent_messages(&state.messages);
    vec![ChatCmd::SpawnAgent { messages: agent_messages }]
}

// ─── Clear Input Confirm ────────────────────────────────────────────────────────

fn handle_clear_input_confirm(state: &mut AppState) {
    if state.clear_input_confirm.wants_clear() {
        state.textarea.select_all();
        state.textarea.delete_line_by_end();
        state.input_right_info = String::new();
    } else {
        state.input_right_info = "Ctrl+C again to clear text".to_string();
    }
}

// ─── Message Conversion ────────────────────────────────────────────────────────

fn to_agent_messages(items: &[MessageItem]) -> Vec<runie_agent::AgentMessage> {
    use runie_agent::{AgentMessage, ContentPart};
    items.iter().filter_map(|item| match item {
        MessageItem::User { text, .. } => Some(AgentMessage {
            role: "user".to_string(),
            content: vec![ContentPart::Text { text: text.clone() }],
            timestamp: 0, usage: None, stop_reason: None, error_message: None,
        }),
        MessageItem::Assistant { text, .. } => Some(AgentMessage {
            role: "assistant".to_string(),
            content: vec![ContentPart::Text { text: text.clone() }],
            timestamp: 0, usage: None, stop_reason: None, error_message: None,
        }),
        MessageItem::Error { .. } => None,
        _ => None,
    }).collect()
}
