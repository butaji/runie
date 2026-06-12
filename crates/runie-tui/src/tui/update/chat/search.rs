//! History search-related message handlers.

use super::ChatCmd;
use crate::tui::state::AppState;

pub fn handle_history_search_msg(state: &mut AppState, msg: crate::tui::state::Msg) -> Vec<ChatCmd> {
    use crate::tui::state::Msg;
    match msg {
        Msg::HistorySearchStart => { start_history_search(state); vec![] }
        Msg::HistorySearchQuery(c) => { append_search_query(state, c); vec![] }
        Msg::HistorySearchBackspace => { delete_search_query(state); vec![] }
        Msg::HistorySearchNext => { navigate_search_next(state); vec![] }
        Msg::HistorySearchPrev => { navigate_search_prev(state); vec![] }
        Msg::HistorySearchConfirm => { confirm_search(state); vec![] }
        Msg::HistorySearchCancel => { cancel_search(state); vec![] }
        _ => vec![],
    }
}

fn start_history_search(state: &mut AppState) {
    if !state.input_history.is_empty() {
        state.history_search_query.clear();
        state.history_search_matches = (0..state.input_history.len()).rev().collect();
        state.history_search_index = 0;
        if state.input_history_index.is_none() && state.input_draft.is_empty() {
            state.input_draft = state.textarea.lines().join("\n");
        }
    }
}

fn confirm_search(state: &mut AppState) {
    state.history_search_query.clear();
    state.history_search_matches.clear();
    state.history_search_index = 0;
    state.input_history_index = None;
    state.input_draft.clear();
}

fn cancel_search(state: &mut AppState) {
    state.textarea.select_all();
    state.textarea.cut();
    state.textarea.insert_str(&state.input_draft);
    state.input_draft.clear();
    state.input_history_index = None;
    state.history_search_query.clear();
    state.history_search_matches.clear();
    state.history_search_index = 0;
}

fn append_search_query(state: &mut AppState, c: char) {
    state.history_search_query.push(c);
    update_history_search(state);
}

fn delete_search_query(state: &mut AppState) {
    state.history_search_query.pop();
    update_history_search(state);
}

fn navigate_search_next(state: &mut AppState) {
    if !state.history_search_matches.is_empty() {
        state.history_search_index = (state.history_search_index + 1)
            .min(state.history_search_matches.len() - 1);
        apply_search_selection(state);
    }
}

fn navigate_search_prev(state: &mut AppState) {
    state.history_search_index = state.history_search_index.saturating_sub(1);
    apply_search_selection(state);
}

fn update_history_search(state: &mut AppState) {
    let query = state.history_search_query.to_lowercase();
    state.history_search_matches = state.input_history
        .iter()
        .enumerate()
        .filter(|(_, text)| text.to_lowercase().contains(&query))
        .map(|(i, _)| i)
        .rev()
        .collect();
    state.history_search_index = 0;
    apply_search_selection(state);
}

fn apply_search_selection(state: &mut AppState) {
    if let Some(&idx) = state.history_search_matches.get(state.history_search_index) {
        if let Some(text) = state.input_history.get(idx) {
            state.textarea.select_all();
            state.textarea.cut();
            state.textarea.insert_str(text);
            state.input_history_index = Some(idx);
        }
    }
}
