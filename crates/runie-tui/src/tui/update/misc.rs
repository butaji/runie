use crate::components::MessageItem;
use crate::tui::state::{AppState, Msg, Cmd};
use super::agent::to_agent_messages;

pub fn handle_scroll(state: &mut AppState, amount: usize) {
    let max_scroll = state.messages.len().saturating_sub(1);
    state.scroll.feed_offset = (state.scroll.feed_offset + amount).min(max_scroll);
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
        return vec![];
    }
    state.messages.push(MessageItem::User {
        text: text.clone(),
        model: Some("You".to_string()),
        timestamp: None,
    });
    // Clear textarea
    state.textarea.select_all();
    state.textarea.delete_line_by_end();
    vec![Cmd::SpawnAgent { messages: to_agent_messages(&state.messages) }]
}
