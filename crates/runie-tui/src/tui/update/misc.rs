use crate::components::MessageItem;
use crate::tui::state::{AppState, Msg, Cmd, TuiMode};
use super::agent::to_agent_messages;

fn current_timestamp() -> Option<String> {
    use chrono::Local;
    Some(Local::now().format("%-I:%M %p").to_string())
}

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
    if let Some(cmds) = try_validate_submit(state, &text) {
        return cmds;
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

fn try_validate_submit(state: &mut AppState, text: &str) -> Option<Vec<Cmd>> {
    if text.chars().all(|c| c.is_whitespace()) {
        state.input_right_info = "Type a message first".to_string();
        return Some(vec![]);
    }
    if state.agent_running {
        state.input_right_info = "Agent running (blocked)... Ctrl+C to stop".to_string();
        return Some(vec![]);
    }
    if let Some(ref onboarding) = state.onboarding {
        if onboarding.is_fetching_models {
            state.input_right_info = "Loading models...".to_string();
            state.messages.push(MessageItem::System {
                text: "Loading available models... Your message will be sent when ready.".to_string(),
            });
            return Some(vec![]);
        }
    }
    None
}

fn submit_add_user_message(state: &mut AppState, text: &str) {
    state.messages.push(MessageItem::User {
        text: text.to_string(),
        model: Some("You".to_string()),
        timestamp: current_timestamp(),
    });
    state.textarea.select_all();
    state.textarea.delete_line_by_end();
}
