use crate::app::UiState;
use runie_core::session::SessionSnapshot;
use runie_core::types::AgentMessage;

pub(super) fn dialog_is_visible(ui: &UiState, id: &'static str) -> bool {
    let legacy_open = match id {
        "shortcuts" => ui.shortcuts_open,
        "commands" => ui.command_palette_open,
        "model" => ui.model_selector_open,
        "session" => ui.session_info_open,
        "changelog" => ui.changelog_open,
        "command-result" => ui.command_result.is_some(),
        _ => false,
    };
    legacy_open && (ui.dialog_stack.is_empty() || ui.dialog_stack.top_id() == Some(id))
}

pub(super) fn compaction_token_estimates(snapshot: &SessionSnapshot) -> Vec<u64> {
    snapshot
        .entries
        .iter()
        .map(|entry| runie_core::session::estimate_message_tokens(&entry.message))
        .collect()
}

pub(super) fn compaction_retained_tail(
    snapshot: &SessionSnapshot,
    preparation: &runie_core::session::CompactionPreparation,
) -> Vec<AgentMessage> {
    preparation
        .retained_indices
        .iter()
        .filter_map(|index| snapshot.entries.get(*index))
        .map(|entry| entry.message.clone())
        .collect()
}
