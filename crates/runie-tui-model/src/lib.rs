//! Renderer-independent TUI projections.
//!
//! This crate deliberately has no terminal, widget, or core-runtime
//! dependency. Actors reduce events into these immutable values; renderers
//! only consume them.

mod dialog;
mod dialog_specs;
mod events;
mod feed;
mod memory;
mod palette_action;
mod palette_meta;
pub use palette_action::PaletteAction;
pub use palette_meta::theme_labels;
mod prompt;
mod scroll;
mod scroll_flush;
mod status;
mod sticky;
mod theme;
mod ui;

pub use dialog::{
    wrap_dialog_selection, DialogAction, DialogFrame, DialogKind, DialogPredicate, DialogResult,
    DialogSpec, DialogStack,
};
pub use dialog_specs::{
    CHANGELOG_DIALOG, COMMAND_DIALOG, COMMAND_RESULT_DIALOG, FILE_SELECTOR_DIALOG,
    MODEL_SELECTOR_DIALOG, PALETTE_PARAMETERS_DIALOG, SESSION_DIALOG, SHORTCUTS_DIALOG,
    THEME_SELECTOR_DIALOG, USER_QUESTION_DIALOG,
};
pub use events::{
    event_projection_scope, is_actor_feed_event, project_event, scrollback_messages_for_event,
    status_messages_for_event, EventProjection, EventProjectionScope,
};
pub use feed::{
    active_tool_count, activity_counts, activity_counts_with_start,
    activity_group_exists_since_latest_user, activity_text, append_user_with_timestamp,
    append_wrapped, append_wrapped_words, atx_heading, background_messages_for_event,
    bus_messages_for_event, classify_activity_tool, completed_tool_header_with_args,
    current_tool_args, current_tool_header, default_tool_display_mode, dense_tool_group_members,
    dense_tool_group_members_with_identity, find_all_containing, find_first_containing,
    format_clock_timestamp, format_elapsed, format_error, grok_effective_compact,
    grok_small_screen_tip_visible, is_fence, is_output_tool, is_quit_command, is_table_row,
    is_table_separator, is_transport_only_update, last_assistant_text, line_is_blank,
    logical_tool_member_index, logical_tool_member_index_at, make_relative_path,
    model_selector_rows, project_tool_blocks, project_tool_card_rows, repository_label,
    running_bullet, selected_cell_text, session_start_messages, structured_update_messages,
    structured_update_text, table_bottom_border, thinking_summary, tool_card_summaries,
    tool_header, tool_mode_for_line, tool_mode_override_for_line, tool_result_text,
    tool_update_header_text, version_badge, web_search_site_count, web_search_sources_line,
    welcome_modal_lines, workflow_text, ActivityKind, CellPosition, CellSelection, FeedFacts,
    FeedNavigation, FeedSnapshot, FeedState, Line, LineKind, ScrollbackContentEvent,
    ScrollbackDomain, ScrollbackEvent, ScrollbackLifecycleEvent, ScrollbackMsg,
    ScrollbackNavigationEvent, ScrollbackToolEvent, ScrollbackWorkflowEvent, ToolBlock,
    ToolCardKind, ToolCardPaintIntent, ToolCardRow, ToolCardRowKind, ToolCardSummary,
    ToolNameLookup, ToolRecord, VersionBadgeVariant, DEFAULT_THINKING_ELAPSED_MS,
    GROK_AUTO_COMPACT_MAX_ROWS, GROK_SMALL_SCREEN_TIP_MAX_ROWS, PROMPT_TIMESTAMP_LIVE_THRESHOLD,
    RUNNING_BULLETS, USER_PREFIX_INDENT,
};
pub use memory::{memory_display_lines, parse_memory_results, MemoryResult};
pub use prompt::{cycle_input_mode, InputMode, PromptOutcome, PromptSnapshot};
pub use scroll::{
    ScrollDirection, ScrollFinalize, ScrollFlush, ScrollFlushState, ScrollMode, ScrollNormalizer,
    DEFAULT_SCROLL_FLUSH_CADENCE_MS, MIN_SCROLL_FLUSH_LINES,
};
pub use status::{
    format_worked_for_seconds, turn_status_text, Status, StatusMsg, StatusSnapshot,
    TurnStatusPhase, BRAILLE_SPINNER_FALLBACK, BRAILLE_SPINNER_FRAMES, DOT_SPINNER_FALLBACK,
    DOT_SPINNER_FRAMES,
};
pub use sticky::{
    compute_sticky_layout, PromptDescriptor, RenderedPrompt, StickyHeaderLayout, MIN_PINNED_HEIGHT,
};
pub use theme::ThemeToken;
pub use ui::{
    palette_display_rows, palette_labels, ui_messages_for_event, UiCommand, UiMsg, UiState,
};

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
        assert_eq!(snapshot.header_meter(), "0 turn / 500K");
    }

    #[test]
    fn command_palette_escape_clears_query_before_closing() {
        let state = UiState::new()
            .update(UiMsg::ToggleCommandPalette)
            .update(UiMsg::CommandPaletteChar('n'))
            .update(UiMsg::CommandPaletteEscape);
        assert!(state.command_palette_open);
        assert!(state.command_palette_query.is_empty());
        assert_eq!(state.dialog_stack.depth(), 1);

        let state = state.update(UiMsg::CommandPaletteEscape);
        assert!(!state.command_palette_open);
        assert!(state.dialog_stack.is_empty());
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
            autoscroll: true,
            follow_latest_user: true,
            theme: ThemeKind::GrokNight,
            ..FeedSnapshot::default()
        }
    }
}
