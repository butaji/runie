//! Copy and metadata handlers.

use crate::components::MessageItem;
use crate::tui::state::AppState;

pub fn copy_block_metadata(state: &mut AppState) {
    if let Some(item) = state.messages.get(state.scroll.feed_offset) {
        if let Some(metadata) = build_block_metadata(item) {
            tracing::info!("Copy block metadata: {}", metadata);
            state.input_right_info = "Metadata copied".to_string();
        }
    }
}

fn build_block_metadata(item: &MessageItem) -> Option<String> {
    match item {
        MessageItem::Assistant { model, timestamp, .. } => {
            Some(format!("model: {:?}, timestamp: {:?}", model, timestamp))
        }
        MessageItem::User { model, timestamp, .. } => {
            Some(format!("model: {:?}, timestamp: {:?}", model, timestamp))
        }
        _ => None,
    }
}
