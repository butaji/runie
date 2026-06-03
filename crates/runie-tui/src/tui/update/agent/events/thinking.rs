//! Thinking event handlers.

use crate::components::MessageItem;
use crate::tui::state::{AppState, ThinkingState};

use super::current_timestamp;

pub(super) fn on_thinking_start(state: &mut AppState, turn: usize) {
    tracing::debug!("on_thinking_start: turn={}", turn);
    state.thinking = Some(ThinkingState {
        start: Some(std::time::Instant::now()),
        text: String::new(),
        accrued_duration: None,
    });
    // Reset turn info for each new turn
    state.last_turn_duration_secs = None;
    state.last_turn_tokens = None;
    state.last_turn_tool_calls = None;
}

pub(super) fn on_thinking_update(state: &mut AppState, text: String) {
    if let Some(ref mut thinking) = state.thinking {
        if let Some(start) = thinking.start.take() {
            thinking.accrued_duration = Some(thinking.accrued_duration.unwrap_or_default() + start.elapsed());
            thinking.start = Some(std::time::Instant::now());
        }
        thinking.text.push_str(&text);
    }

    // Append think content to the last assistant message.
    // This ensures extract_think_blocks() in render_assistant_msg can find and render it.
    // Previously we pushed a separate MessageItem::Thought but those get filtered out
    // in Feed::from conversion (convert.rs returns Err for Thought items).
    append_think_content_to_last_assistant(state, &text, None);
}

pub(super) fn on_thinking_end(state: &mut AppState, duration_ms: u64) {
    let duration = std::time::Duration::from_millis(duration_ms);

    // Record final thinking duration
    if let Some(ref mut thinking) = state.thinking {
        if let Some(start) = thinking.start.take() {
            thinking.accrued_duration = Some(thinking.accrued_duration.unwrap_or_default() + start.elapsed());
        }
    }

    let text = state.thinking.as_ref().map(|t| t.text.clone()).unwrap_or_default();
    // Use accrued_duration if available, otherwise use duration_ms passed in
    let final_duration = state.thinking.as_ref()
        .and_then(|t| t.accrued_duration)
        .unwrap_or(duration);

    // Append think content tags to the last assistant message.
    // This ensures extract_think_blocks() in render_assistant_msg can find and render it.
    // Previously we pushed a separate MessageItem::Thought but those get filtered out
    // in Feed::from conversion (convert.rs returns Err for Thought items).
    append_think_content_to_last_assistant(state, &text, Some(&final_duration));
    state.thinking = None;
}

fn append_think_content_to_last_assistant(
    state: &mut AppState,
    text: &str,
    duration: Option<&std::time::Duration>,
) {
    let Some(duration) = duration else { return };

    let secs = duration.as_secs_f32();
    if secs > 0.5 || !text.is_empty() {
        if !text.is_empty() {
            let think_with_tags = format!("\n<think>\n{}\n
</think>

", text);
            if let Some(MessageItem::Assistant { text: ref mut assistant_text, .. }) = state.messages.last_mut() {
                assistant_text.push_str(&think_with_tags);
            }
        }
        if secs > 0.5 {
            if let Some(MessageItem::Assistant { thought_duration: ref mut td, .. }) = state.messages.last_mut() {
                *td = Some(secs);
            }
        }
    }
}

/// Ensure a thinking placeholder exists when receiving thinking content.
/// This is called from message update handlers to create an assistant
/// placeholder if one doesn't exist yet.
pub(super) fn ensure_thinking_placeholder(state: &mut AppState, content: &[runie_agent::events::ContentPart]) {
    let is_thinking = content.iter().any(|part| {
        if let runie_agent::events::ContentPart::Text { text } = part {
            text.trim_start().starts_with("[thinking:")
        } else {
            false
        }
    });

    if !is_thinking {
        return;
    }

    let has_no_assistant = !state.messages.iter().any(|m| matches!(m, MessageItem::Assistant { .. }));
    let last_has_thinking = state.messages.last()
        .map(|m| matches!(m, MessageItem::Assistant { text, .. } if text.trim_start().starts_with("[thinking:")))
        .unwrap_or(false);

    if has_no_assistant || last_has_thinking {
        state.messages.push(MessageItem::Assistant {
            text: String::new(),
            model: state.current_model.clone(),
            timestamp: current_timestamp(),
            expanded: true,
            thought_duration: None,
            turn_duration: None,
        });
    }
}
