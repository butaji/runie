//! Immutable frame description — the UI DSL.
//! The event loop builds snapshots; the render actor draws them.
//! Zero blocking I/O in the event loop by design.

use crate::view::elements::Element;
use std::sync::Arc;

pub use crate::model::{FeedElementDetail, SubagentDetail};

/// A queued message projected for the queue pane.
#[derive(Clone, Debug)]
pub struct QueuedMessageView {
    /// 1-based position in the queue (matches the `#N` row prefix).
    pub position: usize,
    /// First non-empty trimmed line of the message content.
    pub first_line: String,
    /// Total non-empty line count (drives the `(+N lines)` suffix).
    pub line_count: usize,
    /// Message kind (follow-up vs steering) for per-kind styling.
    pub kind: crate::model::QueuedMessageKind,
}

/// What the active turn is currently doing (grok parity — `compute_activity`).
///
/// Minimal viable set driving the status-line activity label:
/// `Thinking…` / `Responding…` / `Running {tool}…` / `Cancelling…`, with
/// `Working` as the fallback for states without a dedicated label.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TurnActivityKind {
    /// Generic active turn with no more specific activity.
    #[default]
    Working,
    /// Model is reasoning before producing output.
    Thinking,
    /// Model output is streaming (or waiting on the model response).
    Responding,
    /// A tool is executing.
    ToolRunning,
    /// The turn is being cancelled (stop button hidden).
    Cancelling,
}

/// Git repository info detected from current working directory.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GitInfo {
    pub repo_name: Option<String>,
    pub branch: Option<String>,
    /// True when the current directory is a git worktree (not the main repo).
    pub is_worktree: bool,
    /// Path to the main repo for worktrees.
    pub worktree_source: Option<String>,
}

impl GitInfo {
    /// Format for status bar left side when turn is not active.
    /// Returns "repo/branch" when both known, "branch" when only branch known,
    /// or "folder/" when not in a git repo at all.
    /// When inside a worktree, prepends "worktree of {source}".
    pub fn format_right(&self, cwd_name: &str) -> String {
        let base = match (&self.repo_name, &self.branch) {
            (Some(repo), Some(branch)) => format!("{}/{}", repo, branch),
            (None, Some(branch)) => branch.to_string(),
            (Some(repo), None) => format!("{}/", repo),
            (None, None) => format!("{}/", cwd_name),
        };
        if self.is_worktree {
            return format!("{} • worktree", base);
        }
        base
    }
}

#[derive(Clone, Default)]
pub struct Snapshot {
    pub elements: Arc<[Element]>,
    pub line_counts: Arc<[usize]>,
    pub total_lines: usize,
    pub input: String,
    pub cursor_pos: usize,
    /// Rendered input text: labeled chips (e.g. `[Pasted: 4 lines]`) are
    /// substituted for their buffer span. Falls back to `input` when no
    /// labeled chips exist.
    pub input_display: String,
    /// Cursor position in `input_display` coordinates.
    pub cursor_display: usize,
    pub hint_text: String,
    /// Active ephemeral tip spans (grok parity) — None when no tip is
    /// renderable (none active, occluded, or terminal too short).
    pub ephemeral_tip: Option<Vec<crate::model::tips::TipSpan>>,
    /// Inline slash-command dropdown rows (grok parity) — None when closed.
    pub slash_dropdown: Option<crate::model::slash::SlashDropdown>,
    pub path_suggestions: Option<Vec<crate::path_complete::PathCompletion>>,
    pub path_selected: Option<usize>,
    pub turn_active: bool,
    pub spinner_frame: char,
    pub scroll: usize,
    /// Elapsed seconds since turn started. Captured at snapshot creation time.
    pub turn_elapsed_secs: Option<f64>,
    /// Name of the currently running tool, if any. Used by the status bar to
    /// display an activity label and by the monitor pulse glyph.
    pub current_tool_name: Option<String>,
    /// Output tokens received since the current turn started (per-turn, not
    /// session-cumulative). 0 before the first tokens of the turn arrive.
    /// Grok parity: the status line shows `⇣{Nk}` from this counter.
    pub current_turn_tokens: u64,
    /// Elapsed seconds since the current activity (tool call / phase) started.
    /// Reset on each activity transition; falls back to the turn elapsed when
    /// no narrower activity is in flight. Drives the status-line phase timer.
    pub activity_elapsed_secs: Option<f64>,
    /// What the turn is currently doing. Drives the status-line activity label
    /// (`Thinking…` / `Responding…` / `Running {tool}…` / `Cancelling…`).
    pub turn_activity: TurnActivityKind,
    /// True while a cancellation of the active turn is in flight. The status
    /// line shows `Cancelling…` (accent_error) and hides `[stop]`.
    pub turn_cancelling: bool,
    pub provider: String,
    pub model: String,
    /// Active theme name for the render actor
    pub theme_name: String,
    /// Global animation frame counter for deterministic overlays.
    pub animation_frame: u32,
    /// Current thinking level for status display
    pub thinking_level: crate::model::ThinkingLevel,
    /// Read-only mode active — only safe tools exposed to LLM
    pub read_only: bool,
    /// Flash countdown for invalid input feedback.
    pub input_flash: u8,
    /// True when the user is in vim nav mode (input box is disabled,
    /// cursor renders in the disabled style).
    pub vim_nav_mode: bool,
    /// Placeholder text shown when input is empty.
    pub placeholder: String,
    /// Ghost completion suffix shown in gray after cursor.
    pub ghost_completion: Option<String>,
    /// Queue count (pending messages in queue)
    pub queue_count: usize,
    /// Queued messages projected for the queue pane (grok parity — numbered
    /// `#N` rows above the input). FIFO order; `position` is 1-based.
    pub queued_messages: Vec<QueuedMessageView>,
    /// Whether the queued-messages pane is shown above the input.
    pub queue_pane_visible: bool,
    /// Whether the queue pane holds input focus (j/k navigate, x removes).
    pub queue_pane_focused: bool,
    /// Index of the selected queue row.
    pub queue_pane_selected: usize,
    /// Currently open dialog state for rendering overlays.
    pub dialog: Option<crate::commands::DialogState>,
    /// Filtered command list for palette rendering (name, description, category).
    pub palette_items: Arc<[(String, String, String)]>,
    /// Model selector items (provider_header, full_name, cost_str, is_selected, is_current).
    pub model_selector_items: Arc<[crate::model::ModelSelectorItem]>,
    /// Pending file edits awaiting approval.
    pub pending_edits: Vec<crate::edit_preview::EditPreview>,
    /// Scoped models for dialog rendering.
    pub scoped_models: Vec<crate::scoped_model::ScopedModel>,
    /// Settings items for dialog rendering.
    pub settings_items: Arc<[crate::settings::SettingItem]>,
    /// Session tree items for dialog rendering (depth, content preview).
    pub session_tree_items: Arc<[(usize, String)]>,
    /// Base64 image attachments pending in the input field.
    pub image_attachments: Vec<String>,
    /// Active permission approval prompt for modal rendering.
    pub permission_request: Option<crate::model::PermissionRequestState>,
    /// True when a permission prompt or ask_user_question is waiting for user input.
    /// Used by the status bar to render a pulsing diamond instead of the spinner.
    pub is_pending_user_input: bool,
    /// Authenticated providers for status display.
    pub auth_providers: Arc<[String]>,
    /// Transient notification message shown in hints line.
    pub transient_message: Option<String>,
    /// Severity level of the transient notification.
    pub transient_level: Option<crate::event::TransientLevel>,
    /// Cumulative input tokens sent to LLM.
    pub tokens_in: usize,
    /// Cumulative output tokens received from LLM.
    pub tokens_out: usize,
    /// Current streaming speed in tokens/sec.
    pub speed_tps: f64,
    /// Animated display value for tokens_in.
    pub tokens_in_display: f64,
    /// Animated display value for tokens_out.
    pub tokens_out_display: f64,
    /// Git repo info for status bar display.
    pub git_info: Option<GitInfo>,
    /// Current working directory name (fallback when no git).
    pub cwd_name: String,
    /// Top visible line index for multi-line input scrolling.
    pub input_scroll: usize,
    /// Height of the message viewport (updated by the render actor).
    pub last_visible_height: u16,
    /// Width of the message content area (updated by the render actor).
    pub content_width: u16,
    /// Total terminal rows. `0` = unmeasured. Used for auto-compact derivation.
    pub terminal_rows: u16,
    /// Derived compact layout flag: `effective_compact(user_setting, terminal_rows)`.
    /// In-memory only; never persisted.
    pub compact_layout: bool,
    /// Index of the element currently at the top of the message
    /// viewport. `None` if the feed is empty.
    pub current_top_element: Option<usize>,
    /// Navigable posts in the feed. Each post groups a logical unit of
    /// content (e.g. a user message, a thought, a tool result).
    pub posts: Arc<[crate::view::elements::Post]>,
    /// Index of the post selected in vim nav mode. `None` when not in
    /// nav mode or when the feed is empty. Used by the renderer to draw
    /// the selection bracket around the selected post.
    pub selected_post: Option<usize>,
    /// Incomplete streaming content (mutable tail) — rendered in the active cell.
    pub streaming_tail: String,
    /// Input box title: `provider/model · read-only` when read-only.
    pub input_title: String,
    /// True when a provider and model are connected.
    pub has_models: bool,
    /// Plan mode active — write tools blocked until plan is approved.
    pub plan_mode: bool,
    /// Auto-approve mode active — read, edit and shell tools run without
    /// confirmation. Session-scoped (never persisted).
    pub auto_mode: bool,
    /// Context-detail pinned: idle right status shows the usage progress bar
    /// + percentage instead of the compact token text.
    pub context_detail_pinned: bool,
    /// Content of the active plan (markdown).
    pub active_plan_content: String,
    /// ID of the active plan file.
    pub active_plan_id: Option<String>,
    /// Grok-style tasks pane visibility.
    pub tasks_pane_visible: bool,
    /// Configured MCP servers for status display.
    pub mcp_servers: Arc<[crate::dialog::builders::McpServerRow]>,
    /// Show completed workers in the tasks pane (true when no workers are running).
    pub tasks_pane_show_done: bool,
    /// Open subagent detail overlay state.
    pub subagent_detail: Option<SubagentDetail>,
    /// Open feed element detail overlay state.
    pub feed_element_detail: Option<FeedElementDetail>,
    /// Lifecycle rows for the current turn's swarm workers.
    pub pattern_workers: Arc<[crate::model::PatternWorkerRow]>,
    /// Follow mode active — auto-scroll to newest content when it arrives.
    pub follow_mode: bool,
    /// Scroll margin in lines.
    pub scroll_margin: usize,
    /// Active goal state for goal pane rendering.
    pub goal_state: Option<crate::goal::GoalState>,
    /// True when the swarm circuit breaker has tripped (dispatch paused).
    pub circuit_breaker_tripped: bool,
    /// Threshold that triggered the circuit breaker (for display).
    pub circuit_breaker_threshold: u32,
}

/// Compute the index of the element currently at the top of the
/// message viewport. Returns None if the feed is empty.
pub fn compute_current_top_element(
    elements: &[crate::view::elements::Element],
    line_counts: &[usize],
    total_lines: usize,
    scroll: usize,
    visible_height: u16,
) -> Option<usize> {
    if elements.is_empty() || total_lines == 0 {
        return None;
    }
    let visible = (visible_height as usize).max(3);
    let max_scroll = total_lines.saturating_sub(visible);
    let current = scroll.min(max_scroll);
    let top_line = max_scroll.saturating_sub(current);
    // Cumulative line counts: cum[i] = sum(line_counts[0..=i]).
    let mut cum = 0usize;
    for (i, &c) in line_counts.iter().enumerate() {
        cum += c;
        if cum > top_line {
            return Some(i);
        }
    }
    Some(line_counts.len().saturating_sub(1))
}

/// Compute the index of the element currently at the bottom of the
/// message viewport. Returns None if the feed is empty.
pub fn compute_current_bottom_element(
    elements: &[crate::view::elements::Element],
    line_counts: &[usize],
    total_lines: usize,
    scroll: usize,
    visible_height: u16,
) -> Option<usize> {
    if elements.is_empty() || total_lines == 0 {
        return None;
    }
    let visible = (visible_height as usize).max(3);
    let max_scroll = total_lines.saturating_sub(visible);
    let current = scroll.min(max_scroll);
    let top_line = max_scroll.saturating_sub(current);
    let bottom_line = (top_line + visible)
        .saturating_sub(1)
        .min(total_lines.saturating_sub(1));
    let mut cum = 0usize;
    for (i, &c) in line_counts.iter().enumerate() {
        cum += c;
        if cum > bottom_line {
            return Some(i);
        }
    }
    Some(line_counts.len().saturating_sub(1))
}

impl Snapshot {
    pub fn element_count(&self) -> usize {
        self.elements.len()
    }

    pub fn visible(&self, skip: usize, take: usize) -> &[Element] {
        visible_slice(&self.elements, skip, take)
    }

    pub fn scroll_offset(&self, visible_height: usize) -> u16 {
        scroll_offset(self.total_lines, self.scroll, visible_height)
    }

    pub fn scrollbar_metrics(&self, visible_height: usize) -> (usize, usize) {
        scrollbar_metrics(self.total_lines, self.scroll, visible_height)
    }
}

/// Shared slice helper used by `AppState::visible` and `Snapshot::visible`.
pub fn visible_slice<T>(elements: &[T], skip: usize, take: usize) -> &[T] {
    let start = skip.min(elements.len());
    let end = (start + take).min(elements.len());
    &elements[start..end]
}

/// Shared scroll-offset helper used by `AppState` and `Snapshot`.
pub fn scroll_offset(total_lines: usize, scroll: usize, visible_height: usize) -> u16 {
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll = scroll.min(max_scroll);
    // Content is rendered newest-at-top: at scroll=0 (bottom), offset=max_scroll
    // shows the newest lines at the top of the visible area.
    (max_scroll - scroll).min(u16::MAX as usize) as u16
}

/// Shared scrollbar metrics helper used by `AppState` and `Snapshot`.
pub fn scrollbar_metrics(total_lines: usize, scroll: usize, visible_height: usize) -> (usize, usize) {
    if total_lines <= visible_height || visible_height == 0 {
        return (0, 0);
    }
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll = scroll.min(max_scroll);
    let position = max_scroll.saturating_sub(scroll);
    let track = visible_height as f64;
    // Match ratatui's rounding formula exactly:
    // thumb_start = round(position * track / total)
    // thumb_end   = round((position + visible_height) * track / total)
    let thumb_start = (position as f64 * track / total_lines as f64)
        .round()
        .clamp(0.0, track - 1.0) as usize;
    let thumb_end = ((position + visible_height) as f64 * track / total_lines as f64)
        .round()
        .clamp(0.0, track) as usize;
    let thumb = thumb_end.saturating_sub(thumb_start).max(1);
    (thumb, thumb_start)
}
