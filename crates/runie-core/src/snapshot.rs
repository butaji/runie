//! Immutable frame description — the UI DSL.
//! The event loop builds snapshots; the render actor draws them.
//! Zero blocking I/O in the event loop by design.

use crate::ui::elements::Element;
use std::sync::Arc;

/// Git repository info detected from current working directory.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GitInfo {
    pub repo_name: Option<String>,
    pub branch: Option<String>,
}

impl GitInfo {
    /// Format for status bar left side when turn is not active.
    /// Returns "repo/branch" when both known, "branch" when only branch known,
    /// or "folder/" when not in a git repo at all.
    pub fn format_right(&self, cwd_name: &str) -> String {
        match (&self.repo_name, &self.branch) {
            (Some(repo), Some(branch)) => format!("{}/{}", repo, branch),
            (None, Some(branch)) => branch.to_string(),
            (Some(repo), None) => format!("{}/", repo),
            (None, None) => format!("{}/", cwd_name),
        }
    }
}

#[derive(Clone, Default)]
pub struct Snapshot {
    pub elements: Arc<[Element]>,
    pub line_counts: Arc<[usize]>,
    pub total_lines: usize,
    pub input: String,
    pub cursor_pos: usize,
    pub hint_text: String,
    pub path_suggestions: Option<Vec<crate::path_complete::PathCompletion>>,
    pub path_selected: Option<usize>,
    pub turn_active: bool,
    pub spinner_frame: char,
    pub scroll: usize,
    /// Elapsed seconds since turn started. Captured at snapshot creation time.
    pub turn_elapsed_secs: Option<f64>,
    pub provider: String,
    pub model: String,
    /// Active theme name for the render actor
    pub theme_name: String,
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
    /// Index of the element currently at the top of the message
    /// viewport. `None` if the feed is empty.
    pub current_top_element: Option<usize>,
    /// Navigable posts in the feed. Each post groups a logical unit of
    /// content (e.g. a user message, a thought, a tool result).
    pub posts: Arc<[crate::ui::elements::Post]>,
    /// Index of the post selected in vim nav mode. `None` when not in
    /// nav mode or when the feed is empty. Used by the renderer to draw
    /// the selection bracket around the selected post.
    pub selected_post: Option<usize>,
}

/// Compute the index of the element currently at the top of the
/// message viewport. Returns None if the feed is empty.
pub fn compute_current_top_element(
    elements: &[crate::ui::elements::Element],
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
    elements: &[crate::ui::elements::Element],
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
        let start = skip.min(self.elements.len());
        let end = (start + take).min(self.elements.len());
        &self.elements[start..end]
    }

    pub fn scroll_offset(&self, visible_height: usize) -> u16 {
        let max_scroll = self.total_lines.saturating_sub(visible_height);
        let scroll = self.scroll.min(max_scroll);
        max_scroll.saturating_sub(scroll).min(u16::MAX as usize) as u16
    }

    pub fn scrollbar_metrics(&self, visible_height: usize) -> (usize, usize) {
        let total = self.total_lines;
        if total <= visible_height || visible_height == 0 {
            return (0, 0);
        }
        let max_scroll = total.saturating_sub(visible_height);
        let scroll = self.scroll.min(max_scroll);
        let position = max_scroll.saturating_sub(scroll);
        let track = visible_height as f64;
        // Match ratatui's rounding formula exactly:
        // thumb_start = round(position * track / total)
        // thumb_end   = round((position + visible_height) * track / total)
        let track_f = track;
        let thumb_start = (position as f64 * track_f / total as f64)
            .round()
            .clamp(0.0, track_f - 1.0) as usize;
        let thumb_end = ((position + visible_height) as f64 * track_f / total as f64)
            .round()
            .clamp(0.0, track_f) as usize;
        let thumb = thumb_end.saturating_sub(thumb_start).max(1);
        (thumb, thumb_start)
    }
}
