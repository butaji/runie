//! History browsing handlers.

use crate::tui::state::AppState;
use super::ChatCmd;

pub(super) fn handle_history_msg(state: &mut AppState, msg: crate::tui::state::Msg) -> Vec<ChatCmd> {
    use crate::tui::state::Msg;
    match msg {
        Msg::HistoryUp => handle_history_up(state),
        Msg::HistoryDown => handle_history_down(state),
        _ => vec![],
    }
}

pub(super) fn handle_history_up(state: &mut AppState) -> Vec<ChatCmd> {
    if state.input_history.is_empty() {
        return vec![];
    }

    // Save current draft only on the FIRST history-up press
    if state.input_history_index.is_none() && state.input_draft.is_empty() {
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
    vec![]
}

pub(super) fn handle_history_down(state: &mut AppState) -> Vec<ChatCmd> {
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
    vec![]
}
