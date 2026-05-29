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
        state.input_right_info = "Type a message first".to_string();
        return vec![];
    }

    if state.agent_running {
        state.input_right_info = "Agent running (blocked)... Ctrl+C to stop".to_string();
        return vec![];
    }

    if let Some(ref onboarding) = state.onboarding {
        if onboarding.is_fetching_models {
            state.input_right_info = "Loading models...".to_string();
            return vec![];
        }
    }

    let model_missing = state.current_model.as_deref().map_or(true, |s| s.is_empty()) && state.onboarding.is_none();
    submit_add_user_message(state, &text);

    if model_missing {
        state.messages.push(MessageItem::System {
            text: "No model configured. Press Ctrl+O or type /onboard to set up a model.".to_string(),
        });
        return vec![];
    }

    vec![Cmd::SpawnAgent { messages: to_agent_messages(&state.messages) }]
}

fn submit_add_user_message(state: &mut AppState, text: &str) {
    state.messages.push(MessageItem::User {
        text: text.to_string(),
        model: Some("You".to_string()),
        timestamp: None,
    });
    state.textarea.select_all();
    state.textarea.delete_line_by_end();
}
