//! Renderer-independent TUI projections.
//!
//! This crate deliberately has no terminal, widget, or core-runtime
//! dependency. Actors reduce events into these immutable values; renderers
//! only consume them.

mod events;
mod feed;
mod memory;
mod prompt;
mod scroll;
mod status;
mod theme;
mod ui;

pub use events::{is_actor_feed_event, status_messages_for_event};
pub use feed::{
    activity_text, append_user_with_timestamp, append_wrapped, append_wrapped_words, atx_heading,
    classify_activity_tool, completed_tool_header_with_args, default_tool_display_mode,
    format_clock_timestamp, format_elapsed, format_error, grok_effective_compact,
    grok_small_screen_tip_visible, is_fence, is_output_tool, is_quit_command, is_table_row,
    is_table_separator, is_transport_only_update, logical_tool_member_index, make_relative_path,
    model_selector_rows, project_tool_blocks, project_tool_card_rows, repository_label,
    running_bullet, session_start_messages, structured_update_text, table_bottom_border,
    thinking_summary, tool_header, tool_mode_for_line, tool_mode_override_for_line,
    tool_result_text, tool_update_header_text, version_badge, web_search_site_count,
    web_search_sources_line, welcome_modal_lines, workflow_text, ActivityKind, CellPosition,
    CellSelection, FeedNavigation, FeedSnapshot, FeedState, Line, LineKind, ScrollbackMsg,
    ToolBlock, ToolCardKind, ToolCardPaintIntent, ToolCardRow, ToolCardRowKind,
    VersionBadgeVariant, DEFAULT_THINKING_ELAPSED_MS, GROK_AUTO_COMPACT_MAX_ROWS,
    GROK_SMALL_SCREEN_TIP_MAX_ROWS, PROMPT_TIMESTAMP_LIVE_THRESHOLD, RUNNING_BULLETS,
    USER_PREFIX_INDENT,
};
pub use memory::{memory_display_lines, parse_memory_results, MemoryResult};
pub use prompt::{cycle_input_mode, InputMode, PromptOutcome, PromptSnapshot};
pub use scroll::{
    ScrollDirection, ScrollFinalize, ScrollFlush, ScrollFlushState, ScrollMode, ScrollNormalizer,
    DEFAULT_SCROLL_FLUSH_CADENCE_MS, MIN_SCROLL_FLUSH_LINES,
};
pub use status::{
    Status, StatusMsg, StatusSnapshot, BRAILLE_SPINNER_FALLBACK, BRAILLE_SPINNER_FRAMES,
    DOT_SPINNER_FALLBACK, DOT_SPINNER_FRAMES,
};
pub use theme::ThemeToken;
pub use ui::{ui_messages_for_event, PaletteAction, UiCommand, UiMsg, UiState};

/// Immutable aggregate of actor-owned TUI projections for a single view pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TuiSnapshot {
    pub ui: UiState,
    pub feed: FeedSnapshot,
    pub prompt: PromptSnapshot,
    pub status: StatusSnapshot,
}

/// Pure viewport projection for a feed that may follow its newest content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollState {
    pub scroll_top: u16,
    pub content_height: u16,
    pub viewport_height: u16,
    pub following_end: bool,
}

impl ScrollState {
    pub const fn new(follow_end: bool) -> Self {
        Self {
            scroll_top: 0,
            content_height: 0,
            viewport_height: 0,
            following_end: follow_end,
        }
    }

    pub const fn max_scroll_top(self) -> u16 {
        self.content_height.saturating_sub(self.viewport_height)
    }

    pub const fn update_layout(mut self, content_height: u16, viewport_height: u16) -> Self {
        self.content_height = content_height;
        self.viewport_height = viewport_height;
        let max = self.max_scroll_top();
        self.scroll_top = if self.following_end || self.scroll_top > max {
            max
        } else {
            self.scroll_top
        };
        if self.following_end && self.content_height <= self.viewport_height {
            self.scroll_top = 0;
        }
        self
    }

    pub const fn scroll_to(mut self, requested: u16) -> Self {
        let max = self.max_scroll_top();
        self.scroll_top = if requested < max { requested } else { max };
        self.following_end = self.scroll_top == max;
        self
    }

    pub const fn append_content(mut self, content_height: u16) -> Self {
        self.content_height = content_height;
        if self.following_end {
            self.scroll_top = self.max_scroll_top();
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FeedSnapshot, PromptSnapshot, ScrollState, Status, StatusSnapshot, TuiSnapshot, UiMsg,
        UiState,
    };
    use runie_core::types::{AgentEvent, ThemeKind};

    #[test]
    fn following_feed_tracks_appended_content() {
        let state = ScrollState::new(true)
            .update_layout(20, 5)
            .append_content(27);
        assert_eq!(state.scroll_top, 22);
        assert!(state.following_end);
    }

    #[test]
    fn explicit_scroll_detaches_from_tail_until_tail_is_reached() {
        let state = ScrollState::new(true)
            .update_layout(20, 5)
            .scroll_to(3)
            .append_content(27);
        assert_eq!(state.scroll_top, 3);
        assert!(!state.following_end);
    }

    #[test]
    fn aggregate_snapshot_contains_only_actor_projections() {
        let snapshot = TuiSnapshot {
            ui: UiState::new(),
            feed: empty_feed(),
            prompt: PromptSnapshot {
                text: String::new(),
                focused: true,
                history: Vec::new(),
                history_index: None,
                history_search: false,
                mode: super::InputMode::Normal,
                model_caption: "model".into(),
                show_placeholder: true,
                file_candidates: Vec::new(),
                file_candidate_index: 0,
                selected_file: None,
                viewer_lines: Vec::new(),
                theme: ThemeKind::GrokNight,
            },
            status: StatusSnapshot {
                state: Status::Ready,
                theme: ThemeKind::GrokNight,
                animation_frame: 0,
                elapsed_ticks: 0,
                turn_usage: None,
                turn_stop_reason: None,
                context_window: None,
                thinking_elapsed_ms: None,
            },
        };
        assert!(!snapshot.ui.show_welcome);
        assert!(snapshot.feed.is_empty());
        assert_eq!(snapshot.status.state, Status::Ready);
    }

    #[test]
    fn status_snapshot_projects_header_meter_without_renderer_types() {
        let snapshot = StatusSnapshot {
            state: Status::Ready,
            theme: ThemeKind::GrokNight,
            animation_frame: 0,
            elapsed_ticks: 0,
            turn_usage: None,
            turn_stop_reason: None,
            context_window: None,
            thinking_elapsed_ms: None,
        };
        assert_eq!(snapshot.header_meter(), "0 / 500K");
    }

    #[test]
    fn command_palette_escape_clears_query_before_closing() {
        let state = UiState::new()
            .update(UiMsg::ToggleCommandPalette)
            .update(UiMsg::CommandPaletteChar('n'))
            .update(UiMsg::CommandPaletteEscape);
        assert!(state.command_palette_open);
        assert!(state.command_palette_query.is_empty());

        let state = state.update(UiMsg::CommandPaletteEscape);
        assert!(!state.command_palette_open);
    }

    #[test]
    fn model_selector_reduces_query_scope_and_navigation_events() {
        let state = UiState::new()
            .update(UiMsg::SetModelSelectorResultCount(3))
            .update(UiMsg::ToggleModelSelector)
            .update(UiMsg::ModelSelectorChar('g'))
            .update(UiMsg::ModelSelectorMove(2));
        assert!(state.model_selector_open);
        assert_eq!(state.model_selector_query, "g");
        assert_eq!(state.model_selector_index, 2);
        let state = state.update(UiMsg::ModelSelectorToggleScope);
        assert!(state.model_selector_scoped_only);
        assert_eq!(state.model_selector_index, 0);

        let state = state.update(UiMsg::ModelSelectorEscape);
        assert!(state.model_selector_open);
        assert!(state.model_selector_query.is_empty());
        assert_eq!(state.model_selector_index, 0);
        assert!(!state.update(UiMsg::ModelSelectorEscape).model_selector_open);
    }

    #[test]
    fn model_selector_activation_closes_only_ui_projection() {
        let state = UiState::new()
            .update(UiMsg::ToggleModelSelector)
            .update(UiMsg::ModelSelectorChar('g'))
            .update(UiMsg::ActivateModelSelector);
        assert!(!state.model_selector_open);
        assert!(state.model_selector_query.is_empty());
    }

    #[test]
    fn ui_core_event_mapping_is_explicit_and_pure() {
        assert_eq!(
            super::ui_messages_for_event(&AgentEvent::Reset),
            vec![UiMsg::Reset]
        );
        assert!(super::ui_messages_for_event(&AgentEvent::AgentStart).is_empty());
    }

    fn empty_feed() -> FeedSnapshot {
        FeedSnapshot {
            lines: Vec::new(),
            tool_blocks: Vec::new(),
            tool_names: std::collections::HashMap::new(),
            tool_args: std::collections::HashMap::new(),
            activity_dirs: 0,
            activity_files: 0,
            activity_commands: 0,
            activity_subagents: 0,
            activity_failures: 0,
            autoscroll: true,
            scroll_offset: 0,
            reasoning_expanded: false,
            activity_expanded: false,
            prompt_timestamp: None,
            follow_latest_user: true,
            selected_tool_id: None,
            selected_entry: None,
            selection_anchor: None,
            selection_head: None,
            cell_selection: None,
            copy_selection: None,
            selected_member_index: None,
            theme: ThemeKind::GrokNight,
            animation_frame: 0,
            tool_modes: std::collections::HashMap::new(),
            revealed_dense_groups: std::collections::HashSet::new(),
            center_revealed_entry: false,
            workflow_headers: std::collections::HashMap::new(),
            workflow_phases: std::collections::HashMap::new(),
            settled_no_tool_phase: false,
            live_grok_layout: false,
            next_tool_row_id: 0,
            turn_started: false,
            assistant_stream_open: false,
            measured_content_rows: 0,
            measured_viewport_rows: 0,
            measured_anchor_row: None,
        }
    }
}
