//! Entry fold/unfold handlers.

use crate::components::MessageItem;
use crate::tui::state::AppState;

pub(super) fn collapse_entry(state: &mut AppState) {
    if let Some(item) = state.messages.get_mut(state.scroll.feed_offset) {
        if let MessageItem::Assistant { ref mut expanded, .. } = item {
            *expanded = false;
        }
    }
}

pub(super) fn expand_entry(state: &mut AppState) {
    if let Some(item) = state.messages.get_mut(state.scroll.feed_offset) {
        if let MessageItem::Assistant { ref mut expanded, .. } = item {
            *expanded = true;
        }
    }
}

pub(super) fn toggle_fold_entry(state: &mut AppState) {
    if let Some(item) = state.messages.get_mut(state.scroll.feed_offset) {
        if let MessageItem::Assistant { ref mut expanded, .. } = item {
            *expanded = !*expanded;
        }
    }
}

pub(super) fn toggle_all_entries(state: &mut AppState) {
    // Check if any entry is collapsed
    let any_collapsed = state.messages.iter()
        .filter_map(|m| match m {
            MessageItem::Assistant { expanded, .. } => Some(*expanded),
            _ => None,
        })
        .any(|e| !e);

    // Toggle all to the opposite state
    let new_state = any_collapsed;
    for item in &mut state.messages {
        if let MessageItem::Assistant { ref mut expanded, .. } = item {
            *expanded = new_state;
        }
    }
    state.input_right_info = if new_state { "All expanded" } else { "All collapsed" }.to_string();
}

pub(super) fn copy_block_content(state: &mut AppState) {
    if let Some(item) = state.messages.get(state.scroll.feed_offset) {
        let text = match item {
            MessageItem::Assistant { text, .. } => text.clone(),
            MessageItem::User { text, .. } => text.clone(),
            MessageItem::Thought { text, .. } => text.clone(),
            MessageItem::System { text, .. } => text.clone(),
            _ => String::new(),
        };
        if !text.is_empty() {
            tracing::info!("Copy block content: {} chars", text.len());
            state.input_right_info = "Copied".to_string();
        }
    }
}

pub(super) fn copy_block_metadata(state: &mut AppState) {
    if let Some(item) = state.messages.get(state.scroll.feed_offset) {
        let metadata = match item {
            MessageItem::Assistant { model, timestamp, .. } => {
                format!("model: {:?}, timestamp: {:?}", model, timestamp)
            }
            MessageItem::User { model, timestamp, .. } => {
                format!("model: {:?}, timestamp: {:?}", model, timestamp)
            }
            _ => String::new(),
        };
        if !metadata.is_empty() {
            tracing::info!("Copy block metadata: {}", metadata);
            state.input_right_info = "Metadata copied".to_string();
        }
    }
}

pub(super) fn open_entry(state: &mut AppState) {
    if let Some(item) = state.messages.get(state.scroll.feed_offset) {
        if matches!(item, MessageItem::Assistant { .. }) {
            state.input_right_info = "Entry opened".to_string();
        }
    }
}

pub(super) fn open_entry_options(state: &mut AppState) {
    state.input_right_info = "Entry options opened".to_string();
}

pub(super) fn toggle_raw_markdown(state: &mut AppState) {
    if let Some(item) = state.messages.get(state.scroll.feed_offset) {
        if matches!(item, MessageItem::Assistant { .. }) {
            state.input_right_info = "Raw markdown toggle".to_string();
        }
    }
}

pub(super) fn focus_prompt(state: &mut AppState) {
    state.scroll.scroll_focused = false;
    state.input_right_info = String::new();
}

pub(super) fn go_home(state: &mut AppState) {
    state.mode = crate::tui::state::TuiMode::HomeScreen;
    state.textarea.select_all();
    state.textarea.delete_line_by_end();
}
