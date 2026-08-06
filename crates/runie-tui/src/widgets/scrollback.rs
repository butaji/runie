//! Scrollback widget: append-only transcript with autoscroll.

use std::collections::{HashMap, HashSet};

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line as RatLine, Span};
use ratatui::widgets::{Paragraph, Widget, Wrap};

use crate::appearance;
use runie_core::types::ThemeKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    User,
    Assistant,
    Reasoning,
    ThinkingStatus,
    Tool,
    ToolResult,
    ToolOutput,
    SessionStart,
    System,
    Separator,
    TurnSummary,
    CompletedAssistant,
    Activity,
}

impl LineKind {
    pub fn style(self) -> Style {
        self.style_for(ThemeKind::GrokNight)
    }

    pub fn style_for(self, theme: ThemeKind) -> Style {
        match self {
            LineKind::User => appearance::base_style_for(theme),
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
            LineKind::ToolResult => appearance::success_style_for(theme),
            LineKind::ToolOutput => appearance::base_style_for(theme),
            LineKind::SessionStart => appearance::muted_style_for(theme),
            LineKind::System => appearance::muted_style_for(theme).add_modifier(Modifier::DIM),
            LineKind::Separator => appearance::muted_style_for(theme).add_modifier(Modifier::DIM),
            LineKind::TurnSummary => appearance::muted_style_for(theme).add_modifier(Modifier::DIM),
            LineKind::CompletedAssistant => appearance::base_style_for(theme),
            LineKind::Activity => appearance::accent_style_for(theme),
        }
    }

    pub fn prefix(self) -> &'static str {
        match self {
            // Grok reserves a three-column transcript gutter before user
            // content: the cursor is at column 5 in the 80-column frame.
            LineKind::User => "   ❯ ",
            LineKind::Assistant => "┃  ",
            LineKind::Reasoning => "┃  ",
            LineKind::ThinkingStatus => "┃  ",
            LineKind::Tool => "◆ ",
            LineKind::ToolResult => "  ↳ ",
            // Structured Grok tools render terminal output directly below the
            // tool header, with a two-column indentation and no result arrow.
            LineKind::ToolOutput => "  ",
            LineKind::SessionStart => "   ",
            LineKind::System => "   * ",
            LineKind::Separator => "",
            LineKind::TurnSummary => "   ",
            LineKind::CompletedAssistant => "   ",
            LineKind::Activity => "❙  ",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub kind: LineKind,
    pub text: String,
    pub tool_call_id: Option<String>,
    has_vpad: bool,
}

impl Line {
    pub fn new(kind: LineKind, text: impl Into<String>) -> Self {
        Self {
            kind,
            text: text.into(),
            tool_call_id: None,
            has_vpad: false,
        }
    }

    pub fn with_vpad(mut self, has_vpad: bool) -> Self {
        self.has_vpad = has_vpad;
        self
    }

    pub fn has_vpad(&self) -> bool {
        self.has_vpad
    }

    pub fn for_tool(mut self, tool_call_id: impl Into<String>) -> Self {
        self.tool_call_id = Some(tool_call_id.into());
        self
    }
}

/// Explicit reducer inputs for the actor-owned transcript projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScrollbackMsg {
    Append(Line),
    AppendTurnSummary(String),
    Clear,
    SetTheme(ThemeKind),
    RemoveKind(LineKind),
    NormalizeLiveCompletedAssistants,
    AddLiveAssistantTimestamp(usize),
    RemoveEmptyAfter(LineKind),
    NormalizeActivitySpacing,
    SetReasoningExpanded(bool),
    SetActivityExpanded(bool),
    ToggleActivityExpanded,
    SetPromptTimestamp(Option<String>),
    SetToolMode(String, runie_core::types::ToolDisplayMode),
    ReplaceLine(usize, String),
    ReplaceLastByKind(LineKind, String),
    AppendToLastByKind(LineKind, String),
    FinalizeAssistant {
        has_reasoning: bool,
        reasoning_expanded: bool,
        summary: String,
    },
    /// Transitional bridge used while event reduction is moved fully into
    /// the actor. It still publishes one immutable state atomically.
    ReplaceSnapshot(Box<Scrollback>),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scrollback {
    lines: Vec<Line>,
    autoscroll: bool,
    scroll_offset: usize,
    reasoning_expanded: bool,
    activity_expanded: bool,
    prompt_timestamp: Option<String>,
    tool_modes: HashMap<String, runie_core::types::ToolDisplayMode>,
    theme: ThemeKind,
}

impl Scrollback {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            autoscroll: true,
            scroll_offset: 0,
            reasoning_expanded: false,
            activity_expanded: false,
            prompt_timestamp: None,
            tool_modes: HashMap::new(),
            theme: ThemeKind::GrokNight,
        }
    }

    pub fn set_theme(&mut self, theme: ThemeKind) {
        self.theme = theme;
    }

    /// Apply one explicit transcript transition. Actor implementations and
    /// compatibility callers share this reducer boundary.
    #[allow(
        clippy::too_many_lines,
        reason = "the reducer keeps all explicit transcript messages in one readable match"
    )]
    pub fn apply(&mut self, message: ScrollbackMsg) -> Option<usize> {
        match message {
            ScrollbackMsg::Append(line) => return Some(self.append(line)),
            ScrollbackMsg::AppendTurnSummary(text) => {
                return Some(self.append(Line::new(LineKind::TurnSummary, text)));
            }
            ScrollbackMsg::Clear => {
                self.clear();
            }
            ScrollbackMsg::SetTheme(theme) => self.set_theme(theme),
            ScrollbackMsg::RemoveKind(kind) => self.remove_kind(kind),
            ScrollbackMsg::NormalizeLiveCompletedAssistants => {
                self.normalize_live_completed_assistants()
            }
            ScrollbackMsg::AddLiveAssistantTimestamp(width) => {
                self.add_live_assistant_timestamp(width)
            }
            ScrollbackMsg::RemoveEmptyAfter(kind) => self.remove_empty_after(kind),
            ScrollbackMsg::NormalizeActivitySpacing => self.normalize_activity_spacing(),
            ScrollbackMsg::SetReasoningExpanded(expanded) => self.set_reasoning_expanded(expanded),
            ScrollbackMsg::SetActivityExpanded(expanded) => self.set_activity_expanded(expanded),
            ScrollbackMsg::ToggleActivityExpanded => {
                self.set_activity_expanded(!self.activity_expanded);
            }
            ScrollbackMsg::SetPromptTimestamp(timestamp) => self.set_prompt_timestamp(timestamp),
            ScrollbackMsg::SetToolMode(id, mode) => self.set_tool_mode(id, mode),
            ScrollbackMsg::ReplaceLine(index, text) => {
                if let Some(line) = self.line_mut(index) {
                    line.text = text;
                }
            }
            ScrollbackMsg::ReplaceLastByKind(kind, text) => {
                if let Some(line) = self.last_mut_by_kind(kind) {
                    line.text = text;
                }
            }
            ScrollbackMsg::AppendToLastByKind(kind, text) => {
                if let Some(line) = self.last_mut_by_kind(kind) {
                    line.text.push_str(&text);
                } else {
                    self.append(Line::new(kind, text));
                }
            }
            ScrollbackMsg::FinalizeAssistant {
                has_reasoning,
                reasoning_expanded,
                summary,
            } => {
                if !has_reasoning || reasoning_expanded {
                    self.remove_kind(LineKind::ThinkingStatus);
                } else {
                    if let Some(thinking) = self.last_mut_by_kind(LineKind::ThinkingStatus) {
                        thinking.kind = LineKind::TurnSummary;
                        thinking.text = summary;
                    }
                    self.remove_kind(LineKind::Reasoning);
                }
            }
            ScrollbackMsg::ReplaceSnapshot(snapshot) => *self = *snapshot,
        }
        None
    }

    pub fn theme(&self) -> ThemeKind {
        self.theme
    }

    pub fn append(&mut self, line: Line) -> usize {
        let index = self.lines.len();
        self.lines.push(line);
        if self.autoscroll {
            // Hold offset so the tail is in view after the next render
            // (the actual clamp happens in `render` once we know area height).
            self.scroll_offset = self.lines.len();
        }
        index
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.scroll_offset = 0;
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
        self.reasoning_expanded = expanded;
    }

    pub fn reasoning_expanded(&self) -> bool {
        self.reasoning_expanded
    }

    /// Set Grok-compatible grouped-tool display mode. Collapsed mode keeps
    /// the activity summary and hides member tool/output rows.
    pub fn set_activity_expanded(&mut self, expanded: bool) {
        self.activity_expanded = expanded;
    }

    pub fn set_prompt_timestamp(&mut self, timestamp: Option<String>) {
        self.prompt_timestamp = timestamp;
    }

    pub fn activity_expanded(&self) -> bool {
        self.activity_expanded
    }

    pub fn set_tool_mode(
        &mut self,
        tool_call_id: impl Into<String>,
        mode: runie_core::types::ToolDisplayMode,
    ) {
        self.tool_modes.insert(tool_call_id.into(), mode);
    }

    /// Find the index of the first line whose `text` contains the needle.
    pub fn find_first_containing(&self, needle: &str) -> Option<usize> {
        self.lines.iter().position(|l| l.text.contains(needle))
    }

    /// Find all line indices whose `text` contains the needle.
    pub fn find_all_containing(&self, needle: &str) -> Vec<usize> {
        self.lines
            .iter()
            .enumerate()
            .filter_map(|(i, l)| {
                if l.text.contains(needle) {
                    Some(i)
                } else {
                    None
                }
            })
            .collect()
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
        // Wrap-aware: each Line is one logical row that may wrap to multiple
        // physical rows. We approximate by giving each line 1 "slot" plus
        // overflow based on text length and area width.
        let physical_rows = self.physical_rows(area.width as usize);
        let total = physical_rows.len();
        let visible = area.height as usize;
        let compact_scroll_lead =
            if total > visible + crate::layout::COMPACT_SCROLL_OVERFLOW_THRESHOLD {
                crate::layout::COMPACT_SCROLL_OVERFLOW_LEAD_ROWS
            } else {
                crate::layout::COMPACT_SCROLL_LEAD_ROWS
            };
        // Clamp scroll_offset so the tail is visible.
        if total > visible {
            let max_offset = total - visible;
            if self.autoscroll {
                self.scroll_offset = if area.width < 50 {
                    max_offset.saturating_sub(compact_scroll_lead)
                } else {
                    max_offset
                };
            } else if self.scroll_offset > max_offset {
                self.scroll_offset = max_offset;
            }
        } else {
            self.scroll_offset = 0;
        }

        let start = self.scroll_offset;
        let end = (start + visible).min(total);

        if start >= end {
            // Nothing to render. Avoid passing an empty slice to ratatui's
            // Paragraph/Line, which can panic on some versions.
            return;
        }

        for (row, (kind, text, code_row)) in physical_rows[start..end].iter().enumerate() {
            let line = if *code_row {
                styled_code_line(text, self.theme)
            } else {
                styled_line_for(*kind, text, self.theme)
            };
            Paragraph::new(line).wrap(Wrap { trim: false }).render(
                Rect {
                    x: area.x,
                    y: area.y + row as u16,
                    width: area.width,
                    height: 1,
                },
                buf,
            );
        }
    }

    #[allow(
        clippy::cognitive_complexity,
        clippy::too_many_lines,
        reason = "physical row projection keeps fold, markdown, and wrapping rules together"
    )]
    fn physical_rows(&self, width: usize) -> Vec<(LineKind, String, bool)> {
        let mut rows = Vec::new();
        let mut code_block = false;
        let mut truncated_output = HashSet::new();
        let mut user_vpad_emitted = false;
        let mut skip_full_user_separator = false;
        for (line_index, line) in self.lines.iter().enumerate() {
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
            if line.has_vpad() && width >= 70 && !user_vpad_emitted {
                // Grok's prompt block enables vertical padding in full mode;
                // the narrow pager variant suppresses it.
                rows.push((LineKind::System, String::new(), false));
                user_vpad_emitted = true;
                skip_full_user_separator = true;
            }
            if width >= 50 && line.kind == LineKind::TurnSummary {
                rows.push((LineKind::System, String::new(), false));
            }
            let tool_mode = line
                .tool_call_id
                .as_ref()
                .and_then(|id| self.tool_modes.get(id));
            if matches!(
                tool_mode,
                Some(runie_core::types::ToolDisplayMode::Collapsed)
            ) && matches!(line.kind, LineKind::ToolOutput | LineKind::ToolResult)
            {
                continue;
            }
            if matches!(
                tool_mode,
                Some(runie_core::types::ToolDisplayMode::Truncated)
            ) && matches!(line.kind, LineKind::ToolOutput | LineKind::ToolResult)
                && line
                    .tool_call_id
                    .as_ref()
                    .is_some_and(|id| !truncated_output.insert(id.clone()))
            {
                continue;
            }
            if !self.activity_expanded
                && matches!(
                    line.kind,
                    LineKind::Tool | LineKind::ToolOutput | LineKind::ToolResult
                )
                && line.text != "session_start"
            {
                continue;
            }
            let source = if line.kind == LineKind::Reasoning && !self.reasoning_expanded {
                "Thought"
            } else {
                line.text.as_str()
            };
            let fence = line.kind == LineKind::Assistant && is_fence(source);
            let parts: Vec<_> = source.split('\n').collect();
            for (index, part) in parts.iter().enumerate() {
                let prefix = if line.kind == LineKind::TurnSummary && width >= 50 {
                    "     "
                } else if width < 70
                    && matches!(
                        line.kind,
                        LineKind::Assistant | LineKind::Reasoning | LineKind::ThinkingStatus
                    )
                {
                    "┃"
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
                    if let Some(timestamp) = self.prompt_timestamp.as_deref() {
                        let timestamp_width = timestamp.chars().count();
                        let target = width.saturating_sub(timestamp_width);
                        if text.chars().count() > target {
                            let chars: Vec<_> = text.chars().collect();
                            let head: String = chars[..target].iter().collect();
                            let rest: String = chars[target..].iter().collect();
                            rows.push((
                                LineKind::CompletedAssistant,
                                format!("{head}{timestamp}"),
                                false,
                            ));
                            append_wrapped(&mut rows, line.kind, rest, code_block && !fence, width);
                            continue;
                        }
                        text.push_str(&" ".repeat(target - text.chars().count()));
                        text.push_str(timestamp);
                    }
                }
                if line.kind == LineKind::User && index == 0 && self.prompt_timestamp.is_some() {
                    append_user_with_timestamp(
                        &mut rows,
                        text,
                        self.prompt_timestamp.as_deref().unwrap_or_default(),
                        width,
                    );
                } else {
                    append_wrapped(&mut rows, line.kind, text, code_block && !fence, width);
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
                    let border = format!("{}{}", border_prefix, table_bottom_border(part));
                    append_wrapped(&mut rows, line.kind, border, false, width);
                }
            }
            if fence {
                code_block = !code_block;
            }
        }
        rows
    }
}

fn append_wrapped(
    rows: &mut Vec<(LineKind, String, bool)>,
    kind: LineKind,
    text: String,
    code: bool,
    width: usize,
) {
    if width == 0 || text.chars().count() <= width {
        rows.push((kind, text, code));
        return;
    }
    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() {
        let head: String = chars.drain(..width.min(chars.len())).collect();
        rows.push((kind, head, code));
    }
}

fn append_user_with_timestamp(
    rows: &mut Vec<(LineKind, String, bool)>,
    text: String,
    timestamp: &str,
    width: usize,
) {
    // Grok leaves a wide timestamp gutter on the first prompt row.
    let reserved = timestamp.chars().count() + 8;
    let first_width = width.saturating_sub(reserved);
    let mut chars: Vec<char> = text.chars().collect();
    let mut split = first_width.min(chars.len());
    while split > 0 && split < chars.len() && !chars[split].is_whitespace() {
        split -= 1;
    }
    let first: String = chars.drain(..split).collect();
    rows.push((
        LineKind::User,
        format!("{first}{:>reserved$}", timestamp, reserved = reserved),
        false,
    ));
    let indent = " ".repeat(LineKind::User.prefix().chars().count());
    let rest: String = chars.into_iter().collect();
    append_wrapped_words(
        rows,
        LineKind::User,
        format!("{indent}{}", rest.trim_start()),
        first_width,
    );
}

fn append_wrapped_words(
    rows: &mut Vec<(LineKind, String, bool)>,
    kind: LineKind,
    text: String,
    width: usize,
) {
    let leading: String = text.chars().take_while(|ch| ch.is_whitespace()).collect();
    let mut line = leading.clone();
    for word in text.split_whitespace() {
        let candidate = if line.trim().is_empty() {
            word.to_owned()
        } else {
            format!("{line} {word}")
        };
        if !line.trim().is_empty() && candidate.chars().count() > width {
            rows.push((kind, std::mem::replace(&mut line, leading.clone()), false));
        }
        if line.trim().is_empty() {
            line.push_str(word);
        } else {
            line.push(' ');
            line.push_str(word);
        }
    }
    if !line.is_empty() {
        rows.push((kind, line, false));
    }
}

/// Render the small CommonMark subset that is visible in Grok's normal
/// transcript: headings, bullets, inline emphasis, and inline code. Keeping
/// this at the widget boundary means replay events remain the single source
/// of truth and no test needs a terminal process.
fn is_fence(text: &str) -> bool {
    text.trim_start()
        .strip_prefix("┃ ")
        .unwrap_or(text)
        .starts_with("```")
}

fn styled_code_line(text: &str, theme: ThemeKind) -> RatLine<'static> {
    RatLine::from(Span::styled(
        text.to_owned(),
        appearance::base_style_for(theme).add_modifier(Modifier::DIM),
    ))
}

#[allow(
    clippy::too_many_lines,
    reason = "semantic feed card styling keeps Grok header variants together"
)]
#[cfg(test)]
fn styled_line(kind: LineKind, text: &str) -> RatLine<'static> {
    styled_line_for(kind, text, ThemeKind::GrokNight)
}

#[allow(
    clippy::too_many_lines,
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
    if kind == LineKind::Tool {
        let Some(header_start) = text.find("◆ ") else {
            return RatLine::from(text.to_owned()).style(style);
        };
        let split = header_start + "◆ ".len();
        let (prefix, body) = text.split_at(split);
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
    if kind != LineKind::Assistant {
        return RatLine::from(text.to_owned()).style(style);
    }
    styled_assistant_line(text, style)
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
    let trimmed = text.trim();
    trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.matches('|').count() >= 2
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
    text.trim()
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .all(|cell| !cell.is_empty() && cell.chars().all(|ch| matches!(ch, '-' | ':' | ' ')))
}

fn table_bottom_border(text: &str) -> String {
    let widths = text
        .trim()
        .trim_matches('|')
        .split('|')
        .map(|cell| "─".repeat(cell.trim().chars().count() + 2))
        .collect::<Vec<_>>();
    format!("└{}┘", widths.join("┴"))
}

fn atx_heading(text: &str) -> Option<&str> {
    let hashes = text.chars().take_while(|ch| *ch == '#').count();
    (1..=6)
        .contains(&hashes)
        .then(|| text.get(hashes..)?.strip_prefix(' '))
        .flatten()
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
    use ratatui::style::Color;

    #[test]
    fn append_and_len() {
        let mut s = Scrollback::new();
        s.append(Line::new(LineKind::User, "hi"));
        assert_eq!(s.len(), 1);
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
            LineKind::ToolResult.style().fg,
            Some(Color::Rgb(158, 206, 106))
        );
        assert!(LineKind::System
            .style()
            .add_modifier
            .contains(Modifier::DIM));
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
        let mut buffer = Buffer::empty(Rect::new(2, 0, 76, 1));
        scrollback.render(Rect::new(2, 0, 76, 1), &mut buffer);
        assert_eq!(buffer.cell((2, 0)).expect("gutter").symbol(), " ");
        assert_eq!(buffer.cell((5, 0)).expect("Grok user cursor").symbol(), "❯");
        assert_eq!(
            buffer.cell((7, 0)).expect("first user letter").symbol(),
            "P"
        );
    }

    #[test]
    fn turn_summary_uses_groks_column_six_gutter() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::TurnSummary, "Worked for 2.3s"));
        let mut buffer = Buffer::empty(Rect::new(0, 0, 30, 1));
        scrollback.render(Rect::new(0, 0, 30, 1), &mut buffer);
        assert_eq!(buffer.cell((0, 0)).expect("first gutter").symbol(), " ");
        assert_eq!(buffer.cell((2, 0)).expect("fifth gutter").symbol(), " ");
        assert_eq!(buffer.cell((3, 0)).expect("summary start").symbol(), "W");
    }

    #[test]
    fn live_completed_assistant_keeps_primary_body_style() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::Assistant, "Hello from Runie"));
        scrollback.normalize_live_completed_assistants();

        assert_eq!(scrollback.lines()[0].kind, LineKind::CompletedAssistant);
        let mut buffer = Buffer::empty(Rect::new(0, 0, 30, 1));
        scrollback.render(Rect::new(0, 0, 30, 1), &mut buffer);
        let cell = buffer.cell((3, 0)).expect("assistant body");
        assert_eq!(cell.fg, Color::Rgb(225, 225, 225));
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
}
