//! Turn lifecycle handlers.

use crate::components::MessageItem;
use crate::messages::MessageRegistry;
use runie_agent::ContentPart;

use super::super::AgentCmd;
use super::{extract_text_content, on_agent_end, AppState};

/// Handle turn end - add separator with runtime metrics
pub fn on_turn_end(state: &mut AppState, turn_duration_ms: Option<u64>) {
    let turn_duration_ms = turn_duration_ms.filter(|&ms| ms > 0);
    let elapsed = calc_turn_elapsed(state, turn_duration_ms);
    let tool_calls = count_tool_calls(&state.messages);
    
    state.last_turn_duration_secs = Some(elapsed);
    state.last_turn_tokens = Some(state.session_token_usage.total_tokens);
    state.last_turn_tool_calls = Some(tool_calls);
    state.turn_success = Some(true);

    let turn_dur_f32 = calc_turn_duration_f32(state, turn_duration_ms, elapsed);
    if let Some(MessageItem::Assistant { turn_duration: ref mut td, .. }) = state.messages.last_mut() {
        *td = Some(turn_dur_f32);
    }

    let turn_dur_secs = turn_dur_f32 as u64;
    state.messages.push(MessageItem::Separator {
        elapsed_secs: turn_dur_secs,
        tool_calls,
        tokens_used: Some(state.session_token_usage.total_tokens),
    });
}

fn calc_turn_elapsed(state: &AppState, turn_duration_ms: Option<u64>) -> u64 {
    if let Some(start_time) = state.agent_start_time {
        start_time.elapsed().as_secs().max(state.last_turn_duration_secs.unwrap_or(0))
    } else if let Some(override_ms) = turn_duration_ms {
        (override_ms / 1000).max(1)
    } else {
        state.replay_turn_duration_secs.map(|s| s as u64).unwrap_or(1)
    }
}

fn calc_turn_duration_f32(state: &AppState, turn_duration_ms: Option<u64>, elapsed: u64) -> f32 {
    if let Some(ms) = turn_duration_ms {
        ms as f32 / 1000.0
    } else if state.agent_start_time.is_some() {
        elapsed as f32
    } else {
        state.replay_turn_duration_secs.unwrap_or(elapsed as f32)
    }
}

fn count_tool_calls(messages: &[MessageItem]) -> usize {
    messages.iter().filter(|m| matches!(m, MessageItem::ToolCall { .. })).count()
}

pub fn update_last_assistant(state: &mut AppState, content: &[ContentPart]) {
    if let Some(MessageItem::Assistant { ref mut text, .. }) = state.messages.last_mut() {
        let new_content = extract_text_content(content);
        if !new_content.is_empty() && !text.ends_with(&new_content) {
            let needs_newline = !text.is_empty() && !text.ends_with('\n');
            if needs_newline { text.push('\n'); }
            text.push_str(&new_content);
        }
    }
}
