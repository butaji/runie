//! Scrollback widget: append-only transcript with autoscroll.

use std::collections::{HashMap, HashSet};

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line as RatLine, Span};
use ratatui::widgets::{Paragraph, Widget, Wrap};

use crate::appearance;
use crate::view::PaintIntent;
use runie_core::types::ThemeKind;
pub use runie_tui_model::{
    logical_tool_member_index, logical_tool_member_index_at, project_tool_card_rows,
    tool_mode_for_line, tool_mode_override_for_line, FeedNavigation, FeedSnapshot, FeedState, Line,
    LineKind, ScrollbackMsg, ToolBlock, ToolCardKind, ToolCardPaintIntent, ToolCardRowKind,
};

// Grok reserves a visible gutter between the first assistant row and its
// right-aligned clock before wrapping the remaining response text.
const TIMESTAMP_GUTTER_SPACES: usize = 3;
// The wrapped assistant row keeps the source renderer's ten-cell overlay
// gutter and its short-clock inset when materializing a plain text row.
const ASSISTANT_TIMESTAMP_WRAPPED_RESERVATION: usize = 14;

type PhysicalRow = (LineKind, String, bool);
type PhysicalRowsWithSources = (Vec<PhysicalRow>, Vec<Option<usize>>);

/// Grok's default dense activity-group budget. A zero budget is reserved for
/// the source-compatible "no truncation" configuration.
pub const GROK_GROUP_MAX_VISIBLE: usize = 10;

/// Classify consecutive tool members into Grok dense groups. The returned
/// tuples contain `(member_index, group_size)` for each tool id; non-tool
/// entries are represented by `None`. This is deliberately pure so the actor
/// reducer, renderer, and YAML oracle can share the same grouping semantics.
pub fn dense_tool_group_members(tool_ids: &[Option<&str>]) -> Vec<Option<(usize, usize)>> {
    runie_tui_model::dense_tool_group_members(tool_ids)
}

pub trait LinePresentationExt {
    fn style(self) -> Style;
    fn style_for(self, theme: ThemeKind) -> Style;
    fn prefix(self) -> &'static str;
}

impl LinePresentationExt for LineKind {
    fn style(self) -> Style {
        self.style_for(ThemeKind::GrokNight)
    }

    fn style_for(self, theme: ThemeKind) -> Style {
        match self {
            LineKind::User => appearance::user_style_for(theme),
            // Grok uses a vertical transcript bar for assistant/reasoning
            // blocks; body text stays primary rather than green.
            LineKind::Assistant => appearance::base_style_for(theme),
            LineKind::Reasoning => {
                appearance::muted_style_for(theme).add_modifier(Modifier::DIM | Modifier::ITALIC)
            }
            LineKind::ThinkingStatus => {
                appearance::accent_style_for(theme).add_modifier(Modifier::BOLD)
            }
            LineKind::Tool => appearance::base_style_for(theme),
            LineKind::ToolRunning => appearance::accent_style_for(theme),
            LineKind::ToolError => appearance::error_style_for(theme),
            LineKind::ToolResult => appearance::success_style_for(theme),
            LineKind::ToolOutput => appearance::base_style_for(theme),
            LineKind::SessionStart => appearance::muted_style_for(theme),
            LineKind::System => appearance::muted_style_for(theme).add_modifier(Modifier::DIM),
            LineKind::Separator => appearance::muted_style_for(theme).add_modifier(Modifier::DIM),
            // Grok's settled thought/work markers use the muted token at full
            // intensity.  The collapsed thinking rail is dimmed by its own
            // accent projection; applying DIM to the whole row incorrectly
            // darkens the marker text as well.
            LineKind::TurnSummary => appearance::muted_style_for(theme),
            LineKind::CompletedAssistant => appearance::base_style_for(theme),
            LineKind::Activity => appearance::accent_style_for(theme),
        }
    }

    fn prefix(self) -> &'static str {
        runie_tui_model::LineKind::prefix(self)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scrollback {
    lines: Vec<Line>,
    /// Monotonic identity for rows created by this reducer adapter. The
    /// identity is persisted on `Line`; no second live-row ownership map is
    /// kept in the renderer.
    navigation: FeedNavigation,
}

#[allow(
    dead_code,
    reason = "legacy reducer helpers remain quarantined while all public transitions use FeedState"
)]
impl Scrollback {
    /// Build the terminal compatibility adapter from the actor-owned,
    /// renderer-independent projection. This is the only direction allowed
    /// across the model-to-widget boundary.
    pub fn from_model_snapshot(snapshot: FeedSnapshot) -> Self {
        let mut scrollback = Self::new();
        scrollback.lines = snapshot.lines;
        scrollback.navigation.scroll_offset = snapshot.scroll_offset;
        scrollback.navigation.autoscroll = snapshot.autoscroll;
        scrollback.navigation.reasoning_expanded = snapshot.reasoning_expanded;
        scrollback.navigation.activity_expanded = snapshot.activity_expanded;
        scrollback.navigation.prompt_timestamp = snapshot.prompt_timestamp;
        scrollback.navigation.follow_latest_user = snapshot.follow_latest_user;
        scrollback.navigation.theme = snapshot.theme;
        scrollback.navigation.animation_frame = snapshot.animation_frame;
        scrollback.navigation.selected_tool_id = snapshot.selected_tool_id;
        scrollback.navigation.selected_entry = snapshot.selected_entry;
        scrollback.navigation.selection_anchor = snapshot.selection_anchor;
        scrollback.navigation.selection_head = snapshot.selection_head;
        scrollback.navigation.cell_selection = snapshot.cell_selection;
        scrollback.navigation.copy_selection = snapshot.copy_selection;
        scrollback.navigation.tool_modes = snapshot.tool_modes;
        scrollback.navigation.revealed_dense_groups = snapshot.revealed_dense_groups;
        scrollback.navigation.center_revealed_entry = snapshot.center_revealed_entry;
        scrollback.navigation.workflow_headers = snapshot.workflow_headers;
        scrollback.navigation.workflow_phases = snapshot.workflow_phases;
        scrollback.navigation.tool_names = snapshot.tool_names;
        scrollback.navigation.settled_no_tool_phase = snapshot.settled_no_tool_phase;
        scrollback.navigation.live_grok_layout = snapshot.live_grok_layout;
        scrollback.navigation.next_tool_row_id = snapshot.next_tool_row_id;
        scrollback.navigation.turn_started = snapshot.turn_started;
        scrollback.navigation.assistant_stream_open = snapshot.assistant_stream_open;
        scrollback.navigation.measured_content_rows = snapshot.measured_content_rows;
        scrollback.navigation.measured_viewport_rows = snapshot.measured_viewport_rows;
        scrollback.navigation.measured_anchor_row = snapshot.measured_anchor_row;
        scrollback
    }

    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            navigation: FeedNavigation::default(),
        }
    }

    pub fn set_theme(&mut self, theme: ThemeKind) {
        self.navigation.theme = theme;
    }

    /// Delegate renderer-neutral navigation transitions to the canonical
    /// model reducer. Legacy widget callers still receive the same snapshot,
    /// but cannot create a second implementation of actor-owned navigation.
    /// Apply one explicit transcript transition. Actor implementations and
    /// compatibility callers share this reducer boundary.
    /// Compatibility entry point that reduces every transcript transition
    /// through the actor-owned renderer-neutral model. Terminal rendering is
    /// intentionally absent from this path.
    pub fn apply(&mut self, message: ScrollbackMsg) -> Option<usize> {
        let appended_at = match &message {
            ScrollbackMsg::Append(_) | ScrollbackMsg::AppendTurnSummary(_) => {
                Some(self.lines.len())
            }
            _ => None,
        };
        let mut model = FeedState {
            lines: self.lines.clone(),
            navigation: self.navigation.clone(),
        };
        model.reduce(message);
        self.lines = model.lines;
        self.navigation = model.navigation;
        appended_at
    }

    pub fn theme(&self) -> ThemeKind {
        self.navigation.theme
    }

    /// Compatibility adapter for the model-owned typed tool projection.
    pub fn tool_blocks(&self) -> Vec<ToolBlock> {
        runie_tui_model::project_tool_blocks(
            &self.lines,
            &self.navigation.tool_names,
            &self.navigation.tool_modes,
        )
    }

    #[cfg(test)]
    fn replace_tool_by_id(&mut self, tool_call_id: &str, text: String) {
        if let Some(line) = self.live_header_mut(tool_call_id) {
            line.text = text;
            return;
        }
        if let Some(line) = self.lines.iter_mut().rev().find(|line| {
            line.tool_call_id.as_deref() == Some(tool_call_id)
                && matches!(
                    line.kind,
                    LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError
                )
        }) {
            line.text = text;
        }
    }

    #[cfg(test)]
    fn finish_tool_by_id(&mut self, tool_call_id: &str, text: String) {
        if let Some(line) = self.live_header_mut(tool_call_id) {
            line.text = text;
            if line.kind == LineKind::ToolRunning {
                line.kind = LineKind::Tool;
            }
            line.settle_tool_row();
            return;
        }
        if let Some(line) = self.lines.iter_mut().rev().find(|line| {
            line.tool_call_id.as_deref() == Some(tool_call_id)
                && matches!(
                    line.kind,
                    LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError
                )
        }) {
            line.text = text;
            if line.kind == LineKind::ToolRunning {
                line.kind = LineKind::Tool;
            }
        }
    }

    #[cfg(test)]
    fn live_header_mut(&mut self, tool_call_id: &str) -> Option<&mut Line> {
        self.lines.iter_mut().rev().find(|line| {
            line.tool_row_id.is_some()
                && line.is_tool_row_active()
                && line.tool_call_id.as_deref() == Some(tool_call_id)
                && matches!(
                    line.kind,
                    LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError
                )
        })
    }

    #[cfg(test)]
    fn mark_tool_error(&mut self, tool_call_id: &str) {
        if let Some(line) = self.live_header_mut(tool_call_id) {
            line.kind = LineKind::ToolError;
            return;
        }
        // A completion message may be followed by an error marker. Resolve
        // the newest semantic row in that case; token-bearing rows still win
        // over compatibility rows because they occur later in the transcript.
        if let Some(line) = self.lines.iter_mut().rev().find(|line| {
            line.tool_call_id.as_deref() == Some(tool_call_id)
                && matches!(
                    line.kind,
                    LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError
                )
        }) {
            line.kind = LineKind::ToolError;
        }
    }

    #[cfg(test)]
    fn replace_or_append_activity(&mut self, activity: Option<String>) {
        let Some(activity) = activity else {
            return;
        };
        if let Some(line) = self.last_mut_by_kind(LineKind::Activity) {
            line.text = activity;
        } else {
            self.append(Line::new(LineKind::Activity, activity));
        }
    }

    #[cfg(test)]
    fn append_tool_start(&mut self, tool_call_id: String, header: String, kind: LineKind) {
        let row_id = self.navigation.next_tool_row_id;
        self.navigation.next_tool_row_id = self.navigation.next_tool_row_id.wrapping_add(1);
        self.append(
            Line::new(kind, header)
                .for_tool(tool_call_id)
                .for_tool_row(row_id),
        );
    }

    pub fn append(&mut self, line: Line) -> usize {
        let index = self.lines.len();
        if line.kind == LineKind::User {
            self.navigation.follow_latest_user = true;
        }
        self.lines.push(line);
        if self.navigation.autoscroll {
            // Hold offset so the tail is in view after the next render
            // (the actual clamp happens in `render` once we know area height).
            self.navigation.scroll_offset = self.lines.len();
        }
        index
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.navigation.tool_names.clear();
        self.navigation.tool_modes.clear();
        self.navigation.next_tool_row_id = 0;
        self.navigation.workflow_headers.clear();
        self.navigation.workflow_phases.clear();
        self.navigation.revealed_dense_groups.clear();
        self.navigation.center_revealed_entry = false;
        self.navigation.scroll_offset = 0;
        self.navigation.selected_tool_id = None;
        self.navigation.selected_entry = None;
        self.navigation.follow_latest_user = false;
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Borrow the lines (for tests).
    pub fn lines(&self) -> &[Line] {
        &self.lines
    }

    /// Export only immutable model facts; Ratatui rendering caches and
    /// reducer-only bookkeeping never cross the model boundary.
    #[allow(
        clippy::too_many_lines,
        reason = "compatibility snapshot rehydrates the complete renderer-neutral feed projection"
    )]
    pub fn model_snapshot(&self) -> FeedSnapshot {
        let selected_member_index = self
            .navigation
            .selected_entry
            .and_then(|entry| logical_tool_member_index_at(&self.lines, entry));
        FeedSnapshot {
            lines: self.lines.clone(),
            tool_blocks: self.tool_blocks(),
            tool_names: self.navigation.tool_names.clone(),
            tool_args: self.navigation.tool_args.clone(),
            activity_dirs: self.navigation.activity_dirs,
            activity_files: self.navigation.activity_files,
            activity_commands: self.navigation.activity_commands,
            activity_subagents: self.navigation.activity_subagents,
            activity_failures: self.navigation.activity_failures,
            autoscroll: self.navigation.autoscroll,
            scroll_offset: self.navigation.scroll_offset,
            reasoning_expanded: self.navigation.reasoning_expanded,
            activity_expanded: self.navigation.activity_expanded,
            prompt_timestamp: self.navigation.prompt_timestamp.clone(),
            follow_latest_user: self.navigation.follow_latest_user,
            selected_tool_id: self.navigation.selected_tool_id.clone(),
            selected_entry: self.navigation.selected_entry,
            selected_member_index,
            selection_anchor: self.navigation.selection_anchor,
            selection_head: self.navigation.selection_head,
            cell_selection: self.navigation.cell_selection,
            copy_selection: self.navigation.copy_selection,
            theme: self.navigation.theme,
            animation_frame: self.navigation.animation_frame,
            tool_modes: self.navigation.tool_modes.clone(),
            revealed_dense_groups: self.navigation.revealed_dense_groups.clone(),
            center_revealed_entry: self.navigation.center_revealed_entry,
            workflow_headers: self.navigation.workflow_headers.clone(),
            workflow_phases: self.navigation.workflow_phases.clone(),
            settled_no_tool_phase: self.navigation.settled_no_tool_phase,
            live_grok_layout: self.navigation.live_grok_layout,
            next_tool_row_id: self.navigation.next_tool_row_id,
            turn_started: self.navigation.turn_started,
            assistant_stream_open: self.navigation.assistant_stream_open,
            measured_content_rows: self.navigation.measured_content_rows,
            measured_viewport_rows: self.navigation.measured_viewport_rows,
            measured_anchor_row: self.navigation.measured_anchor_row,
        }
    }

    pub fn remove_kind(&mut self, kind: LineKind) {
        self.lines.retain(|line| line.kind != kind);
    }

    /// The live Grok feed drops the vertical assistant rail after a turn
    /// completes; replay fixtures retain the richer transcript rail.
    pub fn normalize_live_completed_assistants(&mut self) {
        for line in &mut self.lines {
            if line.kind == LineKind::Assistant && !line.text.is_empty() {
                line.kind = LineKind::CompletedAssistant;
            }
        }
    }

    pub fn add_live_assistant_timestamp(&mut self, width: usize) {
        // Kept as a compatibility reducer message; timestamp placement is a
        // pure view overlay in `physical_rows`, so event-owned text remains
        // immutable and does not wrap around the timestamp.
        let _ = width;
    }

    pub fn remove_empty_after(&mut self, kind: LineKind) {
        if let Some(index) = self.lines.iter().position(|line| line.kind == kind) {
            if self
                .lines
                .get(index + 1)
                .is_some_and(|line| line.text.is_empty())
            {
                self.lines.remove(index + 1);
            }
        }
    }

    pub fn normalize_activity_spacing(&mut self) {
        let Some(index) = self
            .lines
            .iter()
            .position(|line| line.kind == LineKind::Activity)
        else {
            return;
        };
        let mut cursor = index + 1;
        while cursor < self.lines.len() {
            if self.lines[cursor].kind == LineKind::System && self.lines[cursor].text.is_empty() {
                self.lines.remove(cursor);
            } else {
                cursor += 1;
            }
        }
        self.lines
            .insert(index + 1, Line::new(LineKind::Separator, ""));
    }

    /// Set Grok-compatible reasoning display mode. Collapsed mode renders
    /// only `Thought`; expanded mode renders the captured reasoning body.
    pub fn set_reasoning_expanded(&mut self, expanded: bool) {
        self.navigation.reasoning_expanded = expanded;
    }

    pub fn reasoning_expanded(&self) -> bool {
        self.navigation.reasoning_expanded
    }

    /// Set Grok-compatible grouped-tool display mode. Collapsed mode keeps
    /// the activity summary and hides member tool/output rows.
    pub fn set_activity_expanded(&mut self, expanded: bool) {
        self.navigation.activity_expanded = expanded;
    }

    pub fn set_prompt_timestamp(&mut self, timestamp: Option<String>) {
        self.navigation.prompt_timestamp = timestamp;
    }

    /// Select the production live adapter's Grok gutter geometry. Replay
    /// fixtures retain the historical wider compatibility gutter.
    pub fn set_live_grok_layout(&mut self, enabled: bool) {
        self.navigation.live_grok_layout = enabled;
    }

    pub fn activity_expanded(&self) -> bool {
        self.navigation.activity_expanded
    }

    /// Background work keeps its running bullet animated after the main turn
    /// becomes idle.
    pub fn animation_demand(&self) -> bool {
        self.lines
            .iter()
            .any(|line| line.kind == LineKind::ToolRunning)
    }

    pub fn set_tool_mode(
        &mut self,
        tool_call_id: impl Into<String>,
        mode: runie_core::types::ToolDisplayMode,
    ) {
        let tool_call_id = tool_call_id.into();
        if let Some(row_id) = self
            .lines
            .iter()
            .rev()
            .find(|line| line.tool_call_id.as_deref() == Some(tool_call_id.as_str()))
            .and_then(|line| line.tool_row_id)
        {
            self.navigation
                .tool_modes
                .insert(format!("#row:{row_id}"), mode);
        }
        self.navigation.tool_modes.insert(tool_call_id, mode);
    }

    /// Apply Grok's fold action to one selected tool block. The actor owns the
    /// transition; callers only publish the tool id as an intent.
    pub fn toggle_tool_mode(&mut self, tool_call_id: &str) {
        let read_card = self
            .navigation
            .tool_names
            .get(tool_call_id)
            .is_some_and(|name| matches!(name.as_str(), "read" | "read_file"));
        let running_generic_card = self.tool_blocks().iter().any(|block| {
            block.tool_call_id == tool_call_id
                && block.is_running
                && block.kind == ToolCardKind::Generic
        });
        let next = match self
            .navigation
            .tool_modes
            .get(tool_call_id)
            .copied()
            .unwrap_or(runie_core::types::ToolDisplayMode::Expanded)
        {
            runie_core::types::ToolDisplayMode::Collapsed => {
                if read_card || running_generic_card {
                    runie_core::types::ToolDisplayMode::Truncated
                } else {
                    runie_core::types::ToolDisplayMode::Expanded
                }
            }
            // Grok treats Truncated as an intermediate preview, and folding
            // that preview either returns to the title-only state (settled)
            // or advances to the full output view (while running).
            runie_core::types::ToolDisplayMode::Truncated => {
                if running_generic_card {
                    runie_core::types::ToolDisplayMode::Expanded
                } else {
                    runie_core::types::ToolDisplayMode::Collapsed
                }
            }
            runie_core::types::ToolDisplayMode::Expanded => {
                if running_generic_card {
                    runie_core::types::ToolDisplayMode::Truncated
                } else {
                    runie_core::types::ToolDisplayMode::Collapsed
                }
            }
        };
        self.set_tool_mode(tool_call_id, next);
    }

    pub fn selected_tool_id(&self) -> Option<&str> {
        self.navigation.selected_tool_id.as_deref()
    }

    pub fn selected_entry(&self) -> Option<usize> {
        self.navigation.selected_entry
    }

    pub fn scroll_offset(&self) -> usize {
        self.navigation.scroll_offset
    }

    /// Apply Grok's explicit Ctrl+j/Ctrl+k viewport scroll intent. The actor
    /// owns the offset and hands follow mode off to the user once scrolling
    /// begins; rendering only clamps it against measured physical rows.
    pub fn scroll_by(&mut self, lines: i32) {
        if lines == 0 {
            return;
        }
        self.navigation.detach_from_tail();
        if lines.is_negative() {
            self.navigation.scroll_offset = self
                .navigation
                .scroll_offset
                .saturating_sub(lines.unsigned_abs() as usize);
        } else {
            self.navigation.scroll_offset =
                self.navigation.scroll_offset.saturating_add(lines as usize);
        }
    }

    /// Find the index of the first line whose `text` contains the needle.
    pub fn find_first_containing(&self, needle: &str) -> Option<usize> {
        runie_tui_model::find_first_containing(&self.lines, needle)
    }

    /// Find all line indices whose `text` contains the needle.
    pub fn find_all_containing(&self, needle: &str) -> Vec<usize> {
        runie_tui_model::find_all_containing(&self.lines, needle)
    }

    /// Mutable reference to the last line of `kind`, if any.
    pub fn last_mut_by_kind(&mut self, kind: LineKind) -> Option<&mut Line> {
        self.lines.iter_mut().rev().find(|l| l.kind == kind)
    }

    pub fn line_mut(&mut self, index: usize) -> Option<&mut Line> {
        self.lines.get_mut(index)
    }

    #[allow(
        clippy::too_many_lines,
        reason = "render keeps viewport selection and cell projection together"
    )]
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        self.render_with_terminal_height(area, 0, buf);
    }

    /// Render using the full terminal height for Grok's responsive mode.
    /// `0` preserves the unmeasured compatibility behavior used by isolated
    /// widget tests; application callers should pass the outer frame height.
    #[allow(
        clippy::too_many_lines,
        clippy::cognitive_complexity,
        reason = "render keeps responsive layout, selection styling, and cell projection together"
    )]
    pub fn render_with_terminal_height(&self, area: Rect, terminal_rows: u16, buf: &mut Buffer) {
        // Wrap-aware: each Line is one logical row that may wrap to multiple
        // physical rows. We approximate by giving each line 1 "slot" plus
        // overflow based on text length and area width.
        let compact = crate::layout::grok_effective_compact(false, terminal_rows);
        let mut physical_rows = self.physical_rows(area.width as usize, compact, area.height);
        let prompt_lead_rows = if self.navigation.prompt_timestamp.is_some() {
            2
        } else {
            1
        };
        // Grok reserves one visual lead row for a submitted prompt. Live
        // Event sequences may already contain a separator, but Grok still
        // reserves a distinct lead row before the submitted prompt. Add it
        // only to the projection and never to actor-owned logical state.
        if self.navigation.follow_latest_user {
            if let Some(user_row) = physical_rows
                .iter()
                .position(|(kind, _, _)| *kind == LineKind::User)
            {
                for _ in 0..prompt_lead_rows {
                    physical_rows.insert(user_row, (LineKind::Separator, String::new(), false));
                }
            }
        }
        let total = physical_rows.len();
        let visible = area.height as usize;
        // Rendering is a pure projection of actor-owned state.  Responsive
        // clamping and selection centering are local viewport decisions and
        // must not mutate the feed actor's scroll/fold facts.
        let mut effective_scroll_offset = self.navigation.scroll_offset;
        let compact_scroll_lead =
            if total > visible + crate::layout::COMPACT_SCROLL_OVERFLOW_THRESHOLD {
                crate::layout::COMPACT_SCROLL_OVERFLOW_LEAD_ROWS
            } else {
                crate::layout::COMPACT_SCROLL_LEAD_ROWS
            };
        // Clamp scroll_offset so the tail is visible.
        if total > visible {
            let max_offset = total - visible;
            if self.navigation.autoscroll {
                effective_scroll_offset = if area.width < 50 {
                    max_offset.saturating_sub(compact_scroll_lead)
                } else {
                    max_offset
                };
            } else if effective_scroll_offset > max_offset {
                effective_scroll_offset = max_offset;
            }
        } else {
            effective_scroll_offset = 0;
        }

        if let Some(selected_text) = self
            .navigation
            .selected_entry
            .and_then(|index| self.lines.get(index).map(|line| line.text.as_str()))
        {
            if !selected_text.is_empty() {
                let measured_row = self.measured_anchor_row(area, terminal_rows);
                let selected_row = measured_row
                    .map(|row| {
                        if self.navigation.follow_latest_user {
                            let user_row = physical_rows
                                .iter()
                                .position(|(kind, _, _)| *kind == LineKind::User)
                                .unwrap_or(total);
                            row.saturating_add(if user_row <= row { prompt_lead_rows } else { 0 })
                        } else {
                            row
                        }
                    })
                    .or_else(|| {
                        physical_rows
                            .iter()
                            .position(|(_, text, _)| text.contains(selected_text))
                    });
                if let Some(selected_row) = selected_row {
                    if self.navigation.center_revealed_entry {
                        let max_offset = total.saturating_sub(visible);
                        effective_scroll_offset =
                            selected_row.saturating_sub(visible / 2).min(max_offset);
                    } else if selected_row < effective_scroll_offset {
                        effective_scroll_offset = selected_row;
                    } else if selected_row >= effective_scroll_offset + visible {
                        effective_scroll_offset = selected_row.saturating_sub(visible - 1);
                    }
                }
            }
        }
        let start = if self.navigation.follow_latest_user {
            physical_rows
                .iter()
                .rposition(|(kind, _, _)| *kind == LineKind::User)
                .map(|last_user_row| {
                    let mut user_row = last_user_row;
                    while user_row > 0 && physical_rows[user_row - 1].0 == LineKind::User {
                        user_row -= 1;
                    }
                    let lead = physical_rows[..user_row]
                        .iter()
                        .rev()
                        .take_while(|(_, text, _)| text.is_empty())
                        .count()
                        .min(prompt_lead_rows);
                    let anchored = user_row.saturating_sub(lead);
                    // Keep a newly submitted prompt at the top while the
                    // response fits. Once incoming content outgrows the
                    // viewport, follow the tail so new output remains visible.
                    let incoming_rows = total.saturating_sub(anchored.saturating_add(1));
                    if incoming_rows > visible {
                        total.saturating_sub(visible)
                    } else {
                        anchored
                    }
                })
                .unwrap_or(effective_scroll_offset)
        } else {
            effective_scroll_offset
        };
        let end = (start + visible).min(total);
        let selected_non_tool_text = self.navigation.selected_entry.and_then(|index| {
            self.lines.get(index).and_then(|line| {
                if line.tool_call_id.is_none() {
                    Some(line.text.as_str())
                } else {
                    None
                }
            })
        });
        let selected_tool_ids = self.selected_tool_group_ids();
        let selected_tool_keys = self
            .lines
            .iter()
            .filter_map(|line| {
                line.tool_call_id
                    .as_ref()
                    .filter(|id| selected_tool_ids.contains(*id))
                    .map(|_| (line.kind, line.text.clone()))
            })
            .collect::<Vec<_>>();

        if start >= end {
            // Nothing to render. Avoid passing an empty slice to ratatui's
            // Paragraph/Line, which can panic on some versions.
            return;
        }

        let mut selected_non_tool_row = None;
        let mut selected_tool_rows = Vec::new();
        let mut selected_tool_wrap_active = false;
        for (row, (kind, text, code_row)) in physical_rows[start..end].iter().enumerate() {
            let line = if *code_row {
                styled_code_line(text, self.navigation.theme)
            } else {
                styled_line_for(*kind, text, self.navigation.theme)
            };
            let mut line = line;
            let occurrence = physical_rows[..start + row]
                .iter()
                .filter(|(previous_kind, previous_text, _)| {
                    previous_kind == kind && previous_text == text
                })
                .count();
            if let Some(intent) = self.tool_paint_intent(*kind, text, occurrence) {
                let paint = match intent {
                    ToolCardPaintIntent::Header => PaintIntent::Base,
                    ToolCardPaintIntent::Running => PaintIntent::Accent,
                    ToolCardPaintIntent::Content => PaintIntent::Base,
                    ToolCardPaintIntent::Success => PaintIntent::Success,
                    ToolCardPaintIntent::Error => PaintIntent::Error,
                    ToolCardPaintIntent::Muted => PaintIntent::Muted,
                };
                // Structured metadata already carries source-backed span
                // roles (muted key, primary value, muted location/score).
                // Do not flatten those spans with the row-level muted intent.
                if !(paint == PaintIntent::Muted && is_structured_metadata_text(text)) {
                    let semantic_style = appearance::style_for_intent(self.navigation.theme, paint);
                    for span in &mut line.spans {
                        if let Some(foreground) = semantic_style.fg {
                            span.style = span.style.fg(foreground);
                        }
                    }
                }
            }
            if self.navigation.live_grok_layout && *kind == LineKind::User {
                let panel_background =
                    appearance::panel_background_style_for(self.navigation.theme)
                        .bg
                        .expect("panel background color");
                for span in &mut line.spans {
                    // Grok paints the user panel background across the row but
                    // does not emit an explicit foreground for its trailing
                    // blank cells. Clearing the span foreground lets the
                    // terminal default carry through the panel fill while the
                    // theme token still owns the background.
                    span.style.fg = None;
                    span.style.bg = Some(panel_background);
                }
                // Ratatui fills the remainder of a paragraph from its last
                // span. Emit one reset-foreground cell so the fill matches
                // Grok's background-only trailing panel cells.
                line.spans.push(Span::styled(
                    " ",
                    Style::default().fg(Color::Reset).bg(panel_background),
                ));
            }
            if self.navigation.live_grok_layout
                && matches!(*kind, LineKind::ToolOutput | LineKind::ToolResult)
                && (text.starts_with("    ")
                    || text.starts_with("Result ")
                    || text.trim_start().starts_with("Sources:")
                    || text.trim_start().starts_with("status:")
                    || text.trim_start().starts_with("content_type:")
                    || text.trim_start().starts_with("title:")
                    || (text
                        .trim_start()
                        .as_bytes()
                        .first()
                        .is_some_and(u8::is_ascii_digit)
                        && text.contains(". ")))
            {
                // Grok's memory/search rows are panel rows, not merely
                // panel-colored text. Make the trailing cells explicit so a
                // narrow or wide terminal cannot inherit the surrounding
                // feed background from Paragraph's implicit fill.
                let panel_background =
                    appearance::panel_background_style_for(self.navigation.theme)
                        .bg
                        .expect("panel background color");
                let remaining = area.width.saturating_sub(line.width() as u16) as usize;
                if remaining > 0 {
                    line.spans.push(Span::styled(
                        " ".repeat(remaining),
                        Style::default().fg(Color::Reset).bg(panel_background),
                    ));
                }
            }
            if self.navigation.live_grok_layout && *kind == LineKind::CompletedAssistant {
                let assistant = appearance::assistant_body_style_for(self.navigation.theme);
                for span in &mut line.spans {
                    span.style = span.style.fg(assistant.fg.expect("assistant color"));
                }
            }
            let comparable_text = text
                .strip_prefix("⌄ ")
                .or_else(|| text.strip_prefix("◆ "))
                .or_else(|| text.strip_prefix("› "))
                .unwrap_or(text);
            let selected_tool_row =
                selected_tool_keys
                    .iter()
                    .any(|(selected_kind, selected_text)| {
                        *selected_kind == *kind && selected_text == comparable_text
                    });
            let selected_tool_continuation = selected_tool_wrap_active
                && matches!(
                    *kind,
                    LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError
                )
                && !text.starts_with("⌄ ")
                && !text.starts_with("◆ ")
                && !text.starts_with("› ");
            let selected_tool_row = selected_tool_row || selected_tool_continuation;
            selected_tool_wrap_active = selected_tool_row
                && matches!(
                    *kind,
                    LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError
                );
            let selected_row = selected_tool_row
                || text.starts_with("› ")
                || text.starts_with("⌄ ")
                || selected_non_tool_text.is_some_and(|value| text.contains(value));
            if selected_row {
                let selected_style = appearance::selected_style_for(self.navigation.theme);
                for span in &mut line.spans {
                    span.style = span.style.patch(selected_style);
                }
            }
            Paragraph::new(line).wrap(Wrap { trim: false }).render(
                Rect {
                    x: area.x,
                    y: area.y + row as u16,
                    width: area.width,
                    height: 1,
                },
                buf,
            );
            if self.navigation.live_grok_layout
                && matches!(*kind, LineKind::User | LineKind::CompletedAssistant)
            {
                let timestamp = self
                    .navigation
                    .prompt_timestamp
                    .as_deref()
                    .unwrap_or_default();
                let body_color = if *kind == LineKind::User {
                    // Grok leaves the user-panel body at terminal-default
                    // foreground; only the prompt key is explicitly colored.
                    Color::Reset
                } else {
                    appearance::assistant_body_style_for(self.navigation.theme)
                        .fg
                        .expect("assistant color")
                };
                let key_color = appearance::footer_key_style_for(self.navigation.theme)
                    .fg
                    .expect("key color");
                let muted_color = appearance::muted_style_for(self.navigation.theme)
                    .fg
                    .expect("muted color");
                let row_y = area.y + row as u16;
                let symbols = (area.x..area.x.saturating_add(area.width))
                    .filter_map(|x| buf.cell((x, row_y)).map(|cell| cell.symbol()))
                    .collect::<String>();
                let timestamp_start = (!timestamp.is_empty())
                    .then(|| symbols.rfind(timestamp))
                    .flatten();
                let pointer = (area.x..area.x.saturating_add(area.width)).find(|x| {
                    buf.cell((*x, row_y))
                        .is_some_and(|cell| cell.symbol() == "❯")
                });
                let body_end = if let Some(start) = pointer {
                    Some(
                        (start + 1..area.x.saturating_add(area.width))
                            .rev()
                            .find(|x| {
                                buf.cell((*x, row_y))
                                    .is_some_and(|cell| !cell.symbol().trim().is_empty())
                            })
                            .unwrap_or(start),
                    )
                } else if *kind == LineKind::CompletedAssistant {
                    // Assistant rows have no user pointer. Their themed body
                    // extends through the final non-empty cell before the
                    // right-aligned timestamp; otherwise the overlay would
                    // reset every assistant body cell to the terminal default.
                    let end = timestamp_start
                        .map(|start| start.saturating_sub(1))
                        .unwrap_or(area.width as usize)
                        .min(area.width.saturating_sub(1) as usize);
                    (area.x..=area.x.saturating_add(end as u16))
                        .rev()
                        .find(|x| {
                            buf.cell((*x, row_y))
                                .is_some_and(|cell| !cell.symbol().trim().is_empty())
                        })
                } else {
                    None
                };
                for x in area.x..area.x.saturating_add(area.width) {
                    let index = usize::from(x - area.x);
                    let color = if timestamp_start.is_some_and(|start| index >= start) {
                        muted_color
                    } else if *kind == LineKind::User
                        && pointer.is_some_and(|pointer| x == pointer || x == pointer + 1)
                    {
                        key_color
                    } else if *kind == LineKind::CompletedAssistant && index < 3 {
                        Color::Reset
                    } else if body_end.is_some_and(|end| {
                        x <= end
                            && (*kind != LineKind::User
                                || pointer.is_some_and(|pointer| x > pointer))
                    }) {
                        body_color
                    } else {
                        Color::Reset
                    };
                    if let Some(cell) = buf.cell_mut((x, row_y)) {
                        cell.set_style(cell.style().fg(color));
                    }
                }
            }
            if *kind == LineKind::User {
                let user_style = appearance::panel_background_style_for(self.navigation.theme);
                for column in area.x..area.x.saturating_add(area.width) {
                    if let Some(cell) = buf.cell_mut((column, area.y + row as u16)) {
                        cell.set_style(cell.style().patch(user_style));
                    }
                }
            }
            // The submitted-prompt lead row is part of Grok's panel surface,
            // even though it is represented as a renderer-only separator.
            // Keep the logical feed unchanged and project the panel token to
            // the adjacent separator row.
            let absolute_row = start + row;
            let panel_separator = *kind == LineKind::Separator
                && text.is_empty()
                && (physical_rows
                    .get(absolute_row.saturating_sub(1))
                    .is_some_and(|(neighbor, _, _)| *neighbor == LineKind::User)
                    || physical_rows
                        .get(absolute_row + 1)
                        .is_some_and(|(neighbor, _, _)| *neighbor == LineKind::User));
            if panel_separator {
                let user_style = appearance::panel_background_style_for(self.navigation.theme);
                for column in area.x..area.x.saturating_add(area.width) {
                    if let Some(cell) = buf.cell_mut((column, area.y + row as u16)) {
                        cell.set_style(cell.style().patch(user_style));
                    }
                }
            }
            if matches!(*kind, LineKind::ToolOutput | LineKind::ToolResult)
                && (text.starts_with('+') || text.starts_with('-'))
            {
                let diff_style = if text.starts_with('+') {
                    appearance::diff_insert_style_for(self.navigation.theme)
                } else {
                    appearance::diff_delete_style_for(self.navigation.theme)
                };
                for column in area.x..area.x.saturating_add(area.width) {
                    if let Some(cell) = buf.cell_mut((column, area.y + row as u16)) {
                        cell.set_style(cell.style().patch(diff_style));
                    }
                }
            }
            if selected_row {
                let selected_style = appearance::selected_style_for(self.navigation.theme);
                for column in area.x..area.x.saturating_add(area.width) {
                    if let Some(cell) = buf.cell_mut((column, area.y + row as u16)) {
                        cell.set_style(cell.style().patch(selected_style));
                    }
                }
                if selected_non_tool_text.is_some_and(|value| text.contains(value)) {
                    selected_non_tool_row = Some(area.y + row as u16);
                }
                if selected_tool_row {
                    selected_tool_rows.push(area.y + row as u16);
                }
            }
        }
        if let (Some(&top), Some(&bottom)) = (selected_tool_rows.first(), selected_tool_rows.last())
        {
            let border_style = appearance::selected_border_style_for(self.navigation.theme);
            let left = area.x;
            let right = area.x + area.width.saturating_sub(1);
            for y in top..=bottom {
                if let Some(cell) = buf.cell_mut((left, y)) {
                    if cell.symbol().trim().is_empty() {
                        cell.set_symbol("│").set_style(border_style);
                    }
                }
                if right > left {
                    if let Some(cell) = buf.cell_mut((right, y)) {
                        cell.set_symbol("│").set_style(border_style);
                    }
                }
            }
            if top > area.y {
                for x in left..=right {
                    if let Some(cell) = buf.cell_mut((x, top - 1)) {
                        cell.set_symbol(if x == left {
                            "┌"
                        } else if x == right {
                            "┐"
                        } else {
                            "─"
                        })
                        .set_style(border_style);
                    }
                }
            }
            let bottom_border = bottom.saturating_add(1);
            if bottom_border < area.y.saturating_add(area.height) {
                for x in left..=right {
                    if let Some(cell) = buf.cell_mut((x, bottom_border)) {
                        cell.set_symbol(if x == left {
                            "└"
                        } else if x == right {
                            "┘"
                        } else {
                            "─"
                        })
                        .set_style(border_style);
                    }
                }
            }
        }
        if let Some(inner_y) = selected_non_tool_row {
            let border_style = appearance::selected_border_style_for(self.navigation.theme);
            let left = area.x;
            let right = area.x + area.width.saturating_sub(1);
            if let Some(cell) = buf.cell_mut((left, inner_y)) {
                cell.set_symbol("│").set_style(border_style);
            }
            if right > left {
                if let Some(cell) = buf.cell_mut((right, inner_y)) {
                    cell.set_symbol("│").set_style(border_style);
                }
            }
            if inner_y > area.y {
                let top = inner_y - 1;
                for x in left..=right {
                    if let Some(cell) = buf.cell_mut((x, top)) {
                        cell.set_symbol(if x == left {
                            "┌"
                        } else if x == right {
                            "┐"
                        } else {
                            "─"
                        })
                        .set_style(border_style);
                    }
                }
            }
            let bottom = inner_y.saturating_add(1);
            if bottom < area.y.saturating_add(area.height) {
                for x in left..=right {
                    if let Some(cell) = buf.cell_mut((x, bottom)) {
                        cell.set_symbol(if x == left {
                            "└"
                        } else if x == right {
                            "┘"
                        } else {
                            "─"
                        })
                        .set_style(border_style);
                    }
                }
            }
        }
    }

    /// Measure the same physical rows used by rendering, without producing
    /// terminal output. The result is suitable for `LayoutMeasured` delivery.
    pub fn measured_content_rows(&self, area: Rect, terminal_rows: u16) -> usize {
        let compact = crate::layout::grok_effective_compact(false, terminal_rows);
        self.physical_rows(area.width as usize, compact, area.height)
            .len()
    }

    /// Return the selected member's physical row in the same projection used
    /// by rendering. This is the anchor identity sent to the feed actor.
    pub fn measured_anchor_row(&self, area: Rect, terminal_rows: u16) -> Option<usize> {
        let selected_index = self.navigation.selected_entry?;
        let selected = self.lines.get(selected_index)?;
        let text = selected.text.as_str();
        if text.is_empty() {
            return None;
        }
        let compact = crate::layout::grok_effective_compact(false, terminal_rows);
        let (physical_rows, sources) =
            self.physical_rows_with_sources(area.width as usize, compact, area.height);
        if let Some(row) = sources
            .iter()
            .position(|source| *source == Some(selected_index))
        {
            return Some(row);
        }
        // Physical rows are a rendered projection and may contain duplicate
        // text. Preserve the selected logical entry's identity by selecting
        // the same occurrence among projected rows instead of taking the
        // first text match.
        let anchor_probe: String = text.chars().take(8).collect();
        let occurrence = self.lines[..selected_index]
            .iter()
            .filter(|line| line.kind == selected.kind && line.text.contains(&anchor_probe))
            .count();
        // A wrapped physical row contains only a fragment of the logical
        // text. Use a leading fragment as the identity probe, while keeping
        // the full text for short rows; occurrence selection still prevents
        // duplicate logical rows from collapsing onto the first match.
        physical_rows
            .iter()
            .enumerate()
            .filter(|(_, (kind, candidate, _))| {
                *kind == selected.kind && candidate.contains(&anchor_probe)
            })
            .nth(occurrence)
            .map(|(index, _)| index)
    }

    #[allow(
        clippy::too_many_lines,
        reason = "semantic paint lookup keeps source identity and theme intent together"
    )]
    fn tool_paint_intent(
        &self,
        kind: LineKind,
        text: &str,
        occurrence: usize,
    ) -> Option<ToolCardPaintIntent> {
        if !matches!(
            kind,
            LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError | LineKind::ToolOutput
        ) {
            return None;
        }
        let line = self
            .lines
            .iter()
            .filter(|line| {
                line.kind == kind
                    && (line.text == text
                        || (!text.trim().is_empty() && line.text.contains(text.trim())))
            })
            .nth(occurrence)?;
        let id = line.tool_call_id.as_deref()?;
        let tool_row_id = line.tool_row_id;
        let line_index = self
            .lines
            .iter()
            .position(|candidate| std::ptr::eq(candidate, line))?;
        let member_index = logical_tool_member_index_at(&self.lines, line_index)?;
        let rows = project_tool_card_rows(
            &self.lines,
            &self.navigation.tool_names,
            &self.navigation.tool_modes,
        );
        rows.into_iter()
            .find(|row| {
                row.tool_call_id == id
                    && row.tool_row_id == tool_row_id
                    && row.text == text
                    && row.member_index == member_index
            })
            .map(|row| row.paint_intent())
    }

    #[allow(
        clippy::cognitive_complexity,
        clippy::too_many_lines,
        reason = "physical row projection keeps fold, markdown, and wrapping rules together"
    )]
    fn physical_rows(
        &self,
        width: usize,
        compact: bool,
        available_height: u16,
    ) -> Vec<(LineKind, String, bool)> {
        self.physical_rows_with_sources(width, compact, available_height)
            .0
    }

    /// Build physical rows together with the logical feed line that produced
    /// each row. Synthetic layout rows retain the current logical source so
    /// selection/measurement can use one identity across wrapping.
    #[allow(
        clippy::cognitive_complexity,
        clippy::too_many_lines,
        clippy::type_complexity,
        reason = "physical row projection keeps fold, markdown, and wrapping rules together"
    )]
    fn physical_rows_with_sources(
        &self,
        width: usize,
        compact: bool,
        available_height: u16,
    ) -> PhysicalRowsWithSources {
        let mut rows = Vec::new();
        let mut sources = Vec::new();
        let mut code_block = false;
        let mut truncated_output = HashSet::new();
        let mut preview_output_totals: HashMap<String, usize> = HashMap::new();
        let mut preview_output_seen: HashMap<String, usize> = HashMap::new();
        for row in project_tool_card_rows(
            &self.lines,
            &self.navigation.tool_names,
            &self.navigation.tool_modes,
        ) {
            if row.row_kind == ToolCardRowKind::Content
                && matches!(row.card_kind, ToolCardKind::Read | ToolCardKind::Execute)
            {
                *preview_output_totals.entry(row.tool_call_id).or_default() += 1;
            }
        }
        let dense_groups = self.dense_tool_groups();
        let mut emitted_dense_headers = HashSet::new();
        let mut user_vpad_emitted = false;
        let mut skip_full_user_separator = false;
        for (line_index, line) in self.lines.iter().enumerate() {
            let line_start = rows.len();
            // Grok keeps one blank row between the settled thought summary
            // and the completed assistant body in the live no-tool turn. The
            // separator is a pure physical-row rule: actor-owned logical
            // lines remain unchanged and replay fixtures keep their source
            // event sequence.
            if matches!(
                line.kind,
                LineKind::CompletedAssistant | LineKind::Assistant
            ) && rows
                .last()
                .is_some_and(|(kind, _, _)| *kind == LineKind::TurnSummary)
            {
                rows.push((LineKind::Separator, String::new(), false));
            }
            if width >= 50
                && self.navigation.settled_no_tool_phase
                && line.kind == LineKind::Separator
                && self
                    .lines
                    .get(line_index + 1)
                    .is_some_and(|next| next.kind == LineKind::TurnSummary)
            {
                continue;
            }
            if width < 50
                && matches!(line.kind, LineKind::System | LineKind::Separator)
                && line.text.is_empty()
                && self
                    .lines
                    .get(line_index + 1)
                    .is_some_and(|next| next.kind == LineKind::TurnSummary)
            {
                continue;
            }
            if width >= 70
                && skip_full_user_separator
                && matches!(line.kind, LineKind::System | LineKind::Separator)
                && line.text.is_empty()
            {
                skip_full_user_separator = false;
                continue;
            }
            if line.has_vpad()
                && width >= 70
                && available_height >= 3
                && !compact
                && !user_vpad_emitted
            {
                // Grok's prompt block enables vertical padding in full mode;
                // the narrow pager variant suppresses it.
                rows.push((LineKind::System, String::new(), false));
                user_vpad_emitted = true;
                skip_full_user_separator = true;
            }
            if width >= 50 && line.kind == LineKind::TurnSummary {
                rows.push((LineKind::System, String::new(), false));
            }
            let tool_mode = tool_mode_override_for_line(line, &self.navigation.tool_modes);
            if self.navigation.activity_expanded {
                if let Some(tool_id) = line.tool_call_id.as_deref() {
                    if let Some((member_index, group_size)) = dense_groups.get(tool_id) {
                        let hidden = group_size.saturating_sub(GROK_GROUP_MAX_VISIBLE);
                        let group_revealed =
                            self.dense_group_anchor_for(tool_id).is_some_and(|anchor| {
                                self.navigation.revealed_dense_groups.contains(&anchor)
                            });
                        if !group_revealed && hidden > 0 && *member_index < hidden {
                            if emitted_dense_headers.insert(tool_id.to_owned()) {
                                rows.push((
                                    LineKind::Activity,
                                    format!("╶╶ {} more", hidden.saturating_sub(1)),
                                    false,
                                ));
                            }
                            sources.extend(std::iter::repeat_n(
                                Some(line_index),
                                rows.len().saturating_sub(line_start),
                            ));
                            continue;
                        }
                    }
                }
            }
            if matches!(
                tool_mode,
                Some(runie_core::types::ToolDisplayMode::Collapsed)
            ) && matches!(
                line.kind,
                LineKind::ToolOutput | LineKind::ToolResult | LineKind::ToolError
            ) {
                continue;
            }
            if matches!(
                tool_mode,
                Some(runie_core::types::ToolDisplayMode::Truncated)
            ) && matches!(
                line.kind,
                LineKind::ToolOutput | LineKind::ToolResult | LineKind::ToolError
            ) {
                let Some(tool_id) = line.tool_call_id.as_ref() else {
                    continue;
                };
                let is_read = self
                    .navigation
                    .tool_names
                    .get(tool_id)
                    .is_some_and(|name| matches!(name.as_str(), "read" | "read_file"));
                let is_execute =
                    self.navigation.tool_names.get(tool_id).is_some_and(|name| {
                        matches!(name.as_str(), "bash" | "shell" | "exec" | "run")
                    });
                if is_read || is_execute {
                    let seen = preview_output_seen.entry(tool_id.clone()).or_default();
                    let total = preview_output_totals
                        .get(tool_id)
                        .copied()
                        .unwrap_or_default();
                    let position = *seen;
                    *seen += 1;
                    let (first_lines, last_lines) = if is_read { (5, 3) } else { (2, 3) };
                    if total > first_lines + last_lines {
                        if position == first_lines {
                            let text = if is_execute {
                                format!("… +{} lines", total - first_lines - last_lines)
                            } else {
                                "…".to_owned()
                            };
                            rows.push((LineKind::ToolOutput, text, false));
                        }
                        if position >= first_lines && position < total.saturating_sub(last_lines) {
                            continue;
                        }
                    }
                } else if !truncated_output.insert(tool_id.clone()) {
                    continue;
                }
            }
            if !self.navigation.activity_expanded
                && matches!(
                    line.kind,
                    LineKind::Tool
                        | LineKind::ToolRunning
                        | LineKind::ToolOutput
                        | LineKind::ToolResult
                        | LineKind::ToolError
                )
                && line.text != "session_start"
                && line
                    .tool_call_id
                    .as_deref()
                    .and_then(|id| dense_groups.get(id))
                    .is_some_and(|(_, size)| *size > 1)
                && self.navigation.selected_tool_id.as_deref() != line.tool_call_id.as_deref()
            {
                continue;
            }
            let selected = line
                .tool_call_id
                .as_ref()
                .is_some_and(|id| self.navigation.selected_tool_id.as_ref() == Some(id));
            let source = if line.kind == LineKind::Reasoning && !self.navigation.reasoning_expanded
            {
                "Thought".to_owned()
            } else {
                line.text.clone()
            };
            let fence = line.kind == LineKind::Assistant && is_fence(&source);
            let parts: Vec<_> = source.split('\n').collect();
            for (index, part) in parts.iter().enumerate() {
                let prefix = if line.kind == LineKind::TurnSummary && width >= 50 {
                    if line.text.contains("◆ Thought") {
                        // Grok's settled thought block owns its diamond and
                        // uses the transcript gutter; the activity rail is
                        // reserved for grouped tool rows.
                        "     "
                    } else if width < 70 || self.navigation.live_grok_layout {
                        "   "
                    } else {
                        "     "
                    }
                } else if width < 70
                    && matches!(
                        line.kind,
                        LineKind::Assistant | LineKind::Reasoning | LineKind::ThinkingStatus
                    )
                {
                    "┃"
                } else if selected
                    && matches!(
                        line.kind,
                        LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError
                    )
                {
                    match tool_mode {
                        Some(runie_core::types::ToolDisplayMode::Collapsed) => "› ",
                        _ => "⌄ ",
                    }
                } else if line.kind == LineKind::ToolRunning {
                    runie_tui_model::running_bullet(self.navigation.animation_frame)
                } else {
                    line.kind.prefix()
                };
                let mut text = if part.is_empty() {
                    String::new()
                } else if index == 0 {
                    format!("{prefix}{part}")
                } else {
                    format!("{}{}", " ".repeat(prefix.chars().count()), part)
                };
                if line.kind == LineKind::CompletedAssistant && index == 0 {
                    if let Some(timestamp) = self.navigation.prompt_timestamp.as_deref() {
                        let timestamp_width = timestamp.chars().count();
                        let target =
                            width.saturating_sub(timestamp_width + TIMESTAMP_GUTTER_SPACES);
                        if text.chars().count() > target {
                            let chars: Vec<_> = text.chars().collect();
                            let mut split = target.min(chars.len());
                            while split > 0 && split < chars.len() && !chars[split].is_whitespace()
                            {
                                split -= 1;
                            }
                            let head: String = chars[..split].iter().collect();
                            let rest: String = chars[split..].iter().collect();
                            rows.push((
                                LineKind::CompletedAssistant,
                                format!(
                                    "{head}{:>reserved$}",
                                    timestamp,
                                    reserved = ASSISTANT_TIMESTAMP_WRAPPED_RESERVATION
                                ),
                                false,
                            ));
                            runie_tui_model::append_wrapped_words(
                                &mut rows,
                                line.kind,
                                format!(
                                    "{}{}",
                                    " ".repeat(line.kind.prefix().chars().count()),
                                    rest.trim_start()
                                ),
                                width.saturating_sub(timestamp_width + TIMESTAMP_GUTTER_SPACES + 3),
                            );
                            continue;
                        }
                        // Grok leaves the timestamp two cells closer to the
                        // right edge than the generic wrapped-message
                        // padding; keeping the offset in the projection
                        // avoids letting the final `M` wrap onto a new row.
                        const ASSISTANT_TIMESTAMP_EDGE_OFFSET: usize = 2;
                        let padding = width
                            .saturating_sub(text.chars().count() + timestamp_width)
                            .saturating_sub(ASSISTANT_TIMESTAMP_EDGE_OFFSET);
                        text.push_str(&" ".repeat(padding));
                        text.push_str(timestamp);
                    }
                }
                if line.kind == LineKind::User
                    && index == 0
                    && self.navigation.prompt_timestamp.is_some()
                {
                    append_user_with_timestamp(
                        &mut rows,
                        text,
                        self.navigation
                            .prompt_timestamp
                            .as_deref()
                            .unwrap_or_default(),
                        width,
                    );
                } else {
                    runie_tui_model::append_wrapped(
                        &mut rows,
                        line.kind,
                        text,
                        code_block && !fence,
                        width,
                    );
                }
                if line.kind == LineKind::Assistant
                    && is_table_row(part)
                    && !is_table_separator(part)
                    && parts.get(index + 1).is_none_or(|next| !is_table_row(next))
                {
                    let border_prefix = if index == 0 {
                        prefix.to_owned()
                    } else {
                        " ".repeat(prefix.chars().count())
                    };
                    let border = format!(
                        "{}{}",
                        border_prefix,
                        runie_tui_model::table_bottom_border(part)
                    );
                    runie_tui_model::append_wrapped(&mut rows, line.kind, border, false, width);
                }
            }
            if fence {
                code_block = !code_block;
            }
            sources.extend(std::iter::repeat_n(
                Some(line_index),
                rows.len().saturating_sub(line_start),
            ));
        }
        (rows, sources)
    }

    /// Build the ordered, consecutive tool-member groups used by Grok's
    /// `N more` projection. Outputs remain attached to their member id and
    /// therefore disappear with that member instead of consuming budget.
    fn dense_tool_groups(&self) -> HashMap<String, (usize, usize)> {
        let mut groups = HashMap::new();
        let mut members = Vec::new();
        let mut flush = |members: &mut Vec<String>| {
            let size = members.len();
            for (index, id) in members.drain(..).enumerate() {
                groups.insert(id, (index, size));
            }
        };
        for line in &self.lines {
            let is_member = matches!(
                line.kind,
                LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError
            ) && line.tool_call_id.is_some();
            if is_member {
                if let Some(id) = &line.tool_call_id {
                    if !members.iter().any(|member| member == id) {
                        members.push(id.clone());
                    }
                }
            } else if !matches!(
                line.kind,
                LineKind::Activity | LineKind::ToolOutput | LineKind::ToolResult
            ) {
                flush(&mut members);
            }
        }
        flush(&mut members);
        groups
    }

    fn dense_group_anchor_for(&self, selected: &str) -> Option<String> {
        let mut members = Vec::new();
        let flush = |members: &mut Vec<String>| {
            let anchor = members.first().cloned();
            let contains = members.iter().any(|id| id == selected);
            members.clear();
            contains.then_some(anchor).flatten()
        };
        for line in &self.lines {
            let is_member = matches!(
                line.kind,
                LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError
            ) && line.tool_call_id.is_some();
            if is_member {
                if let Some(id) = &line.tool_call_id {
                    if !members.iter().any(|member| member == id) {
                        members.push(id.clone());
                    }
                }
            } else if !matches!(
                line.kind,
                LineKind::Activity | LineKind::ToolOutput | LineKind::ToolResult
            ) {
                if let Some(anchor) = flush(&mut members) {
                    return Some(anchor);
                }
            }
        }
        flush(&mut members)
    }

    fn selected_tool_group_ids(&self) -> HashSet<String> {
        let Some(selected) = self.navigation.selected_tool_id.as_deref() else {
            return HashSet::new();
        };
        let Some(anchor) = self.dense_group_anchor_for(selected) else {
            return HashSet::from([selected.to_owned()]);
        };
        let mut group = HashSet::new();
        let mut in_group = false;
        for line in &self.lines {
            let is_member = matches!(
                line.kind,
                LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError
            ) && line.tool_call_id.is_some();
            if is_member {
                if let Some(id) = &line.tool_call_id {
                    if id == &anchor {
                        in_group = true;
                    }
                    if in_group {
                        group.insert(id.clone());
                    }
                }
            } else if !matches!(
                line.kind,
                LineKind::Activity | LineKind::ToolOutput | LineKind::ToolResult
            ) && in_group
            {
                break;
            }
        }
        group
    }
}

fn append_user_with_timestamp(
    rows: &mut Vec<(LineKind, String, bool)>,
    text: String,
    timestamp: &str,
    width: usize,
) {
    runie_tui_model::append_user_with_timestamp(rows, text, timestamp, width)
}

/// Render the small CommonMark subset that is visible in Grok's normal
/// transcript: headings, bullets, inline emphasis, and inline code. Keeping
/// this at the widget boundary means replay events remain the single source
/// of truth and no test needs a terminal process.
fn is_fence(text: &str) -> bool {
    runie_tui_model::is_fence(text)
}

fn styled_code_line(text: &str, theme: ThemeKind) -> RatLine<'static> {
    RatLine::from(Span::styled(
        text.to_owned(),
        appearance::base_style_for(theme).add_modifier(Modifier::DIM),
    ))
}

#[allow(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    reason = "semantic feed card styling keeps Grok header variants together"
)]
#[cfg(test)]
fn styled_line(kind: LineKind, text: &str) -> RatLine<'static> {
    styled_line_for(kind, text, ThemeKind::GrokNight)
}

#[allow(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    reason = "semantic feed card styling keeps Grok header variants together"
)]
fn is_structured_metadata_text(text: &str) -> bool {
    let trimmed = text.trim_start();
    text.contains("(score:")
        || trimmed.starts_with("Sources:")
        || trimmed.starts_with("status:")
        || trimmed.starts_with("content_type:")
        || trimmed.starts_with("title:")
        || trimmed.split_once(". ").is_some_and(|(index, rest)| {
            index.chars().all(|ch| ch.is_ascii_digit()) && rest.contains("(score:")
        })
}

#[allow(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    reason = "semantic feed card styling keeps Grok header variants together"
)]
fn styled_line_for(kind: LineKind, text: &str, theme: ThemeKind) -> RatLine<'static> {
    let style = kind.style_for(theme);
    if kind == LineKind::User {
        let pointer = text.find('❯');
        if let Some(pointer) = pointer {
            let body_start = pointer + '❯'.len_utf8();
            return RatLine::from(vec![
                Span::styled(text[..body_start].to_owned(), style),
                Span::styled(text[body_start..].to_owned(), style),
            ]);
        }
    }
    if kind == LineKind::TurnSummary && text.contains("◆ Thought") {
        return styled_thought_summary(text, style, theme);
    }
    if matches!(
        kind,
        LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError
    ) {
        let (header_start, marker_len) = text
            .find("◆ ")
            .map(|start| (start, "◆ ".len()))
            .or_else(|| text.find("› ").map(|start| (start, "› ".len())))
            .or_else(|| text.find("⌄ ").map(|start| (start, "⌄ ".len())))
            .unwrap_or((usize::MAX, 0));
        if header_start == usize::MAX {
            return RatLine::from(text.to_owned()).style(style);
        }
        let split = header_start + marker_len;
        let (prefix, body) = text.split_at(split);
        if let Some(rest) = body.strip_prefix("Workflow ") {
            let body_style = if rest.contains("◌ cancelled") {
                appearance::muted_style_for(theme).add_modifier(Modifier::DIM)
            } else {
                appearance::muted_style_for(theme)
            };
            let mut spans = vec![
                Span::styled(prefix.to_owned(), style),
                Span::styled(
                    "Workflow ".to_owned(),
                    appearance::muted_style_for(theme).add_modifier(Modifier::BOLD),
                ),
            ];
            if let Some(trail_start) = rest.rfind("  [") {
                let head = &rest[..trail_start];
                let trail = &rest[trail_start + 3..];
                let status_marker = [
                    " done in ",
                    " failed in ",
                    " cancelled after ",
                    " paused at ",
                ]
                .iter()
                .find_map(|marker| head.find(marker).map(|start| (start, marker.len())));
                if let Some((status_start, marker_len)) = status_marker {
                    let status_end = head[status_start + marker_len..]
                        .find(':')
                        .map(|offset| status_start + marker_len + offset)
                        .unwrap_or(head.len());
                    spans.push(Span::styled(head[..status_start].to_owned(), body_style));
                    spans.push(Span::styled(
                        head[status_start..status_end].to_owned(),
                        body_style.add_modifier(Modifier::DIM),
                    ));
                    spans.push(Span::styled(head[status_end..].to_owned(), body_style));
                    spans.push(Span::styled("  [", body_style));
                } else {
                    spans.push(Span::styled(format!("{head}  ["), body_style));
                }
                if let Some(trail_end) = trail.find(']') {
                    let trail_body = &trail[..trail_end];
                    for (index, phase) in trail_body.split(" · ").enumerate() {
                        if index > 0 {
                            spans.push(Span::styled(" · ".to_owned(), body_style));
                        }
                        let (phase_style, mark) = match phase.chars().last() {
                            Some('✓') => (appearance::success_style_for(theme), '✓'),
                            Some('✗') => (appearance::error_style_for(theme), '✗'),
                            Some('●') => (appearance::accent_style_for(theme), '●'),
                            _ => (body_style, '○'),
                        };
                        let phase_name = phase.strip_suffix(mark).unwrap_or(phase);
                        spans.push(Span::styled(phase_name.to_owned(), body_style));
                        spans.push(Span::styled(mark.to_string(), phase_style));
                    }
                    spans.push(Span::styled("]".to_owned(), body_style));
                    let suffix = &trail[trail_end + 1..];
                    if let Some(metadata_start) = suffix.find("  (") {
                        spans.push(Span::styled(
                            suffix[..metadata_start].to_owned(),
                            body_style,
                        ));
                        spans.push(Span::styled(
                            suffix[metadata_start..].to_owned(),
                            body_style.add_modifier(Modifier::DIM),
                        ));
                    } else {
                        spans.push(Span::styled(suffix.to_owned(), body_style));
                    }
                } else {
                    spans.push(Span::styled(trail.to_owned(), body_style));
                }
            } else {
                spans.push(Span::styled(rest.to_owned(), body_style));
            }
            return RatLine::from(spans);
        }
        for label in ["Web Search", "Memory Search", "Search Tools"] {
            if let Some(rest) = body.strip_prefix(label) {
                return RatLine::from(vec![
                    Span::styled(prefix.to_owned(), style),
                    Span::styled(label.to_owned(), style.add_modifier(Modifier::BOLD)),
                    Span::styled(rest.to_owned(), style),
                ]);
            }
        }
        if let Some(url) = body.strip_prefix("Fetch ") {
            return RatLine::from(vec![
                Span::styled(prefix.to_owned(), style),
                Span::styled("Fetch".to_owned(), style.add_modifier(Modifier::BOLD)),
                Span::styled(" ".to_owned(), style),
                Span::styled(url.to_owned(), appearance::header_path_style_for(theme)),
            ]);
        }
        for label in ["Use", "Used", "Todo"] {
            if let Some(rest) = body
                .strip_prefix(label)
                .and_then(|rest| rest.strip_prefix(' '))
            {
                return RatLine::from(vec![
                    Span::styled(prefix.to_owned(), style),
                    Span::styled(label.to_owned(), style.add_modifier(Modifier::BOLD)),
                    Span::styled(" ".to_owned(), style),
                    Span::styled(rest.to_owned(), appearance::header_path_style_for(theme)),
                ]);
            }
        }
        for label in ["Read", "List", "Edit"] {
            let Some(path) = body
                .strip_prefix(label)
                .and_then(|rest| rest.strip_prefix(' '))
            else {
                continue;
            };
            return RatLine::from(vec![
                Span::styled(prefix.to_owned(), style),
                Span::styled(label.to_owned(), style.add_modifier(Modifier::BOLD)),
                Span::styled(" ", style),
                Span::styled(path.to_owned(), appearance::header_path_style_for(theme)),
            ]);
        }
        if let Some(search_body) = body.strip_prefix("Search ") {
            if let Some((query, path)) = search_body.rsplit_once(" in ") {
                return RatLine::from(vec![
                    Span::styled(prefix.to_owned(), style),
                    Span::styled("Search".to_owned(), style.add_modifier(Modifier::BOLD)),
                    Span::styled(" ".to_owned(), style),
                    Span::styled(query.to_owned(), style),
                    Span::styled(" in ".to_owned(), style),
                    Span::styled(path.to_owned(), appearance::header_path_style_for(theme)),
                ]);
            }
        }
        let name_end = body.find(|c: char| c.is_whitespace()).unwrap_or(body.len());
        let name = &body[..name_end];
        let rest = &body[name_end..];
        return RatLine::from(vec![
            Span::styled(prefix.to_owned(), style),
            Span::styled(name.to_owned(), style.add_modifier(Modifier::BOLD)),
            Span::styled(rest.to_owned(), style),
        ]);
    }
    if kind == LineKind::SessionStart {
        let Some(header_start) = text.find("◆ ") else {
            return RatLine::from(text.to_owned()).style(style);
        };
        let split = header_start + "◆ ".len();
        let (prefix, body) = text.split_at(split);
        let name_end = body.find(char::is_whitespace).unwrap_or(body.len());
        let name = &body[..name_end];
        let rest = &body[name_end..];
        return RatLine::from(vec![
            Span::styled(prefix.to_owned(), style),
            Span::styled(name.to_owned(), style.add_modifier(Modifier::BOLD)),
            Span::styled(rest.to_owned(), style),
        ]);
    }
    if kind == LineKind::Activity {
        return styled_activity_line(text, style);
    }
    if matches!(kind, LineKind::ToolOutput | LineKind::ToolResult) {
        if text.starts_with("    ") {
            return RatLine::from(text.to_owned()).style(
                appearance::muted_style_for(theme)
                    .patch(appearance::panel_background_style_for(theme)),
            );
        }
        let trimmed = text.trim_start();
        if let Some((index, rest)) = trimmed.split_once(". ") {
            if index.chars().all(|ch| ch.is_ascii_digit()) {
                if let Some((path, metadata)) = rest.split_once("  (score: ") {
                    return RatLine::from(vec![
                        Span::styled("  ", appearance::muted_style_for(theme)),
                        Span::styled(format!("{index}. "), appearance::muted_style_for(theme)),
                        Span::styled(
                            path.to_owned(),
                            appearance::base_style_for(theme).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("  (score: {metadata}"),
                            appearance::muted_style_for(theme).add_modifier(Modifier::DIM),
                        ),
                    ]);
                }
            }
        }
        if let Some(memory) = text.strip_prefix("Result ") {
            let mut parts = memory.split(" · ");
            let number = parts.next().unwrap_or_default();
            let score = parts.next().unwrap_or_default();
            let source = parts.next().unwrap_or_default();
            let path = parts.next();
            if let Some(path) = path {
                return RatLine::from(vec![
                    Span::styled("Result ", appearance::muted_style_for(theme)),
                    Span::styled(number.to_owned(), appearance::muted_style_for(theme)),
                    Span::styled(
                        " · ",
                        appearance::muted_style_for(theme).add_modifier(Modifier::DIM),
                    ),
                    Span::styled(
                        score.to_owned(),
                        appearance::muted_style_for(theme).add_modifier(Modifier::DIM),
                    ),
                    Span::styled(
                        " · ",
                        appearance::muted_style_for(theme).add_modifier(Modifier::DIM),
                    ),
                    Span::styled(
                        source.to_owned(),
                        appearance::muted_style_for(theme).add_modifier(Modifier::DIM),
                    ),
                    Span::styled(
                        " · ",
                        appearance::muted_style_for(theme).add_modifier(Modifier::DIM),
                    ),
                    Span::styled(
                        path.to_owned(),
                        appearance::base_style_for(theme).add_modifier(Modifier::BOLD),
                    ),
                ]);
            }
        }
        if let Some((key, value)) = text.split_once(": ") {
            if matches!(key, "status" | "content_type" | "title") && !value.is_empty() {
                return RatLine::from(vec![
                    Span::styled(
                        format!("{key}: "),
                        appearance::muted_style_for(theme).add_modifier(Modifier::DIM),
                    ),
                    Span::styled(value.to_owned(), appearance::base_style_for(theme)),
                ]);
            }
        }
        if let Some(sources) = text.strip_prefix("  Sources: ") {
            let label = "  Sources: ";
            let primary = appearance::base_style_for(theme);
            let muted = appearance::muted_style_for(theme);
            if let Some((domains, suffix)) = sources.rsplit_once(" (+") {
                return RatLine::from(vec![
                    Span::styled(label.to_owned(), muted),
                    Span::styled(domains.to_owned(), primary),
                    Span::styled(format!(" (+{suffix}"), muted),
                ]);
            }
            return RatLine::from(vec![
                Span::styled(label.to_owned(), muted),
                Span::styled(sources.to_owned(), primary),
            ]);
        }
        let diff_style = if text.starts_with('+') && !text.starts_with("+++") {
            Some(appearance::diff_insert_style_for(theme))
        } else if text.starts_with('-') && !text.starts_with("---") {
            Some(appearance::diff_delete_style_for(theme))
        } else if text.starts_with("@@") {
            Some(appearance::accent_style_for(theme))
        } else {
            None
        };
        if let Some(diff_style) = diff_style {
            return RatLine::from(text.to_owned()).style(diff_style);
        }
    }
    if kind != LineKind::Assistant {
        return RatLine::from(text.to_owned()).style(style);
    }
    styled_assistant_line(text, style)
}

fn styled_thought_summary(text: &str, style: Style, theme: ThemeKind) -> RatLine<'static> {
    let start = text.find("◆ Thought").expect("thought marker");
    let bold_start = start + "◆ ".len();
    let end = bold_start + "Thought".len();
    let rail_len = text.strip_prefix('❙').map_or(0, |_| '❙'.len_utf8());
    let (rail, gutter) = text.split_at(rail_len);
    RatLine::from(vec![
        Span::styled(rail.to_owned(), appearance::thought_accent_style_for(theme)),
        Span::styled(
            gutter[..bold_start.saturating_sub(rail_len)].to_owned(),
            style,
        ),
        Span::styled(
            text[bold_start..end].to_owned(),
            style.add_modifier(Modifier::BOLD),
        ),
        Span::styled(text[end..].to_owned(), style),
    ])
}

fn styled_activity_line(text: &str, style: Style) -> RatLine<'static> {
    let Some(label_start) = text.find("◈ ") else {
        return RatLine::from(text.to_owned()).style(style);
    };
    let split = label_start + "◈ ".len();
    RatLine::from(vec![
        Span::styled(text[..split].to_owned(), style),
        Span::styled(text[split..].to_owned(), style.add_modifier(Modifier::BOLD)),
    ])
}

fn styled_assistant_line(text: &str, style: Style) -> RatLine<'static> {
    let (prefix, body) = text.split_at(
        text.find(|c: char| !c.is_whitespace())
            .unwrap_or(text.len()),
    );
    let mut spans = vec![Span::styled(prefix.to_owned(), style)];
    let body = if let Some(body) = body.strip_prefix("┃ ") {
        spans.push(Span::styled("┃ ".to_owned(), style));
        body
    } else if !body.is_empty() {
        // Wrapped/newline continuation rows retain the gutter as spaces, not
        // a second vertical marker. They still carry assistant markdown.
        body
    } else {
        return RatLine::from(text.to_owned()).style(style);
    };
    let mut body_spans = markdown_spans(body, style);
    spans.append(&mut body_spans);
    RatLine::from(spans)
}

fn markdown_spans(text: &str, base: Style) -> Vec<Span<'static>> {
    if is_table_row(text) {
        return table_spans(text, base);
    }
    if text.starts_with("> ") {
        return blockquote_spans(text, base);
    }
    if text.starts_with("```") {
        return vec![Span::styled(
            text.to_owned(),
            base.add_modifier(Modifier::DIM | Modifier::UNDERLINED),
        )];
    }
    if let Some(title) = atx_heading(text) {
        return vec![Span::styled(
            title.to_owned(),
            base.add_modifier(Modifier::BOLD),
        )];
    }
    let (bullet, content) = text
        .strip_prefix("- ")
        .map(|rest| ("• ", rest))
        .or_else(|| text.strip_prefix("* ").map(|rest| ("• ", rest)))
        .or_else(|| {
            let split = text.find(". ")?;
            if text[..split].chars().all(|ch| ch.is_ascii_digit()) {
                Some(("• ", &text[split + 2..]))
            } else {
                None
            }
        })
        .unwrap_or(("", text));
    let mut spans = vec![Span::styled(bullet.to_owned(), base)];
    if let Some(inline) = inline_markdown(content, base) {
        spans.extend(inline);
        return spans;
    }
    spans.extend(bold_markdown(content, base));
    spans
}

fn is_table_row(text: &str) -> bool {
    runie_tui_model::is_table_row(text)
}

fn table_spans(text: &str, base: Style) -> Vec<Span<'static>> {
    let cells: Vec<_> = text
        .trim()
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .collect();
    let separator = cells
        .iter()
        .all(|cell| !cell.is_empty() && cell.chars().all(|ch| matches!(ch, '-' | ':' | ' ')));
    let (left, right) = if separator {
        ('├', '┤')
    } else {
        ('│', '│')
    };
    let body = if separator {
        cells
            .iter()
            .map(|cell| "─".repeat(cell.chars().count().max(1) + 2))
            .collect::<Vec<_>>()
            .join("┼")
    } else {
        cells.join(" │ ")
    };
    let rendered = if separator {
        format!("{left}{body}{right}")
    } else {
        format!("{left} {body} {right}")
    };
    vec![Span::styled(rendered, base.add_modifier(Modifier::DIM))]
}

fn is_table_separator(text: &str) -> bool {
    runie_tui_model::is_table_separator(text)
}

fn atx_heading(text: &str) -> Option<&str> {
    runie_tui_model::atx_heading(text)
}

fn blockquote_spans(text: &str, base: Style) -> Vec<Span<'static>> {
    let quote = text.strip_prefix("> ").unwrap_or(text);
    let mut spans = vec![Span::styled(
        "│ ".to_owned(),
        base.add_modifier(Modifier::DIM),
    )];
    spans.extend(markdown_spans(quote, base));
    spans
}

#[allow(
    clippy::too_many_lines,
    reason = "the inline markdown grammar stays together as one pure parser"
)]
fn inline_markdown(text: &str, base: Style) -> Option<Vec<Span<'static>>> {
    if let Some(start) = text.find('`') {
        if let Some(end) = text[start + 1..].find('`') {
            let end = start + end + 1;
            let mut spans = markdown_spans(&text[..start], base);
            spans.push(Span::styled(
                text[start + 1..end].to_owned(),
                base.add_modifier(Modifier::UNDERLINED),
            ));
            spans.extend(markdown_spans(&text[end + 1..], base));
            return Some(spans);
        }
    }
    for (delimiter, modifier) in [
        ("~~", Modifier::CROSSED_OUT),
        ("_", Modifier::ITALIC),
        ("*", Modifier::ITALIC),
    ] {
        if delimiter == "*" && text.contains("**") {
            continue;
        }
        if let Some(start) = text.find(delimiter) {
            if let Some(relative_end) = text[start + delimiter.len()..].find(delimiter) {
                let end = start + delimiter.len() + relative_end;
                let mut spans = markdown_spans(&text[..start], base);
                spans.push(Span::styled(
                    text[start + delimiter.len()..end].to_owned(),
                    base.add_modifier(modifier),
                ));
                spans.extend(markdown_spans(&text[end + delimiter.len()..], base));
                return Some(spans);
            }
        }
    }
    let start = text.find('[')?;
    let close = start + text[start + 1..].find(']')? + 1;
    if !text[close + 1..].starts_with('(') {
        return None;
    }
    let end = close + text[close + 2..].find(')')? + 2;
    let mut spans = markdown_spans(&text[..start], base);
    spans.push(Span::styled(
        text[start + 1..close].to_owned(),
        base.add_modifier(Modifier::UNDERLINED),
    ));
    spans.extend(markdown_spans(&text[end + 1..], base));
    Some(spans)
}

fn bold_markdown(text: &str, base: Style) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("**") {
        if start > 0 {
            spans.push(Span::styled(rest[..start].to_owned(), base));
        }
        let after = &rest[start + 2..];
        let Some(end) = after.find("**") else {
            spans.push(Span::styled(rest[start..].to_owned(), base));
            return spans;
        };
        spans.push(Span::styled(
            after[..end].to_owned(),
            base.add_modifier(Modifier::BOLD),
        ));
        rest = &after[end + 2..];
    }
    if !rest.is_empty() {
        spans.push(Span::styled(rest.to_owned(), base));
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrollback_messages_reduce_explicit_feed_state() {
        let mut scrollback = Scrollback::new();
        let index = scrollback
            .apply(ScrollbackMsg::Append(Line::new(
                LineKind::Assistant,
                "hello",
            )))
            .expect("append returns its owned row");
        scrollback.apply(ScrollbackMsg::ReplaceLine(index, "updated".into()));
        assert_eq!(scrollback.lines()[index].text, "updated");
        scrollback.apply(ScrollbackMsg::Clear);
        assert!(scrollback.is_empty());
    }

    #[test]
    fn vpad_is_entry_metadata_not_line_kind_inference() {
        let user = Line::new(LineKind::User, "hi").with_vpad(true);
        let system = Line::new(LineKind::System, "system");
        assert!(user.has_vpad());
        assert!(!system.has_vpad());
    }

    #[test]
    fn terminal_height_controls_user_vpad_projection() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::User, "hi").with_vpad(true));
        scrollback.append(Line::new(LineKind::Assistant, "answer"));

        let mut full = Buffer::empty(Rect::new(0, 0, 80, 4));
        scrollback.render_with_terminal_height(Rect::new(0, 0, 80, 4), 24, &mut full);
        assert_eq!(full.cell((0, 0)).expect("full vpad row").symbol(), " ");
        assert_eq!(full.cell((0, 1)).expect("full user row").symbol(), " ");

        let mut compact = Buffer::empty(Rect::new(0, 0, 80, 4));
        scrollback.render_with_terminal_height(Rect::new(0, 0, 80, 4), 16, &mut compact);
        assert_eq!(
            compact.cell((0, 0)).expect("compact user row").symbol(),
            " "
        );
        assert_eq!(
            compact
                .cell((0, 2))
                .expect("compact assistant row")
                .symbol(),
            "┃"
        );

        let mut clipped = Buffer::empty(Rect::new(0, 0, 80, 3));
        scrollback.render_with_terminal_height(Rect::new(0, 0, 80, 3), 24, &mut clipped);
        assert_eq!(
            clipped.cell((0, 0)).expect("clipped user row").symbol(),
            " "
        );
        assert_eq!(
            clipped
                .cell((0, 2))
                .expect("clipped assistant row")
                .symbol(),
            "┃"
        );
    }

    #[test]
    fn measured_anchor_row_preserves_duplicate_line_occurrence() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::Assistant, "same"));
        scrollback.append(Line::new(LineKind::Assistant, "same"));
        scrollback.navigation.selected_entry = Some(1);

        assert_eq!(
            scrollback.measured_anchor_row(Rect::new(0, 0, 40, 4), 24),
            Some(1)
        );
    }

    #[test]
    fn measured_anchor_row_finds_wrapped_logical_text_fragment() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(
            LineKind::Assistant,
            "A long assistant row that must reflow across narrow terminals",
        ));
        scrollback.navigation.selected_entry = Some(0);

        assert_eq!(
            scrollback.measured_anchor_row(Rect::new(0, 0, 18, 8), 24),
            Some(0)
        );
    }

    #[test]
    fn measured_anchor_row_preserves_shared_wrapped_prefix_occurrence() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(
            LineKind::Assistant,
            "shared prefix first body that wraps",
        ));
        scrollback.append(Line::new(
            LineKind::Assistant,
            "shared prefix second body that wraps",
        ));
        scrollback.navigation.selected_entry = Some(1);

        assert!(scrollback
            .measured_anchor_row(Rect::new(0, 0, 18, 8), 24)
            .is_some_and(|row| row > 0));
    }

    #[test]
    fn completed_assistant_timestamp_wraps_at_a_word_boundary() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(
            LineKind::CompletedAssistant,
            "Hey — what are you working on? I can help with code, tests, debugging, or anything else in this repo.",
        ));
        scrollback.set_prompt_timestamp(Some("2:11 AM".to_owned()));

        let rows = scrollback.physical_rows(58, false, 32);
        let first = rows
            .iter()
            .find(|(_, text, _)| text.contains("2:11 AM"))
            .expect("timestamp row")
            .1
            .clone();
        assert!(first.ends_with("2:11 AM"));
        let timestamp_start = first.find("2:11 AM").expect("timestamp start");
        let gap = first[..timestamp_start]
            .chars()
            .rev()
            .take_while(|character| *character == ' ')
            .count();
        assert_eq!(gap, 7, "row: {first:?}");
        assert!(!first.contains("help 2:11"));
        assert!(rows.iter().any(|(_, text, _)| text.contains("with code")));
    }
    use ratatui::style::Color;

    #[test]
    fn append_and_len() {
        let mut s = Scrollback::new();
        s.append(Line::new(LineKind::User, "hi"));
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn renderer_adapter_rehydrates_only_from_feed_snapshot() {
        let mut source = Scrollback::new();
        source.apply(ScrollbackMsg::ToolStart {
            tool_call_id: "call-1".into(),
            header: "Run cargo test".into(),
            activity: None,
        });
        let snapshot = source.model_snapshot();
        let adapted = Scrollback::from_model_snapshot(snapshot.clone());
        assert_eq!(adapted.model_snapshot().lines, snapshot.lines);
        assert_eq!(adapted.model_snapshot().tool_modes, snapshot.tool_modes);
        assert_eq!(adapted.lines()[0].tool_row_id, Some(0));
    }

    #[test]
    fn clear_empties() {
        let mut s = Scrollback::new();
        s.append(Line::new(LineKind::User, "hi"));
        s.clear();
        assert!(s.is_empty());
    }

    #[test]
    fn find_first_containing() {
        let mut s = Scrollback::new();
        s.append(Line::new(LineKind::Assistant, "Hello world"));
        s.append(Line::new(LineKind::Assistant, "Goodbye world"));
        assert_eq!(s.find_first_containing("world"), Some(0));
    }

    #[test]
    fn visual_styles_are_stable() {
        assert_eq!(LineKind::User.style().fg, Some(Color::Rgb(225, 225, 225)));
        assert!(!LineKind::User.style().add_modifier.contains(Modifier::BOLD));
        assert_eq!(
            LineKind::Assistant.style().fg,
            Some(Color::Rgb(225, 225, 225))
        );
        assert_eq!(LineKind::Tool.style().fg, Some(Color::Rgb(225, 225, 225)));
        assert_eq!(
            LineKind::ToolError.style().fg,
            appearance::error_style_for(ThemeKind::GrokNight).fg
        );
        assert_eq!(
            LineKind::ToolResult.style().fg,
            Some(Color::Rgb(158, 206, 106))
        );
        assert!(LineKind::System
            .style()
            .add_modifier
            .contains(Modifier::DIM));
    }

    #[test]
    fn user_prompt_paints_grok_panel_background_across_the_row() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::SessionStart, "◆ session_start"));
        scrollback.append(Line::new(LineKind::User, "Hey"));
        scrollback.set_live_grok_layout(true);
        let mut buffer = Buffer::empty(Rect::new(0, 0, 80, 4));
        scrollback.render_with_terminal_height(Rect::new(0, 0, 80, 4), 24, &mut buffer);
        let row = (0..4)
            .find(|row| {
                buffer
                    .cell((5, *row))
                    .is_some_and(|cell| cell.symbol() == "H")
            })
            .expect("user row");
        assert_eq!(
            buffer.cell((0, row)).expect("user row background").bg,
            Color::Rgb(36, 36, 36)
        );
        assert_eq!(
            buffer.cell((79, row)).expect("full user row background").bg,
            Color::Rgb(36, 36, 36)
        );
        assert_eq!(
            buffer.cell((2, row)).expect("user gutter foreground").fg,
            Color::Reset,
            "Grok leaves the user-row lead gutter at terminal-default foreground"
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn memory_snippet_paints_grok_panel_background_across_the_row() {
        let mut scrollback = Scrollback::new();
        scrollback.apply(ScrollbackMsg::SetToolName(
            "memory-1".into(),
            "memory_search".into(),
        ));
        scrollback.apply(ScrollbackMsg::ToolStart {
            tool_call_id: "memory-1".into(),
            header: "Memory Search memory".into(),
            activity: None,
        });
        scrollback.apply(ScrollbackMsg::ToolEnd {
            tool_call_id: "memory-1".into(),
            header: "Memory Search memory".into(),
            activity: None,
            output: vec![
                (
                    LineKind::ToolOutput,
                    "  1. notes.md:4-8  (score: 0.72, global)".into(),
                ),
                (LineKind::ToolOutput, "    memory snippet".into()),
            ],
        });
        scrollback.set_tool_mode("memory-1", runie_core::types::ToolDisplayMode::Expanded);
        scrollback.set_live_grok_layout(true);
        let rows = scrollback.physical_rows(80, false, 24);
        let snippet_row = rows
            .iter()
            .position(|(_, text, _)| text.contains("memory snippet"))
            .expect("memory snippet row");
        let metadata_row = rows
            .iter()
            .position(|(_, text, _)| text.contains("notes.md"))
            .expect("memory metadata row");
        let mut buffer = Buffer::empty(Rect::new(0, 0, 80, 8));
        scrollback.render_with_terminal_height(Rect::new(0, 0, 80, 8), 24, &mut buffer);
        let row = snippet_row as u16;
        assert_eq!(
            buffer
                .cell((79, row))
                .expect("full snippet row background")
                .bg,
            Color::Rgb(36, 36, 36)
        );
        assert_eq!(
            buffer
                .cell((79, metadata_row as u16))
                .expect("full metadata row background")
                .bg,
            Color::Rgb(36, 36, 36)
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn web_search_sources_paint_grok_panel_background_across_the_row() {
        let mut scrollback = Scrollback::new();
        scrollback.apply(ScrollbackMsg::SetToolName(
            "web-1".into(),
            "web_search".into(),
        ));
        scrollback.apply(ScrollbackMsg::ToolStart {
            tool_call_id: "web-1".into(),
            header: "Web Search rust".into(),
            activity: None,
        });
        scrollback.apply(ScrollbackMsg::ToolEnd {
            tool_call_id: "web-1".into(),
            header: "Web Search rust".into(),
            activity: None,
            output: vec![(
                LineKind::ToolOutput,
                "  Sources: docs.rs, rust-lang.org".into(),
            )],
        });
        scrollback.set_tool_mode("web-1", runie_core::types::ToolDisplayMode::Expanded);
        scrollback.set_live_grok_layout(true);
        let rows = scrollback.physical_rows(80, false, 24);
        let source_row = rows
            .iter()
            .position(|(_, text, _)| text.contains("Sources:"))
            .expect("web source row");
        let mut buffer = Buffer::empty(Rect::new(0, 0, 80, 8));
        scrollback.render_with_terminal_height(Rect::new(0, 0, 80, 8), 24, &mut buffer);
        assert_eq!(
            buffer
                .cell((79, source_row as u16))
                .expect("full source row background")
                .bg,
            Color::Rgb(36, 36, 36)
        );
    }

    #[test]
    fn first_user_prompt_alone_follows_to_the_top_of_a_long_feed() {
        let mut scrollback = Scrollback::new();
        for index in 0..2 {
            scrollback.append(Line::new(LineKind::Assistant, format!("old {index}")));
        }
        scrollback.append(Line::new(LineKind::User, "Hey"));

        let mut buffer = Buffer::empty(Rect::new(0, 0, 40, 4));
        scrollback.render_with_terminal_height(Rect::new(0, 0, 40, 4), 24, &mut buffer);
        let first_row = (0..4)
            .find(|row| {
                buffer
                    .cell((5, *row))
                    .is_some_and(|cell| cell.symbol() == "H")
            })
            .expect("first submitted prompt remains visible");
        assert_eq!(first_row, 1);
    }

    #[test]
    fn newest_user_prompt_is_the_follow_anchor() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::User, "first"));
        scrollback.append(Line::new(LineKind::Assistant, "answer"));
        scrollback.append(Line::new(LineKind::User, "second"));

        let mut buffer = Buffer::empty(Rect::new(0, 0, 40, 3));
        scrollback.render_with_terminal_height(Rect::new(0, 0, 40, 3), 24, &mut buffer);
        let mut visible = String::new();
        for row in 0..3 {
            for column in 0..40 {
                if let Some(cell) = buffer.cell((column, row)) {
                    visible.push_str(cell.symbol());
                }
            }
        }
        assert!(
            visible.contains("second"),
            "newest prompt missing: {visible:?}"
        );
        assert!(
            !visible.contains("first"),
            "stale prompt anchored: {visible:?}"
        );
    }

    #[test]
    fn incoming_content_hands_following_from_prompt_to_tail() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::User, "Hey"));
        for index in 0..6 {
            scrollback.append(Line::new(LineKind::Assistant, format!("reply {index}")));
        }

        let mut buffer = Buffer::empty(Rect::new(0, 0, 40, 3));
        scrollback.render_with_terminal_height(Rect::new(0, 0, 40, 3), 24, &mut buffer);
        let mut visible = String::new();
        for row in 0..3 {
            for column in 0..40 {
                if let Some(cell) = buffer.cell((column, row)) {
                    visible.push_str(cell.symbol());
                }
            }
        }
        assert!(
            visible.contains("reply 5"),
            "tail was not followed: {visible:?}"
        );
        assert!(
            !visible.contains("Hey"),
            "prompt anchor blocked tail: {visible:?}"
        );
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "the full submitted-frame oracle keeps all row assertions together"
    )]
    fn live_submission_sequence_keeps_timestamped_user_row_visible() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::Separator, ""));
        scrollback.append(Line::new(LineKind::SessionStart, "◆ session_start"));
        scrollback.append(Line::new(LineKind::Separator, ""));
        scrollback.append(Line::new(LineKind::User, "Hey").with_vpad(true));
        scrollback.set_prompt_timestamp(Some("6:20 AM".into()));
        scrollback.append(Line::new(LineKind::Separator, ""));
        scrollback.append(Line::new(LineKind::ThinkingStatus, "◆ Thinking…"));
        scrollback.append(Line::new(LineKind::Separator, ""));
        scrollback.append(Line::new(
            LineKind::Assistant,
            "Hey — what are you working on? I can help with code.",
        ));
        scrollback.apply(ScrollbackMsg::FinalizeAssistant {
            has_reasoning: false,
            reasoning_expanded: false,
            summary: "Thought for 0.2s".into(),
            settled_no_tool_phase: true,
        });
        scrollback.remove_kind(LineKind::SessionStart);
        scrollback.normalize_live_completed_assistants();

        let mut buffer = Buffer::empty(Rect::new(0, 0, 76, 15));
        scrollback.render_with_terminal_height(Rect::new(0, 0, 76, 15), 24, &mut buffer);
        let mut visible = String::new();
        for row in 0..15 {
            for column in 0..76 {
                if let Some(cell) = buffer.cell((column, row)) {
                    visible.push_str(cell.symbol());
                }
            }
        }
        assert!(
            visible.contains("❯ Hey"),
            "live user row missing: {visible:?}"
        );
        let user_row = (0..15)
            .find(|row| {
                buffer
                    .cell((5, *row))
                    .is_some_and(|cell| cell.symbol() == "H")
            })
            .expect("live user row position");
        assert_eq!(user_row, 2, "submitted prompt must retain Grok's lead row");
        assert!(
            visible.contains("6:20 AM"),
            "user timestamp missing: {visible:?}"
        );
    }

    #[test]
    fn feed_style_uses_selected_theme_tokens() {
        assert_eq!(
            LineKind::Assistant.style_for(ThemeKind::GrokDay).fg,
            Some(Color::Rgb(38, 38, 38))
        );
        assert_eq!(
            LineKind::Activity.style_for(ThemeKind::GrokDay).fg,
            Some(Color::Rgb(125, 75, 198))
        );
        let mut scrollback = Scrollback::new();
        scrollback.set_theme(ThemeKind::GrokDay);
        assert_eq!(scrollback.theme(), ThemeKind::GrokDay);
    }

    #[test]
    fn edit_diff_rows_use_semantic_theme_tokens() {
        let inserted = styled_line_for(LineKind::ToolResult, "+new", ThemeKind::GrokDay);
        let deleted = styled_line_for(LineKind::ToolResult, "-old", ThemeKind::GrokDay);
        let hunk = styled_line_for(LineKind::ToolOutput, "@@ -1 +1 @@", ThemeKind::GrokDay);
        assert_eq!(
            inserted.style.fg,
            appearance::success_style_for(ThemeKind::GrokDay).fg
        );
        assert_eq!(inserted.style.bg, Some(Color::Rgb(218, 242, 220)));
        assert_eq!(
            deleted.style.fg,
            appearance::error_style_for(ThemeKind::GrokDay).fg
        );
        assert_eq!(deleted.style.bg, Some(Color::Rgb(245, 218, 222)));
        assert_eq!(
            hunk.style.fg,
            appearance::accent_style_for(ThemeKind::GrokDay).fg
        );
    }

    #[test]
    fn structured_tool_header_bolds_only_the_action_name() {
        let rendered = styled_line(LineKind::Tool, "   ◆ List .");
        assert!(rendered.spans[1]
            .style
            .add_modifier
            .contains(Modifier::BOLD));
        assert!(!rendered.spans[2]
            .style
            .add_modifier
            .contains(Modifier::BOLD));
    }

    #[test]
    fn specialized_tool_headers_bold_the_full_action_label() {
        for (text, label) in [
            ("   ◆ Web Search rust", "Web Search"),
            ("   ◆ Memory Search cache", "Memory Search"),
        ] {
            let rendered = styled_line(LineKind::Tool, text);
            assert!(rendered.spans[1]
                .style
                .add_modifier
                .contains(Modifier::BOLD));
            assert_eq!(rendered.spans[1].content.as_ref(), label);
        }
    }

    #[test]
    fn file_tool_headers_resolve_paths_through_theme_tokens() {
        let rendered = styled_line_for(LineKind::Tool, "   ◆ Read src/lib.rs", ThemeKind::GrokDay);
        assert_eq!(
            rendered.spans[3].style.fg,
            appearance::header_path_style_for(ThemeKind::GrokDay).fg
        );
        assert!(rendered.spans[1]
            .style
            .add_modifier
            .contains(Modifier::BOLD));
    }

    #[test]
    fn search_headers_resolve_scope_paths_separately() {
        let rendered = styled_line_for(
            LineKind::Tool,
            "   ◆ Search \"TODO\" in src",
            ThemeKind::GrokNight,
        );
        assert_eq!(rendered.spans[5].content.as_ref(), "src");
        assert_eq!(
            rendered.spans[5].style.fg,
            appearance::header_path_style_for(ThemeKind::GrokNight).fg
        );
        assert!(!rendered.spans[3]
            .style
            .add_modifier
            .contains(Modifier::BOLD));
    }

    #[test]
    fn web_search_source_label_uses_muted_token() {
        let rendered = styled_line_for(
            LineKind::ToolOutput,
            "  Sources: example.com, docs.rs (+2 more)",
            ThemeKind::GrokDay,
        );
        assert_eq!(
            rendered.spans[0].style.fg,
            appearance::muted_style_for(ThemeKind::GrokDay).fg
        );
        assert_eq!(
            rendered.spans[1].style.fg,
            appearance::base_style_for(ThemeKind::GrokDay).fg
        );
        assert_eq!(
            rendered.spans[2].style.fg,
            appearance::muted_style_for(ThemeKind::GrokDay).fg
        );
    }

    #[test]
    fn web_fetch_header_styles_url_as_a_specialized_path() {
        let rendered = styled_line_for(
            LineKind::Tool,
            "◆ Fetch https://example.com/release",
            ThemeKind::GrokNight,
        );
        assert!(rendered.spans[1]
            .style
            .add_modifier
            .contains(Modifier::BOLD));
        assert_eq!(rendered.spans[3].content, "https://example.com/release");
        assert_eq!(
            rendered.spans[3].style.fg,
            appearance::header_path_style_for(ThemeKind::GrokNight).fg
        );
    }

    #[test]
    fn web_fetch_metadata_rows_split_muted_keys_from_primary_values() {
        for (key, value) in [("content_type", "text/html"), ("title", "Release notes")] {
            let rendered = styled_line_for(
                LineKind::ToolOutput,
                &format!("{key}: {value}"),
                ThemeKind::GrokNight,
            );
            assert_eq!(rendered.spans[0].content, format!("{key}: "));
            assert!(rendered.spans[0].style.add_modifier.contains(Modifier::DIM));
            assert_eq!(rendered.spans[1].content, value);
            assert_eq!(
                rendered.spans[1].style.fg,
                appearance::base_style_for(ThemeKind::GrokNight).fg
            );
        }
    }

    #[test]
    fn web_fetch_metadata_paints_grok_panel_background_across_the_row() {
        let mut scrollback = Scrollback::new();
        scrollback.apply(ScrollbackMsg::SetToolName(
            "fetch-1".into(),
            "web_fetch".into(),
        ));
        scrollback.apply(ScrollbackMsg::ToolStart {
            tool_call_id: "fetch-1".into(),
            header: "Fetch https://example.com".into(),
            activity: None,
        });
        scrollback.apply(ScrollbackMsg::ToolEnd {
            tool_call_id: "fetch-1".into(),
            header: "Fetch https://example.com".into(),
            activity: None,
            output: vec![(LineKind::ToolOutput, "status: 200".into())],
        });
        scrollback.set_tool_mode("fetch-1", runie_core::types::ToolDisplayMode::Expanded);
        scrollback.set_live_grok_layout(true);
        let rows = scrollback.physical_rows(80, false, 24);
        let metadata_row = rows
            .iter()
            .position(|(_, text, _)| text.contains("status: 200"))
            .expect("fetch metadata row");
        let mut buffer = Buffer::empty(Rect::new(0, 0, 80, 8));
        scrollback.render_with_terminal_height(Rect::new(0, 0, 80, 8), 24, &mut buffer);
        assert_eq!(
            buffer
                .cell((79, metadata_row as u16))
                .expect("full fetch metadata background")
                .bg,
            Color::Rgb(36, 36, 36)
        );
    }

    #[test]
    fn web_fetch_metadata_keeps_primary_value_foreground_after_card_paint() {
        let mut scrollback = Scrollback::new();
        scrollback.apply(ScrollbackMsg::SetToolName(
            "fetch-1".into(),
            "web_fetch".into(),
        ));
        scrollback.apply(ScrollbackMsg::ToolStart {
            tool_call_id: "fetch-1".into(),
            header: "Fetch https://example.com".into(),
            activity: None,
        });
        scrollback.apply(ScrollbackMsg::ToolEnd {
            tool_call_id: "fetch-1".into(),
            header: "Fetch https://example.com".into(),
            activity: None,
            output: vec![(LineKind::ToolOutput, "status: 200".into())],
        });
        scrollback.set_tool_mode("fetch-1", runie_core::types::ToolDisplayMode::Expanded);
        scrollback.set_live_grok_layout(true);
        let rows = scrollback.physical_rows(80, false, 24);
        let row = rows
            .iter()
            .position(|(_, text, _)| text.contains("status: 200"))
            .expect("fetch metadata row") as u16;
        let mut buffer = Buffer::empty(Rect::new(0, 0, 80, 8));
        scrollback.render_with_terminal_height(Rect::new(0, 0, 80, 8), 24, &mut buffer);
        let value_cell = (0..80)
            .find_map(|x| {
                buffer
                    .cell((x, row))
                    .filter(|cell| cell.symbol() == "2")
                    .map(|cell| cell.fg)
            })
            .expect("status value cell");
        assert_eq!(
            value_cell,
            appearance::base_style_for(ThemeKind::GrokNight)
                .fg
                .expect("base foreground")
        );
    }

    #[test]
    fn use_and_todo_headers_style_their_action_name_and_target() {
        for text in ["◆ Use git_status", "◆ Used git_status", "◆ Todo release"] {
            let rendered = styled_line_for(LineKind::Tool, text, ThemeKind::GrokNight);
            assert!(rendered.spans[1]
                .style
                .add_modifier
                .contains(Modifier::BOLD));
            assert_eq!(
                rendered.spans[3].style.fg,
                appearance::header_path_style_for(ThemeKind::GrokNight).fg
            );
        }
    }

    #[test]
    fn memory_result_paths_are_bold_primary_spans() {
        let rendered = styled_line_for(
            LineKind::ToolOutput,
            "Result 1 · 0.72 · global · memory.md:1-2",
            ThemeKind::GrokNight,
        );
        assert_eq!(rendered.spans.len(), 8);
        assert!(!rendered.spans[1].style.add_modifier.contains(Modifier::DIM));
        assert!(rendered.spans[3].style.add_modifier.contains(Modifier::DIM));
        assert!(rendered.spans[5].style.add_modifier.contains(Modifier::DIM));
        assert!(rendered.spans[7]
            .style
            .add_modifier
            .contains(Modifier::BOLD));
        assert_eq!(
            rendered.spans[7].style.fg,
            appearance::base_style_for(ThemeKind::GrokNight).fg
        );
    }

    #[test]
    fn grok_activity_label_bolds_the_grouped_summary() {
        let rendered = styled_line(LineKind::Activity, "❙  ◈ Listed 1 dir, Read 1 file");
        assert!(!rendered.spans[0]
            .style
            .add_modifier
            .contains(Modifier::BOLD));
        assert!(rendered.spans[1]
            .style
            .add_modifier
            .contains(Modifier::BOLD));
    }

    #[test]
    fn embedded_newlines_render_as_separate_rows() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::Assistant, "first\nsecond"));
        let mut buffer = Buffer::empty(Rect::new(0, 0, 30, 4));
        scrollback.render(Rect::new(0, 0, 30, 4), &mut buffer);
        assert_eq!(buffer.cell((0, 0)).expect("first row").symbol(), "┃");
        assert_eq!(buffer.cell((0, 1)).expect("second row").symbol(), " ");
        let rendered: String = (0..30)
            .flat_map(|y| (0..30).map(move |x| (x, y)))
            .filter_map(|(x, y)| buffer.cell((x, y)))
            .map(|cell| cell.symbol().to_string())
            .collect();
        assert!(rendered.contains("second"));
    }

    #[test]
    fn assistant_markdown_preserves_grok_bullets_and_bold_spans() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::Assistant, "# runie"));
        scrollback.append(Line::new(LineKind::Assistant, "- **fast** replay"));
        let mut buffer = Buffer::empty(Rect::new(0, 0, 30, 3));
        scrollback.render(Rect::new(0, 0, 30, 3), &mut buffer);
        assert_eq!(buffer.cell((0, 0)).expect("heading prefix").symbol(), "┃");
        assert_eq!(buffer.cell((1, 0)).expect("heading").symbol(), "#");
        assert_eq!(buffer.cell((1, 1)).expect("bullet prefix").symbol(), "-");
        let bullet_row: String = (0..30)
            .filter_map(|column| buffer.cell((column, 1)))
            .map(|cell| cell.symbol().to_owned())
            .collect();
        assert!(bullet_row.contains("fast"));
    }

    #[test]
    fn assistant_markdown_styles_code_and_links() {
        let rendered = styled_line(LineKind::Assistant, "┃ use `cargo test` [docs](https://x)");
        assert!(rendered.spans.iter().any(|span| {
            span.content == "cargo test" && span.style.add_modifier.contains(Modifier::UNDERLINED)
        }));
        assert!(rendered.spans.iter().any(|span| {
            span.content == "docs" && span.style.add_modifier.contains(Modifier::UNDERLINED)
        }));
    }

    #[test]
    fn assistant_markdown_renders_blockquote_gutter() {
        let line = styled_assistant_line("> quoted **text**", Style::default());
        let rendered = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();
        assert_eq!(rendered, "│ quoted text");
        assert!(line.spans[1].style.add_modifier.contains(Modifier::DIM));
    }

    #[test]
    fn assistant_markdown_bolds_all_atx_heading_levels() {
        for level in 1..=6 {
            let text = format!("{} title", "#".repeat(level));
            let line = styled_assistant_line(&text, Style::default());
            let heading = line.spans.last().expect("heading span");
            assert_eq!(heading.content, "title");
            assert!(heading.style.add_modifier.contains(Modifier::BOLD));
        }
    }

    #[test]
    fn assistant_markdown_renders_table_box_drawing_rows() {
        let header = styled_assistant_line("| Name | Type |", Style::default());
        let separator = styled_assistant_line("| ---- | ---- |", Style::default());
        assert_eq!(
            header.spans.last().expect("table header").content,
            "│ Name │ Type │"
        );
        assert_eq!(
            separator.spans.last().expect("table separator").content,
            "├──────┼──────┤"
        );

        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(
            LineKind::Assistant,
            "| Name | Type |\n| ---- | ---- |\n| runie | tool |",
        ));
        let mut buffer = Buffer::empty(Rect::new(0, 0, 30, 5));
        scrollback.render(Rect::new(0, 0, 30, 5), &mut buffer);
        let row = (0..30)
            .map(|column| {
                buffer
                    .cell((column, 3))
                    .expect("table border cell")
                    .symbol()
            })
            .collect::<String>();
        assert!(row.starts_with(" └───────┴──────┘"), "{row:?}");
    }

    #[test]
    fn assistant_markdown_styles_ordered_lists_and_emphasis() {
        let rendered = styled_line(LineKind::Assistant, "┃ 1. _quiet_ ~~old~~");
        assert!(rendered.spans.iter().any(|span| span.content == "• "));
        assert!(rendered.spans.iter().any(|span| {
            span.content == "quiet" && span.style.add_modifier.contains(Modifier::ITALIC)
        }));
        assert!(rendered.spans.iter().any(|span| {
            span.content == "old" && span.style.add_modifier.contains(Modifier::CROSSED_OUT)
        }));
    }

    #[test]
    fn assistant_markdown_styles_fence_markers() {
        let rendered = styled_line(LineKind::Assistant, "   ┃ ```rust");
        assert!(rendered.spans.iter().any(|span| {
            span.content == "```rust"
                && span.style.add_modifier.contains(Modifier::DIM)
                && span.style.add_modifier.contains(Modifier::UNDERLINED)
        }));
    }

    #[test]
    fn fenced_code_styles_interior_lines() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::Assistant, "```rust"));
        scrollback.append(Line::new(LineKind::Assistant, "cargo test"));
        scrollback.append(Line::new(LineKind::Assistant, "```"));
        let mut buffer = Buffer::empty(Rect::new(0, 0, 30, 4));
        scrollback.render(Rect::new(0, 0, 30, 4), &mut buffer);
        assert!(buffer
            .cell((0, 1))
            .expect("code row")
            .modifier
            .contains(Modifier::DIM));
    }

    #[test]
    fn grok_user_feed_cursor_is_at_column_five() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::User, "Please list files"));
        let mut buffer = Buffer::empty(Rect::new(2, 0, 76, 2));
        scrollback.render(Rect::new(2, 0, 76, 2), &mut buffer);
        assert_eq!(buffer.cell((2, 0)).expect("gutter").symbol(), " ");
        assert_eq!(buffer.cell((5, 1)).expect("Grok user cursor").symbol(), "❯");
        assert_eq!(
            buffer.cell((7, 1)).expect("first user letter").symbol(),
            "P"
        );
    }

    #[test]
    fn live_user_panel_trailing_cells_keep_default_foreground() {
        let mut scrollback = Scrollback::new();
        scrollback.set_live_grok_layout(true);
        scrollback.append(Line::new(LineKind::User, "Hey"));
        let mut buffer = Buffer::empty(Rect::new(0, 0, 40, 2));
        scrollback.render(Rect::new(0, 0, 40, 2), &mut buffer);
        let trailing = buffer.cell((10, 1)).expect("user panel trailing cell");
        assert_eq!(trailing.bg, Color::Rgb(36, 36, 36));
        assert_eq!(trailing.fg, Color::Reset);
    }

    #[test]
    fn turn_summary_uses_groks_column_six_gutter() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::TurnSummary, "Worked for 2.3s"));
        let mut buffer = Buffer::empty(Rect::new(0, 0, 60, 1));
        scrollback.render(Rect::new(0, 0, 60, 1), &mut buffer);
        assert_eq!(buffer.cell((0, 0)).expect("first gutter").symbol(), " ");
        assert_eq!(buffer.cell((2, 0)).expect("fifth gutter").symbol(), " ");
        assert_eq!(buffer.cell((3, 0)).expect("summary start").symbol(), "W");
    }

    #[test]
    fn thought_summary_uses_groks_transcript_gutter() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::TurnSummary, "◆ Thought for 0.2s"));
        let mut buffer = Buffer::empty(Rect::new(0, 0, 60, 1));
        scrollback.render(Rect::new(0, 0, 60, 1), &mut buffer);
        assert_eq!(buffer.cell((0, 0)).expect("thought gutter").symbol(), " ");
        assert_eq!(buffer.cell((5, 0)).expect("thought marker").symbol(), "◆");
    }

    #[test]
    fn live_completed_assistant_keeps_primary_body_style() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::Assistant, "Hello from Runie"));
        scrollback.normalize_live_completed_assistants();
        scrollback.set_live_grok_layout(true);

        assert_eq!(scrollback.lines()[0].kind, LineKind::CompletedAssistant);
        let mut buffer = Buffer::empty(Rect::new(0, 0, 30, 1));
        scrollback.render(Rect::new(0, 0, 30, 1), &mut buffer);
        let cell = buffer.cell((3, 0)).expect("assistant body");
        assert_eq!(
            cell.fg,
            appearance::assistant_body_style_for(ThemeKind::GrokNight)
                .fg
                .expect("assistant body token")
        );
        assert!(!cell.modifier.contains(Modifier::DIM));
    }

    #[test]
    fn grok_user_pointer_uses_blue_accent_without_bold_body() {
        let rendered = styled_line(LineKind::User, "   ❯ hello");
        assert_eq!(rendered.spans[0].style.fg, Some(Color::Rgb(225, 225, 225)));
        assert!(!rendered.spans[0]
            .style
            .add_modifier
            .contains(Modifier::BOLD));
        assert!(!rendered.spans[1]
            .style
            .add_modifier
            .contains(Modifier::BOLD));
    }

    #[test]
    fn reasoning_uses_grok_dim_italic_transcript_style() {
        let style = LineKind::Reasoning.style();
        assert!(style.add_modifier.contains(Modifier::DIM));
        assert!(style.add_modifier.contains(Modifier::ITALIC));
        assert_eq!(LineKind::Reasoning.prefix(), "┃  ");
    }

    #[test]
    fn reasoning_fold_has_deterministic_collapsed_and_expanded_cells() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::Reasoning, "checking the request"));
        let mut collapsed = Buffer::empty(Rect::new(0, 0, 40, 1));
        scrollback.render(Rect::new(0, 0, 40, 1), &mut collapsed);
        assert_eq!(
            collapsed.cell((0, 0)).expect("collapsed gutter").symbol(),
            "┃"
        );
        assert_eq!(
            collapsed.cell((1, 0)).expect("collapsed label").symbol(),
            "T"
        );

        scrollback.set_reasoning_expanded(true);
        let mut expanded = Buffer::empty(Rect::new(0, 0, 40, 1));
        scrollback.render(Rect::new(0, 0, 40, 1), &mut expanded);
        assert_eq!(expanded.cell((1, 0)).expect("expanded body").symbol(), "c");
        assert!(expanded
            .cell((1, 0))
            .expect("expanded style")
            .modifier
            .contains(Modifier::DIM));
        assert!(expanded
            .cell((3, 0))
            .expect("expanded style")
            .modifier
            .contains(Modifier::ITALIC));
    }

    #[test]
    fn typed_tool_blocks_rebuild_from_parallel_actor_rows() {
        let mut scrollback = Scrollback::new();
        scrollback.apply(ScrollbackMsg::ToolStart {
            tool_call_id: "a".into(),
            header: "Read a.txt".into(),
            activity: None,
        });
        scrollback.apply(ScrollbackMsg::ToolStart {
            tool_call_id: "b".into(),
            header: "Run tests".into(),
            activity: None,
        });
        scrollback.apply(ScrollbackMsg::ToolEnd {
            tool_call_id: "b".into(),
            header: "Run tests (ok)".into(),
            activity: None,
            output: vec![(LineKind::ToolResult, "passed".into())],
        });
        scrollback.apply(ScrollbackMsg::SetToolMode(
            "a".into(),
            runie_core::types::ToolDisplayMode::Truncated,
        ));

        let blocks = scrollback.tool_blocks();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].tool_call_id, "a");
        assert_eq!(
            blocks[0].mode,
            runie_core::types::ToolDisplayMode::Truncated
        );
        assert!(!blocks[0].is_running);
        assert_eq!(blocks[1].output, vec!["passed"]);
        assert!(!blocks[1].is_running);
    }

    #[test]
    fn live_tool_identity_targets_new_row_after_compatibility_seed() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::Tool, "seed header").for_tool("duplicate"));
        scrollback.apply(ScrollbackMsg::ToolStart {
            tool_call_id: "duplicate".into(),
            header: "live header".into(),
            activity: None,
        });
        scrollback.apply(ScrollbackMsg::ToolUpdate {
            tool_call_id: "duplicate".into(),
            header: Some("live update".into()),
            output: vec!["live output".into()],
        });
        scrollback.apply(ScrollbackMsg::ToolEnd {
            tool_call_id: "duplicate".into(),
            header: "live done".into(),
            activity: None,
            output: Vec::new(),
        });

        let headers = scrollback
            .lines()
            .iter()
            .filter(|line| line.tool_call_id.as_deref() == Some("duplicate"))
            .filter(|line| matches!(line.kind, LineKind::Tool | LineKind::ToolRunning))
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(headers, vec!["seed header", "live done"]);
        assert_eq!(scrollback.lines()[0].tool_row_id, None);
        assert!(scrollback.lines()[1].tool_row_id.is_some());
        assert!(scrollback
            .lines()
            .iter()
            .any(|line| line.kind == LineKind::ToolOutput && line.text == "live output"));
    }

    #[test]
    fn duplicate_live_provider_ids_keep_each_opaque_row_owner() {
        let mut scrollback = Scrollback::new();
        scrollback.apply(ScrollbackMsg::ToolStart {
            tool_call_id: "duplicate".into(),
            header: "first running".into(),
            activity: None,
        });
        scrollback.apply(ScrollbackMsg::ToolStart {
            tool_call_id: "duplicate".into(),
            header: "second running".into(),
            activity: None,
        });
        scrollback.apply(ScrollbackMsg::ToolUpdate {
            tool_call_id: "duplicate".into(),
            header: Some("second updated".into()),
            output: Vec::new(),
        });
        scrollback.apply(ScrollbackMsg::ToolEnd {
            tool_call_id: "duplicate".into(),
            header: "second done".into(),
            activity: None,
            output: Vec::new(),
        });
        scrollback.apply(ScrollbackMsg::ToolEnd {
            tool_call_id: "duplicate".into(),
            header: "first done".into(),
            activity: None,
            output: Vec::new(),
        });

        let headers = scrollback
            .lines()
            .iter()
            .filter(|line| line.tool_call_id.as_deref() == Some("duplicate"))
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(headers, vec!["first done", "second done"]);
        assert_ne!(
            scrollback.lines()[0].tool_row_id,
            scrollback.lines()[1].tool_row_id
        );
    }

    #[test]
    fn typed_tool_name_survives_header_rewrite() {
        let mut scrollback = Scrollback::new();
        scrollback.apply(ScrollbackMsg::SetToolName("call-1".into(), "read".into()));
        scrollback.apply(ScrollbackMsg::ToolStart {
            tool_call_id: "call-1".into(),
            header: "Read README.md".into(),
            activity: None,
        });
        scrollback.apply(ScrollbackMsg::ToolEnd {
            tool_call_id: "call-1".into(),
            header: "Completed README.md".into(),
            activity: None,
            output: Vec::new(),
        });
        assert_eq!(scrollback.tool_blocks()[0].kind, ToolCardKind::Read);
    }

    #[test]
    fn selected_tool_fold_cycles_collapsed_and_expanded_modes() {
        let mut scrollback = Scrollback::new();
        scrollback.apply(ScrollbackMsg::ToolStart {
            tool_call_id: "call-1".into(),
            header: "Read README.md".into(),
            activity: None,
        });
        assert_eq!(
            scrollback.tool_blocks()[0].mode,
            runie_core::types::ToolDisplayMode::Expanded
        );
        scrollback.apply(ScrollbackMsg::ToggleToolMode("call-1".into()));
        assert_eq!(
            scrollback.tool_blocks()[0].mode,
            runie_core::types::ToolDisplayMode::Collapsed
        );
        scrollback.apply(ScrollbackMsg::ToggleToolMode("call-1".into()));
        assert_eq!(
            scrollback.tool_blocks()[0].mode,
            runie_core::types::ToolDisplayMode::Expanded
        );
    }

    #[test]
    fn running_generic_tool_uses_grok_truncated_fold_cycle_then_settled_cycle() {
        let mut scrollback = Scrollback::new();
        scrollback.apply(ScrollbackMsg::ToolStartRunning {
            tool_call_id: "call-1".into(),
            header: "custom_tool running".into(),
            activity: None,
        });

        scrollback.apply(ScrollbackMsg::ToggleToolMode("call-1".into()));
        assert_eq!(
            scrollback.tool_blocks()[0].mode,
            runie_core::types::ToolDisplayMode::Truncated
        );
        scrollback.apply(ScrollbackMsg::ToggleToolMode("call-1".into()));
        assert_eq!(
            scrollback.tool_blocks()[0].mode,
            runie_core::types::ToolDisplayMode::Expanded
        );
        scrollback.apply(ScrollbackMsg::ToggleToolMode("call-1".into()));
        assert_eq!(
            scrollback.tool_blocks()[0].mode,
            runie_core::types::ToolDisplayMode::Truncated
        );

        scrollback.apply(ScrollbackMsg::ToolEnd {
            tool_call_id: "call-1".into(),
            header: "custom_tool done".into(),
            activity: None,
            output: Vec::new(),
        });
        scrollback.apply(ScrollbackMsg::ToggleToolMode("call-1".into()));
        assert_eq!(
            scrollback.tool_blocks()[0].mode,
            runie_core::types::ToolDisplayMode::Collapsed
        );
        scrollback.apply(ScrollbackMsg::ToggleToolMode("call-1".into()));
        assert_eq!(
            scrollback.tool_blocks()[0].mode,
            runie_core::types::ToolDisplayMode::Expanded
        );
    }

    #[test]
    fn read_truncated_preview_keeps_groks_first_five_ellipsis_and_last_three() {
        let mut scrollback = Scrollback::new();
        scrollback.apply(ScrollbackMsg::SetToolName("read-1".into(), "read".into()));
        scrollback.apply(ScrollbackMsg::ToolStart {
            tool_call_id: "read-1".into(),
            header: "Read src/lib.rs".into(),
            activity: None,
        });
        for index in 1..=10 {
            scrollback.append(
                Line::new(LineKind::ToolOutput, format!("line {index}")).for_tool("read-1"),
            );
        }
        scrollback.apply(ScrollbackMsg::SetToolMode(
            "read-1".into(),
            runie_core::types::ToolDisplayMode::Truncated,
        ));

        let rows = scrollback
            .physical_rows(80, false, 30)
            .into_iter()
            .map(|(_, text, _)| text)
            .collect::<Vec<_>>();
        assert!(rows.iter().any(|row| row.contains("line 1")));
        assert!(rows.iter().any(|row| row.contains("line 5")));
        assert!(rows.iter().any(|row| row == "…"));
        assert!(rows.iter().any(|row| row.contains("line 8")));
        assert!(rows.iter().any(|row| row.contains("line 10")));
        assert!(!rows.iter().any(|row| row.contains("line 6")));
        assert!(!rows.iter().any(|row| row.contains("line 7")));
    }

    #[test]
    fn execute_truncated_preview_keeps_groks_first_two_counted_ellipsis_and_last_three() {
        let mut scrollback = Scrollback::new();
        scrollback.apply(ScrollbackMsg::SetToolName("exec-1".into(), "bash".into()));
        scrollback.apply(ScrollbackMsg::ToolStart {
            tool_call_id: "exec-1".into(),
            header: "Run cargo test".into(),
            activity: None,
        });
        for index in 1..=8 {
            scrollback.append(
                Line::new(LineKind::ToolOutput, format!("output {index}")).for_tool("exec-1"),
            );
        }
        scrollback.apply(ScrollbackMsg::SetToolMode(
            "exec-1".into(),
            runie_core::types::ToolDisplayMode::Truncated,
        ));

        let rows = scrollback
            .physical_rows(80, false, 30)
            .into_iter()
            .map(|(_, text, _)| text)
            .collect::<Vec<_>>();
        assert!(rows.iter().any(|row| row.contains("output 1")));
        assert!(rows.iter().any(|row| row.contains("output 2")));
        assert!(rows.iter().any(|row| row == "… +3 lines"));
        assert!(rows.iter().any(|row| row.contains("output 6")));
        assert!(rows.iter().any(|row| row.contains("output 8")));
        assert!(!rows.iter().any(|row| row.contains("output 3")));
    }

    #[test]
    fn tool_selection_wraps_in_transcript_order() {
        let mut scrollback = Scrollback::new();
        for id in ["first", "second"] {
            scrollback.apply(ScrollbackMsg::ToolStart {
                tool_call_id: id.into(),
                header: format!("Read {id}"),
                activity: None,
            });
        }
        scrollback.apply(ScrollbackMsg::SelectNextTool);
        assert_eq!(scrollback.selected_tool_id(), Some("first"));
        scrollback.apply(ScrollbackMsg::SelectNextTool);
        assert_eq!(scrollback.selected_tool_id(), Some("second"));
        scrollback.apply(ScrollbackMsg::SelectNextTool);
        assert_eq!(scrollback.selected_tool_id(), Some("first"));
        scrollback.apply(ScrollbackMsg::SelectPreviousTool);
        assert_eq!(scrollback.selected_tool_id(), Some("second"));
    }

    #[test]
    fn entry_selection_navigates_semantic_rows_and_projects_tool_id() {
        let mut scrollback = Scrollback::new();
        scrollback.apply(ScrollbackMsg::Append(Line::new(LineKind::User, "Hey")));
        scrollback.apply(ScrollbackMsg::ToolStart {
            tool_call_id: "call-1".into(),
            header: "Read README.md".into(),
            activity: None,
        });
        scrollback.apply(ScrollbackMsg::Append(Line::new(
            LineKind::Assistant,
            "Done",
        )));
        scrollback.apply(ScrollbackMsg::SelectNextEntry);
        assert_eq!(scrollback.selected_entry(), Some(0));
        assert_eq!(scrollback.selected_tool_id(), None);
        scrollback.apply(ScrollbackMsg::SelectNextEntry);
        assert_eq!(scrollback.selected_tool_id(), Some("call-1"));
        scrollback.apply(ScrollbackMsg::SelectPreviousEntry);
        assert_eq!(scrollback.selected_entry(), Some(0));
    }

    #[test]
    fn selected_non_tool_entry_paints_the_theme_selection_surface() {
        let mut scrollback = Scrollback::new();
        scrollback.apply(ScrollbackMsg::Append(Line::new(LineKind::User, "Hey")));
        scrollback.apply(ScrollbackMsg::SelectNextEntry);
        let mut buffer = Buffer::empty(Rect::new(0, 0, 40, 2));
        scrollback.render(Rect::new(0, 0, 40, 2), &mut buffer);
        assert_eq!(
            buffer.cell((39, 0)).expect("selected row").bg,
            ratatui::style::Color::Rgb(28, 28, 28)
        );
        assert_eq!(buffer.cell((0, 0)).expect("selection border").symbol(), "│");
    }

    #[test]
    fn entry_navigation_reveals_selected_row_in_small_viewport() {
        let mut scrollback = Scrollback::new();
        for text in ["one", "two", "three", "four", "five"] {
            scrollback.apply(ScrollbackMsg::Append(Line::new(LineKind::User, text)));
        }
        for _ in 0..5 {
            scrollback.apply(ScrollbackMsg::SelectNextEntry);
        }
        let mut buffer = Buffer::empty(Rect::new(0, 0, 20, 2));
        scrollback.render(Rect::new(0, 0, 20, 2), &mut buffer);
        let mut visible = String::new();
        for row in 0..2 {
            for column in 0..20 {
                if let Some(cell) = buffer.cell((column, row)) {
                    visible.push_str(cell.symbol());
                }
            }
        }
        assert!(
            visible.contains("five"),
            "selected row not revealed: {visible:?}"
        );
    }

    #[test]
    fn rendering_does_not_mutate_actor_owned_viewport_state() {
        let mut scrollback = Scrollback::new();
        for index in 0..8 {
            scrollback.append(Line::new(LineKind::Assistant, format!("row {index}")));
        }
        scrollback.apply(ScrollbackMsg::SelectNextEntry);
        let before = scrollback.model_snapshot();
        let mut buffer = Buffer::empty(Rect::new(0, 0, 20, 2));
        scrollback.render_with_terminal_height(Rect::new(0, 0, 20, 2), 24, &mut buffer);
        assert_eq!(scrollback.model_snapshot(), before);
    }

    #[test]
    fn explicit_scroll_intent_hands_off_from_autoscroll() {
        let mut scrollback = Scrollback::new();
        for index in 0..8 {
            scrollback.append(Line::new(LineKind::Assistant, format!("row {index}")));
        }
        scrollback.apply(ScrollbackMsg::ScrollBy(2));
        assert!(!scrollback.navigation.autoscroll);
        assert!(!scrollback.navigation.follow_latest_user);
        assert_eq!(scrollback.navigation.scroll_offset, 10);
        scrollback.apply(ScrollbackMsg::ScrollBy(-1));
        assert_eq!(scrollback.navigation.scroll_offset, 9);
    }

    #[test]
    fn actor_snapshot_preserves_explicit_latest_reveal_for_rendering() {
        let mut source = Scrollback::new();
        for index in 0..16 {
            source.append(Line::new(LineKind::Assistant, format!("row {index}")));
        }
        source.apply(ScrollbackMsg::ScrollBy(-3));
        source.apply(ScrollbackMsg::RevealLatest);

        let adapted = Scrollback::from_model_snapshot(source.model_snapshot());
        assert!(adapted.navigation.autoscroll);
        assert_eq!(
            adapted.navigation.scroll_offset,
            source.navigation.scroll_offset
        );
    }

    #[test]
    fn clear_resets_actor_owned_tool_selection() {
        let mut scrollback = Scrollback::new();
        scrollback.apply(ScrollbackMsg::ToolStart {
            tool_call_id: "call-1".into(),
            header: "Read README.md".into(),
            activity: None,
        });
        scrollback.apply(ScrollbackMsg::SelectNextTool);
        assert_eq!(scrollback.selected_tool_id(), Some("call-1"));
        scrollback.apply(ScrollbackMsg::Clear);
        assert_eq!(scrollback.selected_tool_id(), None);
    }

    #[test]
    fn selected_tool_header_uses_grok_fold_indicator() {
        let mut scrollback = Scrollback::new();
        scrollback.set_activity_expanded(true);
        scrollback.apply(ScrollbackMsg::Append(Line::new(
            LineKind::Assistant,
            "before",
        )));
        scrollback.apply(ScrollbackMsg::ToolStart {
            tool_call_id: "selected".into(),
            header: "Read README.md".into(),
            activity: None,
        });
        scrollback.apply(ScrollbackMsg::SelectNextTool);
        let mut buffer = Buffer::empty(Rect::new(0, 0, 40, 3));
        scrollback.render(Rect::new(0, 0, 40, 3), &mut buffer);
        let selected_cell = (0..3)
            .flat_map(|row| (0..40).map(move |column| (column, row)))
            .find_map(|(column, row)| {
                buffer
                    .cell((column, row))
                    .filter(|cell| cell.symbol() == "⌄")
                    .map(|cell| cell.bg)
            })
            .expect("selected fold indicator");
        assert_eq!(selected_cell, ratatui::style::Color::Rgb(28, 28, 28));
        assert_eq!(
            buffer
                .cell((39, selected_cell_row(&buffer)))
                .expect("row")
                .bg,
            ratatui::style::Color::Rgb(28, 28, 28)
        );
    }

    #[test]
    fn collapsed_selected_tool_header_uses_grok_right_chevron() {
        let mut scrollback = Scrollback::new();
        scrollback.set_activity_expanded(true);
        scrollback.apply(ScrollbackMsg::ToolStart {
            tool_call_id: "collapsed".into(),
            header: "Read README.md".into(),
            activity: None,
        });
        scrollback.apply(ScrollbackMsg::SetToolMode(
            "collapsed".into(),
            runie_core::types::ToolDisplayMode::Collapsed,
        ));
        scrollback.apply(ScrollbackMsg::SelectNextTool);
        let mut buffer = Buffer::empty(Rect::new(0, 0, 40, 3));
        scrollback.render(Rect::new(0, 0, 40, 3), &mut buffer);
        assert!((0..3).any(|row| {
            (0..40).any(|column| {
                buffer
                    .cell((column, row))
                    .is_some_and(|cell| cell.symbol() == "›")
            })
        }));
    }

    #[test]
    fn collapsed_activity_hides_default_truncated_tool_rows() {
        let mut scrollback = Scrollback::new();
        scrollback.set_activity_expanded(false);
        scrollback.apply(ScrollbackMsg::SetToolName("bash-1".into(), "bash".into()));
        scrollback.apply(ScrollbackMsg::ToolStart {
            tool_call_id: "bash-1".into(),
            header: "Run cargo test".into(),
            activity: Some("Ran 2 commands".into()),
        });
        scrollback.apply(ScrollbackMsg::ToolStart {
            tool_call_id: "bash-2".into(),
            header: "Run cargo check".into(),
            activity: None,
        });
        let rows = scrollback
            .physical_rows(80, false, 30)
            .into_iter()
            .map(|(_, text, _)| text)
            .collect::<Vec<_>>();
        assert!(rows.iter().any(|row| row.contains("Ran 2 commands")));
        assert!(!rows.iter().any(|row| row == "Run cargo test"));
        assert!(!rows.iter().any(|row| row == "Run cargo check"));
    }

    #[test]
    fn selected_dense_tool_group_uses_one_spanning_selection_box() {
        let mut scrollback = Scrollback::new();
        scrollback.set_activity_expanded(true);
        scrollback.apply(ScrollbackMsg::Append(Line::new(
            LineKind::Assistant,
            "before",
        )));
        for (id, header) in [("first", "Run one"), ("second", "Run two")] {
            scrollback.apply(ScrollbackMsg::ToolStart {
                tool_call_id: id.into(),
                header: header.into(),
                activity: None,
            });
        }
        scrollback.apply(ScrollbackMsg::SelectNextTool);
        let mut buffer = Buffer::empty(Rect::new(0, 0, 40, 5));
        scrollback.render(Rect::new(0, 0, 40, 5), &mut buffer);
        let border_rows = (0..5)
            .filter(|row| {
                (0..40).any(|column| {
                    buffer
                        .cell((column, *row))
                        .is_some_and(|cell| cell.symbol() == "│")
                })
            })
            .count();
        assert!(border_rows >= 2, "selected group should span both members");
        assert!((0..40).any(|column| buffer
            .cell((column, 0))
            .is_some_and(|cell| cell.symbol() == "┌")));
    }

    #[test]
    fn selected_wrapped_tool_member_box_spans_physical_rows() {
        let mut scrollback = Scrollback::new();
        scrollback.set_activity_expanded(true);
        scrollback.apply(ScrollbackMsg::Append(Line::new(
            LineKind::Assistant,
            "before",
        )));
        scrollback.apply(ScrollbackMsg::ToolStart {
            tool_call_id: "wrapped".into(),
            header: "Run a very long command header that wraps".into(),
            activity: None,
        });
        scrollback.apply(ScrollbackMsg::SelectNextTool);

        let mut buffer = Buffer::empty(Rect::new(0, 0, 18, 8));
        scrollback.render(Rect::new(0, 0, 18, 8), &mut buffer);
        let selected_background = buffer.cell((5, 0)).expect("wrapped header").style().bg;
        let selected_rows = (0..8)
            .filter(|row| {
                (0..18).any(|column| {
                    buffer
                        .cell((column, *row))
                        .is_some_and(|cell| cell.style().bg == selected_background)
                })
            })
            .count();
        assert!(
            selected_rows >= 3,
            "selected wrapped member should style all physical rows"
        );
    }

    #[test]
    fn selected_wrapped_dense_group_keeps_one_reflowed_selection_surface() {
        let mut scrollback = Scrollback::new();
        scrollback.set_activity_expanded(true);
        scrollback.apply(ScrollbackMsg::Append(Line::new(
            LineKind::Assistant,
            "before",
        )));
        for (id, header) in [
            ("wrapped-first", "Run the first very long command header"),
            ("wrapped-second", "Run the second very long command header"),
        ] {
            scrollback.apply(ScrollbackMsg::ToolStart {
                tool_call_id: id.into(),
                header: header.into(),
                activity: None,
            });
        }
        scrollback.apply(ScrollbackMsg::SelectNextTool);

        let mut buffer = Buffer::empty(Rect::new(0, 0, 18, 10));
        scrollback.render(Rect::new(0, 0, 18, 10), &mut buffer);
        let selected_background = buffer.cell((5, 0)).expect("wrapped header").style().bg;
        let selected_rows = (0..10)
            .filter(|row| {
                (0..18).any(|column| {
                    buffer
                        .cell((column, *row))
                        .is_some_and(|cell| cell.style().bg == selected_background)
                })
            })
            .count();
        assert!(
            selected_rows >= 6,
            "selected dense group should retain both wrapped members"
        );
    }

    fn selected_cell_row(buffer: &Buffer) -> u16 {
        (0..3)
            .find(|row| {
                (0..40).any(|column| {
                    buffer
                        .cell((column, *row))
                        .is_some_and(|cell| cell.symbol() == "⌄")
                })
            })
            .expect("selected row")
    }

    #[test]
    fn running_tool_bullet_advances_as_actor_owned_animation_state() {
        let mut scrollback = Scrollback::new();
        scrollback.set_activity_expanded(true);
        scrollback.apply(ScrollbackMsg::ToolStart {
            tool_call_id: "worker".into(),
            header: "Subagent running: \"inspect\"".into(),
            activity: None,
        });
        let mut first = Buffer::empty(Rect::new(0, 0, 40, 1));
        scrollback.render(Rect::new(0, 0, 40, 1), &mut first);
        assert_eq!(first.cell((0, 0)).expect("running bullet").symbol(), "⋅");
        scrollback.apply(ScrollbackMsg::AdvanceAnimation);
        let mut second = Buffer::empty(Rect::new(0, 0, 40, 1));
        scrollback.render(Rect::new(0, 0, 40, 1), &mut second);
        assert_eq!(
            second.cell((0, 0)).expect("next running bullet").symbol(),
            ":"
        );
        assert!(scrollback.animation_demand());
        scrollback.apply(ScrollbackMsg::ToolEnd {
            tool_call_id: "worker".into(),
            header: "Subagent completed: \"inspect\"".into(),
            activity: None,
            output: Vec::new(),
        });
        assert!(!scrollback.animation_demand());
    }

    #[test]
    fn expanded_tool_member_remains_visible_inside_collapsed_activity() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::Activity, "❙  ◈ Read 1 file"));
        scrollback.append(Line::new(LineKind::Tool, "Read README.md").for_tool("read-1"));
        scrollback.set_tool_mode("read-1", runie_core::types::ToolDisplayMode::Expanded);

        let mut buffer = Buffer::empty(Rect::new(0, 0, 40, 2));
        scrollback.render(Rect::new(0, 0, 40, 2), &mut buffer);
        assert_eq!(buffer.cell((0, 1)).expect("tool bullet").symbol(), "◆");
        assert_eq!(buffer.cell((2, 1)).expect("tool label").symbol(), "R");
    }

    #[test]
    fn dense_tool_groups_preserve_breaks_and_member_positions() {
        let ids = [Some("a"), Some("b"), Some("c"), None, Some("d"), Some("e")];
        assert_eq!(
            dense_tool_group_members(&ids),
            vec![
                Some((0, 3)),
                Some((1, 3)),
                Some((2, 3)),
                None,
                Some((0, 2)),
                Some((1, 2)),
            ]
        );
    }

    #[test]
    fn selected_dense_group_does_not_cross_another_group_of_same_size() {
        let mut scrollback = Scrollback::new();
        for id in ["first-a", "first-b"] {
            scrollback.apply(ScrollbackMsg::ToolStart {
                tool_call_id: id.into(),
                header: id.into(),
                activity: None,
            });
        }
        scrollback.append(Line::new(LineKind::Assistant, "break"));
        for id in ["second-a", "second-b"] {
            scrollback.apply(ScrollbackMsg::ToolStart {
                tool_call_id: id.into(),
                header: id.into(),
                activity: None,
            });
        }
        scrollback.apply(ScrollbackMsg::SelectNextTool);

        assert_eq!(
            scrollback.selected_tool_group_ids(),
            HashSet::from(["first-a".to_owned(), "first-b".to_owned()])
        );
    }

    #[test]
    fn hidden_dense_reveal_uses_the_selected_group_anchor() {
        let mut scrollback = Scrollback::new();
        for prefix in ["first", "second"] {
            for index in 0..=GROK_GROUP_MAX_VISIBLE {
                scrollback.append(
                    Line::new(LineKind::Tool, format!("{prefix}-{index}"))
                        .for_tool(format!("{prefix}-{index}")),
                );
            }
            if prefix == "first" {
                scrollback.append(Line::new(LineKind::Assistant, "break"));
            }
        }

        assert_eq!(
            scrollback.dense_group_anchor_for("second-10"),
            Some("second-0".to_owned())
        );
    }

    #[test]
    fn grok_group_budget_is_named_and_source_defaulted() {
        assert_eq!(GROK_GROUP_MAX_VISIBLE, 10);
    }

    #[test]
    fn selecting_hidden_dense_member_reveals_the_whole_group() {
        let mut scrollback = Scrollback::new();
        scrollback.set_activity_expanded(true);
        scrollback.append(Line::new(LineKind::Activity, "❙  ◈ Ran 12 commands"));
        for index in 1..=12 {
            scrollback.append(
                Line::new(LineKind::Tool, format!("Run command-{index}"))
                    .for_tool(format!("call-{index}")),
            );
        }
        let mut before = Buffer::empty(Rect::new(0, 0, 40, 30));
        scrollback.render(Rect::new(0, 0, 40, 30), &mut before);
        let row_text = |buffer: &Buffer| {
            (0..30)
                .map(|row| {
                    (0..40)
                        .filter_map(|column| buffer.cell((column, row)))
                        .map(|cell| cell.symbol())
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
                .join("\n")
        };
        let _ = row_text(&before);
        for _ in 0..11 {
            scrollback.apply(ScrollbackMsg::SelectNextTool);
        }
        let mut after = Buffer::empty(Rect::new(0, 0, 40, 30));
        scrollback.render(Rect::new(0, 0, 40, 30), &mut after);
        assert!(row_text(&after).contains("Run command-1"));
    }

    #[test]
    fn selecting_hidden_dense_member_centers_the_revealed_member() {
        let mut scrollback = Scrollback::new();
        scrollback.set_activity_expanded(true);
        scrollback.append(Line::new(LineKind::Activity, "❙  ◈ Ran 12 commands"));
        for index in 1..=12 {
            scrollback.append(
                Line::new(LineKind::Tool, format!("Run command-{index}"))
                    .for_tool(format!("call-{index}")),
            );
        }
        for _ in 0..11 {
            scrollback.apply(ScrollbackMsg::SelectNextTool);
        }

        let mut buffer = Buffer::empty(Rect::new(0, 0, 40, 6));
        scrollback.render(Rect::new(0, 0, 40, 6), &mut buffer);
        let selected_row = (0..6)
            .find(|row| {
                (0..40)
                    .filter_map(|column| buffer.cell((column, *row)))
                    .map(|cell| cell.symbol())
                    .collect::<String>()
                    .contains("command-11")
            })
            .unwrap_or_else(|| {
                let visible = (0..6)
                    .map(|row| {
                        (0..40)
                            .filter_map(|column| buffer.cell((column, row)))
                            .map(|cell| cell.symbol())
                            .collect::<String>()
                    })
                    .collect::<Vec<_>>();
                panic!("selected hidden member is visible: {visible:?}");
            });
        assert_eq!(
            selected_row, 4,
            "selected member should be viewport-centered"
        );
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "the workflow status matrix keeps Grok's source variants together"
    )]
    fn workflow_card_uses_grok_status_and_phase_glyph_order() {
        assert_eq!(
            runie_tui_model::workflow_text(
                "Workflow release: ship the release",
                &[
                    ("plan".into(), "done".into()),
                    ("tests".into(), "active".into())
                ],
                "done",
                Some(1_200),
                0,
            ),
            "Workflow release done in 1.2s: ship the release  [plan ✓ · tests ●]"
        );
        assert_eq!(
            runie_tui_model::workflow_text(
                "Workflow release: ship the release",
                &[("tests".into(), "active".into())],
                "active",
                None,
                2,
            ),
            "Workflow release: ship the release  [tests ●]  (2 agents)"
        );
        assert_eq!(
            runie_tui_model::workflow_text(
                "Workflow release: ship the release",
                &[("tests".into(), "failed".into())],
                "failed",
                Some(1_200),
                0,
            ),
            "Workflow release failed in 1.2s: ship the release  [tests ✗]"
        );
        assert_eq!(
            runie_tui_model::workflow_text(
                "Workflow release: ship the release",
                &[],
                "cancelled",
                Some(1_200),
                0,
            ),
            "Workflow release ◌ cancelled after 1.2s: ship the release"
        );
        assert_eq!(
            runie_tui_model::workflow_text(
                "Workflow release: ship the release",
                &[],
                "paused",
                Some(1_200),
                0,
            ),
            "Workflow release paused at 1.2s: ship the release"
        );
    }

    #[test]
    fn workflow_objective_flattens_multiline_source_text() {
        assert_eq!(
            runie_tui_model::workflow_text(
                "Workflow research: compare A\nthen compare B\r\nfinish",
                &[],
                "active",
                None,
                0,
            ),
            "Workflow research: compare A then compare B finish"
        );
    }

    #[allow(
        clippy::too_many_lines,
        reason = "the workflow styling regression covers running, completed, and cancelled cards"
    )]
    #[test]
    fn workflow_card_uses_grok_semantic_header_and_cancelled_text_tokens() {
        let running = styled_line_for(
            LineKind::ToolRunning,
            "◆ Workflow release: ship it  [tests ●]  (2 agents)",
            ThemeKind::GrokNight,
        );
        assert!(running.spans[1].style.add_modifier.contains(Modifier::BOLD));
        assert!(running.spans.iter().any(|span| span.content == "●"
            && span.style.fg == appearance::accent_style_for(ThemeKind::GrokNight).fg));
        assert_eq!(
            running.spans[2].style.fg,
            appearance::muted_style_for(ThemeKind::GrokNight).fg
        );
        assert!(running
            .spans
            .last()
            .expect("workflow metadata span")
            .style
            .add_modifier
            .contains(Modifier::DIM));

        let completed = styled_line_for(
            LineKind::Tool,
            "◆ Workflow release done in 1.2s: ship it  [tests ✓]",
            ThemeKind::GrokNight,
        );
        assert!(completed
            .spans
            .iter()
            .any(|span| span.content == " done in 1.2s"
                && span.style.add_modifier.contains(Modifier::DIM)));

        let cancelled = styled_line_for(
            LineKind::Tool,
            "◆ Workflow release ◌ cancelled after 1.2s: ship it",
            ThemeKind::GrokDay,
        );
        assert!(cancelled.spans[2]
            .style
            .add_modifier
            .contains(Modifier::DIM));
        assert_eq!(
            cancelled.spans[2].style.fg,
            appearance::muted_style_for(ThemeKind::GrokDay).fg
        );
    }
}
