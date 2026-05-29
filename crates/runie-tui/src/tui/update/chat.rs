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
        crate::tui::state::Msg::ScrollUp => { state.scroll.feed_offset = state.scroll.feed_offset.saturating_sub(1); state.scroll.user_scrolled_up = true; vec![] }
        crate::tui::state::Msg::ScrollDown => { 
            let new_offset = (state.scroll.feed_offset + 1).min(state.messages.len().saturating_sub(1));
            state.scroll.feed_offset = new_offset;
            state.scroll.user_scrolled_up = new_offset > 0;
            vec![]
        }
        crate::tui::state::Msg::ScrollPageUp => { state.scroll.feed_offset = state.scroll.feed_offset.saturating_sub(10); state.scroll.user_scrolled_up = true; vec![] }
        crate::tui::state::Msg::ScrollPageDown => { 
            let new_offset = (state.scroll.feed_offset + 10).min(state.messages.len().saturating_sub(1));
            state.scroll.feed_offset = new_offset;
            state.scroll.user_scrolled_up = new_offset > 0;
            vec![]
        }
        crate::tui::state::Msg::ClearInputConfirm => { handle_clear_input_confirm(state); vec![] }
        crate::tui::state::Msg::ClearInput => { state.textarea.select_all(); state.textarea.delete_line_by_end(); vec![] }
        crate::tui::state::Msg::ClearChat => { state.messages.clear(); vec![] }
        crate::tui::state::Msg::Paste(text) => { for c in text.chars() { state.textarea.input(ratatui_textarea::Input { key: ratatui_textarea::Key::Char(c), ctrl: false, alt: false, shift: false }); } vec![] }
        crate::tui::state::Msg::HistoryUp => { handle_history_up(state); vec![] }
        crate::tui::state::Msg::HistoryDown => { handle_history_down(state); vec![] }
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
    // Save to input history
    if !text.trim().is_empty() {
        state.input_history.push(text.clone());
        if state.input_history.len() > 100 {
            state.input_history.remove(0);
        }
    }
    // Reset history navigation state
    state.input_history_index = None;
    state.input_draft.clear();
    // FIX: If agent was running, cancel it and allow new submit
    // This prevents message dropping when user submits before AgentEnd is processed
    if state.agent_running {
        state.agent_running = false;
        state.is_thinking = false;
        state.thinking_start = None;
        state.thinking_duration = None;
        state.status_header = None;
        state.status_details = None;
        state.status_start_time = None;
        // Remove empty placeholder from previous turn if exists
        if let Some(MessageItem::Assistant { text, .. }) = state.messages.last() {
            if text.is_empty() {
                state.messages.pop();
            }
        }
    }
    // Always set agent_running = true for the NEW agent we're about to spawn
    state.agent_running = true;
    // Reset scroll state - user wants to see new output
    state.scroll.feed_offset = 0;
    state.scroll.user_scrolled_up = false;
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
    // Add placeholder assistant message immediately so user sees "Thinking..." indicator
    state.messages.push(MessageItem::Assistant {
        text: String::new(),
        model: state.current_model.clone(),
        timestamp: None,
    });
    state.is_thinking = true;
    state.thinking_start = Some(Instant::now());
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

// ─── Input History ─────────────────────────────────────────────────────────────

fn handle_history_up(state: &mut AppState) {
    if state.input_history.is_empty() {
        return;
    }

    // Save current draft if not already browsing history
    if state.input_history_index.is_none() {
        state.input_draft = state.textarea.lines().join("\n");
    }

    // Move back in history
    let new_index = state.input_history_index.map_or(
        state.input_history.len().saturating_sub(1),
        |i| i.saturating_sub(1),
    );

    if let Some(text) = state.input_history.get(new_index) {
        state.input_history_index = Some(new_index);
        state.textarea.select_all();
        state.textarea.cut();
        state.textarea.insert_str(text);
    }
}

fn handle_history_down(state: &mut AppState) {
    if let Some(index) = state.input_history_index {
        if index + 1 >= state.input_history.len() {
            // Back to draft
            state.input_history_index = None;
            state.textarea.select_all();
            state.textarea.cut();
            state.textarea.insert_str(&state.input_draft);
            state.input_draft.clear();
        } else {
            // Forward in history
            let new_index = index + 1;
            if let Some(text) = state.input_history.get(new_index) {
                state.input_history_index = Some(new_index);
                state.textarea.select_all();
                state.textarea.cut();
                state.textarea.insert_str(text);
            }
        }
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
            tool_calls: vec![],
        }),
        MessageItem::Assistant { text, .. } => Some(AgentMessage {
            role: "assistant".to_string(),
            content: vec![ContentPart::Text { text: text.clone() }],
            timestamp: 0, usage: None, stop_reason: None, error_message: None,
            tool_calls: vec![],
        }),
        MessageItem::Error { .. } => None,
        _ => None,
    }).collect()
}
