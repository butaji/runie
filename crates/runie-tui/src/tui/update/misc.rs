use crate::components::MessageItem;
use crate::tui::state::{AppState, Msg, Cmd, TuiMode};
use super::agent::to_agent_messages;

pub fn handle_scroll(state: &mut AppState, amount: usize) {
    let max_scroll = state.messages.len().saturating_sub(1);
    state.scroll.feed_offset = (state.scroll.feed_offset + amount).min(max_scroll);
}

/// Returns Some(Msg::PermissionTimeout) if permission modal has timed out
pub fn check_permission_timeout(state: &AppState) -> Option<Msg> {
    // Only check if we're in permission mode with timeout tracking active
    if state.mode != TuiMode::Permission {
        return None;
    }
    
    // Skip if already timed out
    if state.permission_modal.timed_out {
        return None;
    }
    
    // Check if we have a timeout start time
    if let Some(start) = state.permission_modal.timeout_start {
        const TIMEOUT_SECS: u64 = 300; // 5 minutes
        let elapsed = start.elapsed();
        if elapsed.as_secs() >= TIMEOUT_SECS {
            return Some(Msg::PermissionTimeout);
        }
    }
    
    None
}

pub fn handle_anim(state: &mut AppState, msg: &Msg) {
    match msg {
        Msg::Tick => {
            state.animation.braille_frame = (state.animation.braille_frame + 1) % 10;
            state.animation.rewind_braille_frame = (state.animation.rewind_braille_frame + 1) % 10;
        }
        Msg::CursorBlink => {
            state.animation.streaming_cursor_visible = !state.animation.streaming_cursor_visible;
        }
        _ => {}
    }
}

pub fn handle_submit(state: &mut AppState) -> Vec<Cmd> {
    let text = state.textarea.lines().join("\n");
    if text.is_empty() {
        // P0-3 FIX: Show feedback when submitting empty input
        state.input_right_info = "Type a message first".to_string();
        return vec![];
    }
    
    // BG-7 & P1-4 FIX: Block double-submit with user feedback
    if state.agent_running {
        state.messages.push(MessageItem::System {
            text: "Agent is still running. Please wait or press Ctrl+C to stop the current task.".to_string(),
        });
        return vec![];
    }
    
    // BG-5 FIX: Block submit while models are still being fetched in onboarding
    if let Some(ref onboarding) = state.onboarding {
        if onboarding.is_fetching_models {
            state.messages.push(MessageItem::System {
                text: "Still loading models... Please wait.".to_string(),
            });
            return vec![];
        }
    }
    
    // P0-2 FIX: Block submit when no model is configured - guide user to onboarding
    // But still clear textarea and add message for better UX
    let model_missing = state.current_model.is_none() && state.onboarding.is_none();
    
    state.messages.push(MessageItem::User {
        text: text.clone(),
        model: Some("You".to_string()),
        timestamp: None,
    });
    // Clear textarea
    state.textarea.select_all();
    state.textarea.delete_line_by_end();
    
    if model_missing {
        state.messages.push(MessageItem::System {
            text: "No model configured. Press Ctrl+O or type /onboard to set up a model.".to_string(),
        });
        return vec![];
    }
    
    // P1-6 FIX: Validate model is configured before spawning agent
    // Only block agent spawn if in a real running state (agent_running would be false anyway)
    // We allow submit in all cases so the user message always gets recorded.
    // The agent loop itself handles the case where no model is configured.
    vec![Cmd::SpawnAgent { messages: to_agent_messages(&state.messages) }]
}
