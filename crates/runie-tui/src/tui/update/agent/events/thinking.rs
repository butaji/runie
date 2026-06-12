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

pub(super) fn on_thinking_update(state: &mut AppState, delta: &str) {
    if let Some(ref mut thinking) = state.thinking {
        if let Some(start) = thinking.start.take() {
            thinking.accrued_duration = Some(thinking.accrued_duration.unwrap_or_default() + start.elapsed());
            thinking.start = Some(std::time::Instant::now());
        }
        if !delta.is_empty() {
            thinking.text.push_str(delta);
        }
    }
    // Note: thinking content is stored in state.thinking.text and rendered
    // separately via streaming_think_content. Do NOT append to assistant_text.
}

pub(super) fn on_thinking_end(state: &mut AppState, duration_ms: u64) {
    let duration = std::time::Duration::from_millis(duration_ms);

    // Record final thinking duration
    if let Some(ref mut thinking) = state.thinking {
        if let Some(start) = thinking.start.take() {
            thinking.accrued_duration = Some(thinking.accrued_duration.unwrap_or_default() + start.elapsed());
        }
    }

    // Pick the larger of the param-supplied duration and the locally
    // accrued wall-clock duration. In replay (or any fast test) the
    // wall-clock elapsed is ~0, so the param is authoritative. In live
    // runs the accrued wall-clock wins for accuracy.
    let accrued = state.thinking.as_ref()
        .and_then(|t| t.accrued_duration)
        .unwrap_or_default();
    let final_duration = if accrued > duration { accrued } else { duration };

    // For replay/test: there is no agent_start_time set (no submit
    // happened), so the subsequent TurnEnd will compute elapsed_secs = 0
    // and the "Turn completed in Xs" separator will be missing. Record
    // the thinking duration here so on_turn_end can use it as a proxy
    // when the wall clock is at 0. We use a separate field
    // `replay_turn_duration_secs` (not reset by on_thinking_start) so a
    // second ThinkingStart in the same turn doesn't wipe it.
    state.replay_turn_duration_secs = Some(final_duration.as_secs_f32().max(0.1));

    // Update thought_duration on last assistant message if thinking took > 0.5s.
    //
    // On the real wire order ThinkingEnd sometimes arrives before the
    // matching MessageStart (e.g. a quick thinking pass with no streamed
    // text). In that case there is no assistant message yet — stash the
    // duration in `state.pending_thought_duration` and let
    // `on_message_start` attach it when the assistant placeholder is
    // created.
    if final_duration.as_secs_f32() > 0.5 {
        if let Some(MessageItem::Assistant { thought_duration: ref mut td, .. }) = state.messages.last_mut() {
            *td = Some(final_duration.as_secs_f32());
        } else {
            state.pending_thought_duration = Some(final_duration.as_secs_f32());
        }
    }

    // Note: thinking content is stored in state.thinking.text and rendered
    // separately via streaming_think_content. Do NOT append to assistant_text.
    state.thinking = None;
}

/// Ensure a thinking placeholder exists when receiving thinking content.
/// This is called from message update handlers to create an assistant
/// placeholder if one doesn't exist yet.
pub(super) fn ensure_thinking_placeholder(state: &mut AppState, content: &[runie_agent::events::ContentPart]) {
    let is_thinking = content.iter().any(|part| {
        if let runie_agent::events::ContentPart::Text { text } = part {
            text.trim_start().starts_with("<think>")
        } else {
            false
        }
    });

    if !is_thinking {
        return;
    }

    let has_no_assistant = !state.messages.iter().any(|m| matches!(m, MessageItem::Assistant { .. }));
    let last_has_thinking = state.messages.last()
        .map(|m| matches!(m, MessageItem::Assistant { text, .. } if text.trim_start().starts_with("<think>")))
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
