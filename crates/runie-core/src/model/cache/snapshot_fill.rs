//! Snapshot fill helpers — populates Snapshot fields from AppState.
//! Extracted from cache/mod.rs to stay under the 500-line limit.

use crate::model::state::AppState;
use crate::snapshot::Snapshot;

/// Build the input box title string.
/// Format: `provider/model · read-only` when read-only, otherwise `provider/model`.
pub(crate) fn build_input_title(provider: &str, model: &str, read_only: bool) -> String {
    let base = format!("{}/{}", provider, model);
    if read_only {
        format!("{} · read-only", base)
    } else {
        base
    }
}

pub(crate) fn fill_snapshot_input(s: &mut Snapshot, state: &AppState) {
    let input = state.input();
    let completion = state.completion();
    s.input = input.input.clone();
    s.cursor_pos = input.cursor_pos;
    s.hint_text = state.hint_text();
    s.placeholder = input.placeholder.clone();
    s.ghost_completion = input.ghost_completion.clone();
    s.input_scroll = input.input_scroll;
    s.path_suggestions = completion.path_suggestions.clone();
    s.path_selected = completion.path_selected;
}

pub(crate) fn fill_snapshot_agent(s: &mut Snapshot, state: &AppState) {
    // Sync authoritative turn fields from TurnState to AgentState for the snapshot.
    // Non-authoritative fields (queues, streaming_tail) retain their test-set values.
    let agent = state.agent_state();
    let input = state.input();
    let view = state.view();
    s.turn_active = agent.turn_active;
    s.input_flash = input.input_flash;
    s.vim_nav_mode = view.vim_nav_mode;
    s.spinner_frame = state.spinner_frame();
    s.turn_elapsed_secs = state.turn_elapsed_secs();
    s.queue_count = agent.message_queue.len() + agent.request_queue.len();
    s.tokens_in = agent.tokens_in;
    s.tokens_out = agent.tokens_out;
    s.speed_tps = agent.speed_tps;
    s.tokens_in_display = agent.tokens_in_display;
    s.tokens_out_display = agent.tokens_out_display;
    s.streaming_tail = agent.streaming_buffer.tail().to_owned();
}

pub(crate) fn fill_snapshot_config(s: &mut Snapshot, state: &AppState) {
    let config = state.config();
    s.provider = config.current_provider.clone();
    s.model = config.current_model.clone();
    s.has_models = state.has_models();
    s.theme_name = config.theme_name.clone();
    s.thinking_level = config.thinking_level;
    s.read_only = config.read_only;
    s.input_title = build_input_title(
        &config.current_provider,
        &config.current_model,
        config.read_only,
    );
}

pub(crate) fn fill_snapshot_dialog(s: &mut Snapshot, state: &AppState) {
    s.dialog = state.open_dialog().cloned();
    s.palette_items = state.palette_items();
    s.model_selector_items = state.model_selector_items();
    s.settings_items = state.settings_items();
    s.session_tree_items = state.session_tree_items();
    s.auth_providers = state.auth_providers();
}

pub(crate) fn fill_snapshot_meta(s: &mut Snapshot, state: &AppState) {
    s.transient_message = state.transient_message().cloned();
    s.transient_level = state.transient_level;
    s.git_info = state.git_info().cloned();
    s.cwd_name = state.cwd_name().to_owned();
    s.pending_edits = state.session().pending_edits.clone();
    s.scoped_models = state.config().scoped_models.clone();
    s.image_attachments = state.session().image_attachments.clone();
    s.permission_request = state.permission_request_opt().cloned();
    s.last_visible_height = state.view().last_visible_height;
    // Plan mode projection
    s.plan_mode = state.view().plan_mode;
    s.active_plan_content = state.view().active_plan_content.clone();
}
