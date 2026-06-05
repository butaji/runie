//! Message event handlers.

use crate::components::MessageItem;
use crate::messages::MessageRegistry;
use crate::tui::state::{AppState, ThinkingState};
use runie_agent::events::AgentMessage;

use super::{extract_text_content, thinking::ensure_thinking_placeholder, AgentCmd};

fn current_timestamp() -> Option<String> {
    use chrono::Local;
    if let Ok(mock) = std::env::var("RUNIE_MOCK_TIMESTAMP") {
        if !mock.is_empty() { return Some(mock); }
    }
    Some(Local::now().format("%-I:%M %p").to_string())
}

pub fn on_message_start(state: &mut AppState, _message: AgentMessage) {
    tracing::debug!("on_message_start: setting agent_running = true");
    state.agent_running = true;
    state.session_starting = None;
    state.status_header = Some(MessageRegistry::status_thinking().to_string());
    state.status_start_time = Some(std::time::Instant::now());
    state.status_details = Some(MessageRegistry::format_elapsed(0));
    state.thinking = Some(ThinkingState {
        start: Some(std::time::Instant::now()),
        text: String::new(),
        accrued_duration: None,
    });
    state.last_turn_duration_secs = None;
    state.last_turn_tokens = None;
    state.last_turn_tool_calls = None;
    
    if !state.scroll.user_scrolled_up { state.scroll.feed_offset = 0; }
    
    let has_placeholder = state.messages.last()
        .map(|m| matches!(m, MessageItem::Assistant { text, .. } if text.is_empty()))
        .unwrap_or(false);
    
    if !has_placeholder {
        state.messages.push(MessageItem::Assistant {
            text: String::new(),
            model: state.current_model.clone(),
            timestamp: current_timestamp(),
            expanded: true,
            thought_duration: state.pending_thought_duration.take(),
            turn_duration: None,
        });
    }
}

pub fn on_message(state: &mut AppState, role: &str, content: &str) {
    match role {
        "user" => state.messages.push(MessageItem::User {
            text: content.to_string(),
            model: Some("You".to_string()),
            timestamp: current_timestamp(),
        }),
        "assistant" => state.messages.push(MessageItem::Assistant {
            text: content.to_string(),
            model: state.current_model.clone(),
            timestamp: current_timestamp(),
            expanded: true,
            thought_duration: None,
            turn_duration: None,
        }),
        "system" => {
            if !content.starts_with("Using ") && !content.starts_with("Mock mode") {
                state.messages.push(MessageItem::System { text: content.to_string() });
            }
        }
        _ => state.messages.push(MessageItem::System { text: content.to_string() }),
    }
}

pub fn on_message_update(state: &mut AppState, message: AgentMessage, delta: &str, replace: bool) {
    if !state.scroll.user_scrolled_up { state.scroll.feed_offset = 0; }
    ensure_thinking_placeholder(state, &message.content);
    
    if replace {
        if let Some(MessageItem::Assistant { ref mut text, .. }) = state.messages.last_mut() {
            *text = delta.to_string();
        }
    } else if !delta.is_empty() {
        if let Some(MessageItem::Assistant { ref mut text, .. }) = state.messages.last_mut() {
            text.push_str(delta);
        }
    }
}

pub fn on_message_end(state: &mut AppState, message: AgentMessage) {
    state.agent_running = false;
    state.status_header = None;
    state.status_details = None;
    state.status_start_time = None;
    
    let duration = calc_thinking_duration(&mut state.thinking);
    state.thinking = None;

    if !state.scroll.user_scrolled_up { state.scroll.feed_offset = 0; }

    if let Some(duration) = duration {
        let secs = duration.as_secs_f32();
        if secs > 0.5 {
            if let Some(MessageItem::Assistant { thought_duration: ref mut td, .. }) = state.messages.last_mut() {
                *td = Some(secs);
            }
        }
    }

    super::events_turn::update_last_assistant(state, &message.content);
}

fn calc_thinking_duration(thinking: &mut Option<ThinkingState>) -> Option<std::time::Duration> {
    if let Some(ref mut t) = thinking {
        let current = t.start.take().map(|start| start.elapsed());
        current.map(|d| t.accrued_duration.map(|acc| d + acc).unwrap_or(d))
    } else {
        None
    }
}
