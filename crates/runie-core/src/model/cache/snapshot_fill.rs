//! Snapshot fill helpers — populates Snapshot fields from AppState.
//! Extracted from cache/mod.rs to stay under the 500-line limit.

use crate::model::state::AppState;
use crate::snapshot::Snapshot;

/// Build the input box title string.
/// Format: `mode · provider/model · read-only` when non-single mode and read-only,
/// `mode · provider/model` when non-single mode, and `provider/model` otherwise.
pub(crate) fn build_input_title(
    provider: &str,
    model: &str,
    read_only: bool,
    mode_active: &str,
) -> String {
    // Models are sometimes stored with their provider prefix (e.g. "minimax/MiniMax-M3");
    // avoid rendering "minimax/minimax/MiniMax-M3".
    let base = if let Some(stripped) = model.strip_prefix(&format!("{}/", provider)) {
        format!("{}/{}", provider, stripped)
    } else {
        format!("{}/{}", provider, model)
    };
    let with_mode = if mode_active == "single" {
        base
    } else {
        format!("{} · {}", mode_active, base)
    };
    if read_only {
        format!("{} · read-only", with_mode)
    } else {
        with_mode
    }
}

pub(crate) fn fill_snapshot_input(s: &mut Snapshot, state: &AppState) {
    let input = state.input();
    let completion = state.completion();
    s.input = input.input.clone();
    s.cursor_pos = input.cursor_pos;
    let (display, display_cursor) = input.display_view();
    s.input_display = display;
    s.cursor_display = display_cursor;
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
    s.animation_frame = state.view().animation_frame;
    s.turn_elapsed_secs = state.turn_elapsed_secs();
    s.current_tool_name = agent.current_tool_name.clone();
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
    s.thinking_level = state.effective_thinking_level();
    s.read_only = config.read_only;
    s.input_title = build_input_title(
        &config.current_provider,
        &config.current_model,
        config.read_only,
        &config.mode.active,
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
    s.is_pending_user_input = state.permission_request_opt().is_some();
    s.last_visible_height = state.view().last_visible_height;
    // Plan mode projection
    s.plan_mode = state.view().plan_mode;
    s.active_plan_content = state.view().active_plan_content.clone();
    s.active_plan_id = state.view().active_plan_id.clone();
    // Auto-approve mode projection
    s.auto_mode = state.view().auto_mode;
    // Tasks pane and subagent detail projection
    s.tasks_pane_visible = state.view().tasks_pane_visible;
    s.tasks_pane_show_done = state.view().tasks_pane_show_done;
    s.subagent_detail = state.view().subagent_detail.clone();
    s.feed_element_detail = state.view().feed_element_detail.clone();
    s.pattern_workers = state.agent_state().pattern_workers.clone().into();
    s.follow_mode = state.view().follow_mode;
    s.scroll_margin = state.view().scroll_margin;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_title_default_is_base() {
        let title = build_input_title("openai", "gpt-4o", false, "single");
        assert_eq!(title, "openai/gpt-4o");
    }

    #[test]
    fn input_title_includes_read_only() {
        let title = build_input_title("openai", "gpt-4o", true, "single");
        assert!(
            title.contains("read-only"),
            "title should contain read-only: {title}"
        );
    }

    #[test]
    fn input_title_no_suffix_for_default() {
        let title = build_input_title("anthropic", "claude-3-5-sonnet", false, "single");
        assert!(
            !title.contains("read-only"),
            "read-only should not appear: {title}"
        );
    }

    #[test]
    fn input_title_uses_provider_and_model() {
        let title = build_input_title("google", "gemini-2.5", false, "single");
        assert!(
            title.starts_with("google/"),
            "title should start with provider: {title}"
        );
        assert!(
            title.contains("gemini-2.5"),
            "title should contain model: {title}"
        );
    }

    #[test]
    fn input_title_includes_non_single_mode() {
        let title = build_input_title("openai", "gpt-4o", false, "swarm");
        assert_eq!(title, "swarm · openai/gpt-4o");
    }

    #[test]
    fn input_title_single_mode_omits_mode() {
        let title = build_input_title("openai", "gpt-4o", false, "single");
        assert!(
            !title.contains("single"),
            "single mode should not appear in title: {title}"
        );
    }

    #[test]
    fn input_title_mode_and_read_only() {
        let title = build_input_title("openai", "gpt-4o", true, "improve");
        assert_eq!(title, "improve · openai/gpt-4o · read-only");
    }

    #[test]
    fn input_title_dedupes_provider_prefix() {
        let title = build_input_title("minimax", "minimax/MiniMax-M3", false, "single");
        assert_eq!(title, "minimax/MiniMax-M3");
    }

    #[test]
    fn input_title_dedupes_provider_prefix_with_mode() {
        let title = build_input_title("minimax", "minimax/MiniMax-M3", false, "swarm");
        assert_eq!(title, "swarm · minimax/MiniMax-M3");
    }
}
