//! Immutable frame description — the UI DSL.
//! The event loop builds snapshots; the render actor draws them.
//! Zero blocking I/O in the event loop by design.

use crate::view::elements::Element;
use std::sync::Arc;

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

/// Which region of the TUI the mouse is currently over.
/// Computed by the TUI layer from the last known mouse position and the
/// current layout. Used for hover hints and click routing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum MouseTarget {
    /// Mouse is over the scrollable message feed.
    Feed,
    /// Mouse is over the input box area.
    Input,
    /// Mouse is over the status bar.
    StatusBar,
    /// Mouse is over the hints line.
    Hints,
    /// No known position (never tracked or terminal does not support it).
    #[default]
    Unknown,
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
    /// Active permission approval prompt for modal rendering.
    pub permission_request: Option<crate::model::PermissionRequestState>,
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
    pub posts: Arc<[crate::view::elements::Post]>,
    /// Index of the post selected in vim nav mode. `None` when not in
    /// nav mode or when the feed is empty. Used by the renderer to draw
    /// the selection bracket around the selected post.
    pub selected_post: Option<usize>,
    /// Incomplete streaming content (mutable tail) — rendered in the active cell.
    pub streaming_tail: String,
    /// Region the mouse is over (computed by the TUI before snapshot is sent
    /// to the render actor). Used for hover styling and click routing.
    pub mouse_target: MouseTarget,
    /// Element index under the mouse cursor, if the mouse is in the feed area
    /// and over a known element. `None` if mouse is elsewhere or unknown.
    pub hovered_element: Option<usize>,
    /// Last known mouse position in terminal coordinates.
    pub mouse_position: Option<(u16, u16)>,
    /// Input box title: `provider/model · read-only` when read-only.
    pub input_title: String,
    /// True when a provider and model are connected.
    pub has_models: bool,
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
    max_scroll.saturating_sub(scroll).min(u16::MAX as usize) as u16
}

/// Shared scrollbar metrics helper used by `AppState` and `Snapshot`.
pub fn scrollbar_metrics(
    total_lines: usize,
    scroll: usize,
    visible_height: usize,
) -> (usize, usize) {
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

// ─────────────────────────────────────────────────────────────────────────────
// Public mouse-target helpers (pure functions, no state needed).
// ─────────────────────────────────────────────────────────────────────────────

/// Derive which UI region the mouse is over from raw coordinates + layout.
pub fn compute_mouse_target(
    mouse_pos: Option<(u16, u16)>,
    width: u16,
    height: u16,
    input: &str,
    has_models: bool,
) -> MouseTarget {
    let (row, col) = match mouse_pos {
        Some(pos) => pos,
        None => return MouseTarget::Unknown,
    };
    if width == 0 || height == 0 {
        return MouseTarget::Unknown;
    }
    let layout = compute_layout(width, height, input, has_models);
    target_from_row(row, col, &layout)
}

struct MouseLayout {
    width: u16,
    margin: u16,
    feed_end: u16,
    input_height: u16,
}

fn compute_layout(width: u16, height: u16, input: &str, has_models: bool) -> MouseLayout {
    let margin = if width > 20 && height > 10 { 1 } else { 0 };
    let area_height = height.saturating_sub(margin * 2);
    let input_lines = if input.is_empty() {
        1
    } else {
        input.lines().count().max(1)
    };
    let input_height = (input_lines + 2).min(10) as u16;
    let feed_end = if has_models {
        margin + area_height.saturating_sub(input_height + 4)
    } else {
        margin + area_height.saturating_sub(1)
    };
    MouseLayout {
        width,
        margin,
        feed_end,
        input_height,
    }
}

fn target_from_row(row: u16, col: u16, layout: &MouseLayout) -> MouseTarget {
    if col > layout.width || row < layout.margin {
        return MouseTarget::Unknown;
    }
    if row < layout.feed_end {
        return MouseTarget::Feed;
    }
    let mut y = layout.feed_end + layout.margin + 1; // status bar
    if row < y {
        return MouseTarget::StatusBar;
    }
    y += layout.input_height;
    if row < y {
        MouseTarget::Input
    } else {
        MouseTarget::Hints
    }
}

/// Compute which element is under the mouse cursor in the feed area.
/// Returns the element index, or None if the mouse is not in the feed.
// allow: all args are orthogonal (mouse pos, dimensions, content) — refactoring would hurt clarity
#[allow(clippy::too_many_arguments)]
pub fn compute_hovered_element(
    mouse_pos: Option<(u16, u16)>,
    width: u16,
    height: u16,
    input: &str,
    elements: &[crate::view::elements::Element],
    line_counts: &[usize],
    total_lines: usize,
    has_models: bool,
) -> Option<usize> {
    if compute_mouse_target(mouse_pos, width, height, input, has_models) != MouseTarget::Feed {
        return None;
    }

    let (row, _) = mouse_pos?;
    let margin = if width > 20 && height > 10 { 1 } else { 0 };

    if elements.is_empty() || total_lines == 0 {
        return None;
    }

    let feed_top = margin;
    let content_row = row.saturating_sub(feed_top) as usize;
    let visible_height = (height.saturating_sub(margin * 2)).max(3) as usize;
    let max_scroll = total_lines.saturating_sub(visible_height);
    let top_line = max_scroll.saturating_sub(max_scroll); // feed top = 0 when scroll=0
    let target_line = top_line
        .saturating_add(content_row)
        .min(total_lines.saturating_sub(1));

    let mut cum = 0usize;
    for (i, &c) in line_counts.iter().enumerate() {
        cum += c;
        if cum > target_line {
            return Some(i);
        }
    }
    None
}
