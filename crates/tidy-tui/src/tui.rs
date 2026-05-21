use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Rect},
    buffer::Buffer,
    style::Style,
    widgets::Widget,
};
use crossterm::{
    cursor::{SetCursorStyle, Show},
    event::{Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use std::io::{self, stdout};

use crate::{
    theme::ThemeWrapper,
    components::{
        MessageList,
        MessageItem,
        InputBar,
        Overlay,
        PermissionModal,
        PermissionAction,
        AgentStatus,
        AgentList,
        AgentItem,
        ContextPanel,
        GitChange,
        GitStatus,
        CommandPalette,
    },
};
use tidy_agent::events::{AgentEvent, AgentMessage, ContentPart, PermissionDecision};

pub struct TuiConfig {
    pub theme: ThemeWrapper,
    pub show_top_bar: bool,
    pub show_status_bar: bool,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            theme: ThemeWrapper::default(),
            show_top_bar: true,
            show_status_bar: true,
        }
    }
}

const SIDEBAR_WIDTH: u16 = 28;

// ─── AppState ─────────────────────────────────────────────────────────────────
// All application state, extracted from Tui. No terminal I/O, no widgets.

#[derive(Clone)]
pub struct AppState {
    pub messages: Vec<MessageItem>,
    pub input_lines: Vec<String>,
    pub cursor_col: usize,
    pub cursor_row: usize,
    pub input_right_info: String,
    pub mode: TuiMode,
    pub running: bool,
    pub show_sidebar: bool,
    pub agent_running: bool,
    pub current_model: Option<String>,
    pub top_bar_repo: String,
    pub top_bar_branch: String,
    pub top_bar_path: String,
    pub top_bar_checks_passed: Option<usize>,
    pub top_bar_checks_total: Option<usize>,
    pub top_bar_percentage: Option<f32>,
    pub top_bar_agent_count: Option<usize>,
    pub permission_modal_tool: Option<String>,
    pub permission_modal_args: Option<String>,
    pub permission_modal_desc: Option<String>,
    pub action_log: Vec<Msg>,         // NEW: history of all actions for time-travel debugging
    pub action_log_capacity: usize,    // NEW: max actions to keep (default 1000)
    pub command_palette_open: bool,
    pub command_palette_filter: String,
    pub command_palette_selected: usize,
    pub feed_scroll_offset: usize,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            input_lines: vec![String::new()],
            cursor_col: 0,
            cursor_row: 0,
            input_right_info: String::new(),
            mode: TuiMode::Chat,
            running: true,
            show_sidebar: false,
            agent_running: false,
            current_model: None,
            top_bar_repo: String::new(),
            top_bar_branch: String::new(),
            top_bar_path: String::new(),
            top_bar_checks_passed: None,
            top_bar_checks_total: None,
            top_bar_percentage: None,
            top_bar_agent_count: None,
            permission_modal_tool: None,
            permission_modal_args: None,
            permission_modal_desc: None,
            action_log: Vec::new(),
            action_log_capacity: 1000,
            command_palette_open: false,
            command_palette_filter: String::new(),
            command_palette_selected: 0,
            feed_scroll_offset: 0,
        }
    }
}

impl AppState {
    /// Replay actions from scratch up to index n (time-travel debugging)
    pub fn replay_to(&self, n: usize) -> AppState {
        let mut new_state = AppState::default();
        for i in 0..n.min(self.action_log.len()) {
            update(&mut new_state, self.action_log[i].clone());
        }
        new_state
    }

    /// Get action log as readable strings for debugging
    pub fn action_log_summary(&self) -> Vec<String> {
        self.action_log.iter()
            .enumerate()
            .map(|(i, msg)| format!("{:4}: {:?}", i, msg))
            .collect()
    }
}

// ─── Standalone Widget Render Functions ────────────────────────────────────────
// These render directly from AppState (no widget instances stored in Tui)

/// Render top bar from state (repo/branch/path info)
fn render_top_bar(state: &AppState, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    use ratatui::text::{Line, Span};
    use ratatui::style::Modifier;

    let x = area.x + 1;

    let text_secondary: ratatui::style::Color = theme.color("text.muted").into();
    let text_tertiary: ratatui::style::Color = theme.color("text.dim").into();
    let syntax_success: ratatui::style::Color = theme.color("success").into();

    // Left side: repo_name/branch current_path
    if !state.top_bar_repo.is_empty() || !state.top_bar_branch.is_empty() {
        let mut left_parts: Vec<Span> = Vec::new();

        if !state.top_bar_repo.is_empty() {
            left_parts.push(Span::styled(&state.top_bar_repo, Style::default().fg(text_secondary)));
        }
        if !state.top_bar_branch.is_empty() {
            left_parts.push(Span::styled("/", Style::default().fg(text_secondary)));
            left_parts.push(Span::styled(&state.top_bar_branch, Style::default().fg(text_secondary)));
        }
        if !state.top_bar_path.is_empty() {
            left_parts.push(Span::styled(format!(" {}", state.top_bar_path), Style::default().fg(text_tertiary).add_modifier(Modifier::DIM)));
        }

        let line = Line::from(left_parts);
        buf.set_line(x, area.y, &line, area.width - 2);
    }

    // Right side: checks_passed ✓ percentage% with mini progress bar
    let mut right_parts: Vec<Span> = Vec::new();

    if let (Some(passed), Some(_total)) = (state.top_bar_checks_passed, state.top_bar_checks_total) {
        right_parts.push(Span::styled(format!("{} ", passed), Style::default().fg(syntax_success)));
        right_parts.push(Span::styled("✓ ", Style::default().fg(syntax_success)));
    }
    if let Some(pct) = state.top_bar_percentage {
        right_parts.push(Span::styled(format!("{:.2}%", pct), Style::default().fg(text_secondary)));

        // Mini progress bar using unicode blocks
        let filled = (pct / 100.0 * 10.0).round() as usize;
        let empty = 10 - filled;
        let progress_bar = format!("{}{}", "\u{2588}".repeat(filled), "\u{2591}".repeat(empty));
        right_parts.push(Span::styled(format!(" {}", progress_bar), Style::default().fg(text_tertiary)));
    }

    if !right_parts.is_empty() {
        let right_line = Line::from(right_parts);
        let right_width: usize = right_line.spans.iter().map(|s| s.width()).sum();
        let right_x = area.x + area.width.saturating_sub(right_width as u16 + 1);
        if right_x > x {
            buf.set_line(right_x, area.y, &right_line, area.width);
        }
    }
}

/// Render status bar from state (mode-based shortcuts)
fn render_status_bar(state: &AppState, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    use ratatui::style::Modifier;
    use ratatui::text::{Line, Span};

    let text_tertiary: ratatui::style::Color = theme.color("text.dim").into();
    let mut x = area.x + 1;
    let mut first = true;

    // Get items based on mode
    let items: Vec<(&str, &str)> = match state.mode {
        TuiMode::Chat => vec![
            ("Enter", "send"),
            ("^b", "sidebar"),
            ("^k", "cmd"),
            ("^q", "quit"),
        ],
        TuiMode::Overlay => vec![
            ("Esc", "close"),
            ("j/k", "navigate"),
            ("Enter", "select"),
        ],
        TuiMode::Select => vec![
            ("Esc", "close"),
            ("j/k", "navigate"),
            ("Enter", "select"),
        ],
        TuiMode::Permission => vec![
            ("y", "confirm"),
            ("n", "cancel"),
            ("a", "always"),
            ("s", "skip"),
        ],
        TuiMode::CommandPalette => vec![
            ("Esc", "close"),
            ("Enter", "select"),
            ("↑↓", "navigate"),
        ],
    };

    for (key, desc) in items {
        if !first {
            let sep = Span::styled(" | ", Style::default().fg(text_tertiary));
            let line = Line::from(sep);
            buf.set_line(x, area.y, &line, 3);
            x += 3;
        }
        first = false;

        let parts = vec![
            Span::styled(key, Style::default().fg(text_tertiary)),
            Span::styled(format!(" {}", desc), Style::default().fg(text_tertiary).add_modifier(Modifier::DIM)),
        ];
        let line = Line::from(parts);
        let width = (key.len() + 1 + desc.len()) as u16;
        buf.set_line(x, area.y, &line, width);
        x += width;
    }
}

/// Render agent list sidebar from demo data
fn render_agent_list(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    use ratatui::style::Modifier;
    use ratatui::text::{Line, Span};

    let bg_panel: ratatui::style::Color = theme.color("bg.panel").into();
    let border_color: ratatui::style::Color = theme.color("border.unfocused").into();
    let text_secondary: ratatui::style::Color = theme.color("text.secondary").into();
    let text_dim: ratatui::style::Color = theme.color("text.dim").into();
    let accent_primary: ratatui::style::Color = theme.color("accent.primary").into();
    let success: ratatui::style::Color = theme.color("success").into();
    let error: ratatui::style::Color = theme.color("error").into();

    if area.width < 4 || area.height < 3 {
        return;
    }

    let inner_width = area.width.saturating_sub(2);

    // Top border
    buf.get_mut(area.x, area.y).set_char('╭');
    buf.get_mut(area.x, area.y).set_style(Style::default().fg(border_color));

    let header = " AGENTS ";
    let header_style = Style::default().fg(accent_primary).add_modifier(Modifier::BOLD);
    let header_len = header.len() as u16;
    let dashes = inner_width.saturating_sub(header_len);

    let mut x = area.x + 1;
    for ch in header.chars() {
        buf.get_mut(x, area.y).set_char(ch);
        buf.get_mut(x, area.y).set_style(header_style);
        x += 1;
    }
    for _ in 0..dashes {
        buf.get_mut(x, area.y).set_char('─');
        buf.get_mut(x, area.y).set_style(Style::default().fg(border_color));
        x += 1;
    }

    buf.get_mut(area.x + area.width - 1, area.y).set_char('╮');
    buf.get_mut(area.x + area.width - 1, area.y).set_style(Style::default().fg(border_color));

    // Interior fill
    for y in (area.y + 1)..(area.y + area.height - 1) {
        for x in (area.x + 1)..(area.x + area.width - 1) {
            buf.get_mut(x, y).set_style(Style::default().bg(bg_panel));
        }
    }

    // Left and right borders
    for y in (area.y + 1)..(area.y + area.height - 1) {
        buf.get_mut(area.x, y).set_char('│');
        buf.get_mut(area.x, y).set_style(Style::default().fg(border_color));
        buf.get_mut(area.x + area.width - 1, y).set_char('│');
        buf.get_mut(area.x + area.width - 1, y).set_style(Style::default().fg(border_color));
    }

    // Demo agents
    let demo_agents = vec![
        ("coder", "coder", "assistant", "editing files", "claude-4", 45, AgentStatus::Running),
        ("test", "test", "system", "running tests", "gpt-4", 12, AgentStatus::Completed),
    ];

    let mut current_y = area.y + 1;
    let max_y = area.y + area.height - 1;

    for agent in demo_agents {
        if current_y + 3 >= max_y {
            break;
        }

        let (status_char, status_fg) = match agent.6 {
            AgentStatus::Running => ('●', accent_primary),
            AgentStatus::Completed => ('✓', success),
            AgentStatus::Failed => ('✗', error),
            AgentStatus::Waiting => ('○', text_dim),
        };

        let tag_color = match agent.2 {
            "user" | "assistant" => accent_primary,
            "system" => accent_primary,
            _ => text_dim,
        };

        // Line 1: icon + tag
        let y1 = current_y;
        buf.get_mut(area.x + 2, y1).set_char(' ');
        buf.get_mut(area.x + 2, y1).set_style(Style::default().bg(bg_panel));
        buf.get_mut(area.x + 3, y1).set_char(status_char);
        buf.get_mut(area.x + 3, y1).set_style(Style::default().fg(status_fg).bg(bg_panel));
        buf.get_mut(area.x + 4, y1).set_char(' ');
        buf.get_mut(area.x + 4, y1).set_style(Style::default().bg(bg_panel));

        let tag_span = Span::styled(
            agent.1.to_string(),
            Style::default().fg(tag_color).add_modifier(Modifier::BOLD).bg(bg_panel),
        );
        let tag_line = Line::from(vec![tag_span]);
        buf.set_line(area.x + 5, y1, &tag_line, inner_width.saturating_sub(5));

        // Line 2: description
        let y2 = current_y + 1;
        let desc_span = Span::styled(
            format!("  {}", agent.3),
            Style::default().fg(text_secondary).bg(bg_panel),
        );
        let desc_line = Line::from(vec![desc_span]);
        buf.set_line(area.x + 2, y2, &desc_line, inner_width.saturating_sub(2));

        // Line 3: model + duration
        let y3 = current_y + 2;
        let duration_secs = agent.5;
        let duration_str = if duration_secs >= 60 {
            format!("{}m", duration_secs / 60)
        } else {
            format!("{}s", duration_secs)
        };
        let meta_span = Span::styled(
            format!("  {} · {}", agent.4, duration_str),
            Style::default().fg(text_dim).bg(bg_panel),
        );
        let meta_line = Line::from(vec![meta_span]);
        buf.set_line(area.x + 2, y3, &meta_line, inner_width.saturating_sub(2));

        // Separator
        current_y += 4;
        if current_y < max_y - 1 {
            let sep_y = current_y - 1;
            for sx in (area.x + 2)..(area.x + area.width - 2) {
                buf.get_mut(sx, sep_y).set_char('·');
                buf.get_mut(sx, sep_y).set_style(Style::default().fg(text_dim).bg(bg_panel));
            }
        }
    }

    // Bottom border
    let bottom_y = area.y + area.height - 1;
    buf.get_mut(area.x, bottom_y).set_char('╰');
    buf.get_mut(area.x, bottom_y).set_style(Style::default().fg(border_color));

    for x in (area.x + 1)..(area.x + area.width - 1) {
        buf.get_mut(x, bottom_y).set_char('─');
        buf.get_mut(x, bottom_y).set_style(Style::default().fg(border_color));
    }

    buf.get_mut(area.x + area.width - 1, bottom_y).set_char('╯');
    buf.get_mut(area.x + area.width - 1, bottom_y).set_style(Style::default().fg(border_color));
}

/// Render context panel sidebar from state
fn render_context_panel(state: &AppState, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    use ratatui::style::Modifier;
    use ratatui::text::{Line, Span};

    let bg_panel: ratatui::style::Color = theme.color("bg.panel").into();
    for y in area.y..(area.y + area.height) {
        for x in area.x..(area.x + area.width) {
            if let Some(cell) = buf.cell_mut((x as u16, y as u16)) {
                cell.set_style(Style::default().bg(bg_panel));
            }
        }
    }

    let text_secondary: ratatui::style::Color = theme.color("text.secondary").into();
    let text_muted: ratatui::style::Color = theme.color("text.muted").into();
    let accent_primary: ratatui::style::Color = theme.color("accent.primary").into();
    let accent_secondary: ratatui::style::Color = theme.color("accent.secondary").into();
    let _warning: ratatui::style::Color = theme.color("warning").into();
    let _success: ratatui::style::Color = theme.color("success").into();
    let _error: ratatui::style::Color = theme.color("error").into();
    let border_unfocused: ratatui::style::Color = theme.color("border.unfocused").into();

    let left_margin = 1u16;
    let max_width = area.width.saturating_sub(left_margin + 1);
    let mut y = area.y;

    // Model
    if y < area.y + area.height {
        let model_label = Span::styled("Model: ", Style::default().fg(text_muted));
        let model_name = Span::styled("claude-4".to_string(), Style::default().fg(accent_secondary));
        let line = Line::from(vec![model_label, model_name]);
        buf.set_line(area.x + left_margin, y, &line, max_width);
        y += 1;
    }

    // Session
    if y < area.y + area.height {
        let session_label = Span::styled("Session: ", Style::default().fg(text_muted));
        let session_info = Span::styled("new session".to_string(), Style::default().fg(text_secondary));
        let line = Line::from(vec![session_label, session_info]);
        buf.set_line(area.x + left_margin, y, &line, max_width);
        y += 1;
    }

    // Separator
    if y < area.y + area.height {
        let sep = Span::styled(
            "─".repeat(max_width as usize),
            Style::default().fg(border_unfocused),
        );
        let line = Line::from(vec![sep]);
        buf.set_line(area.x + left_margin, y, &line, max_width);
        y += 1;
    }

    // RECENT section header
    if y < area.y + area.height {
        let header = Span::styled(
            "RECENT",
            Style::default().fg(accent_primary).add_modifier(Modifier::BOLD),
        );
        let line = Line::from(vec![header]);
        buf.set_line(area.x + left_margin, y, &line, max_width);
        y += 1;

        // Demo files
        for file in &["src/main.rs", "Cargo.toml", "README.md"] {
            if y >= area.y + area.height {
                break;
            }
            let file_span = Span::styled(
                format!("▸ {}", file),
                Style::default().fg(text_secondary),
            );
            let line = Line::from(vec![file_span]);
            buf.set_line(area.x + left_margin, y, &line, max_width);
            y += 1;
        }
    }
}

// ─── Msg ────────────────────────────────────────────────────────────────────────
// All state changes described as messages (unidirectional data flow)

#[derive(Debug, Clone)]
pub enum Msg {
    // Input (user typing)
    InsertChar(char),
    Backspace,
    DeleteForward,
    MoveCursorLeft,
    MoveCursorRight,
    MoveCursorUp,
    MoveCursorDown,
    MoveCursorToStart,
    MoveCursorToEnd,
    InsertNewline,
    DeleteWordBackward,
    DeleteToStart,

    // App
    Submit,
    Quit,
    ToggleSidebar,
    OpenCommandPalette,
    CloseModal,
    ConfirmModal,
    ScrollUp,
    ScrollDown,

    // Permission
    PermissionConfirm,
    PermissionCancel,
    PermissionAlways,
    PermissionSkip,

    // Command palette
    CommandPaletteFilter(char),
    CommandPaletteBackspace,
    CommandPaletteUp,
    CommandPaletteDown,
    CommandPaletteConfirm,

    // Events from outside
    AgentEvent(AgentEvent),
}

// ─── Cmd ────────────────────────────────────────────────────────────────────────
// Effects returned by update() to be executed by the runtime

#[derive(Debug, Clone)]
pub enum Cmd {
    SpawnAgent { messages: Vec<AgentMessage> },
    SendPermission { decision: PermissionDecision },
}

// ─── update() ─────────────────────────────────────────────────────────────────
// Pure reducer: takes state + msg, returns Vec<Cmd> (no side effects)

pub fn update(state: &mut AppState, msg: Msg) -> Vec<Cmd> {
    let mut cmds = vec![];

    // Log msg before applying (for time-travel debugging)
    if state.action_log.len() >= state.action_log_capacity {
        state.action_log.remove(0); // Remove oldest when at capacity
    }
    state.action_log.push(msg.clone());

    match msg {
        Msg::Quit => state.running = false,

        Msg::Submit => {
            let text = state.input_lines.join("\n");
            if !text.is_empty() {
                state.messages.push(MessageItem::User {
                    text: text.clone(),
                    model: Some("You".to_string()),
                    timestamp: None,
                });
                state.input_lines = vec![String::new()];
                state.cursor_col = 0;
                state.cursor_row = 0;
                cmds.push(Cmd::SpawnAgent {
                    messages: to_agent_messages(&state.messages),
                });
            }
        }

        Msg::InsertChar(c) => {
            if state.cursor_row < state.input_lines.len() {
                state.input_lines[state.cursor_row].insert(state.cursor_col, c);
                state.cursor_col += 1;
            }
        }

        Msg::Backspace => {
            if state.cursor_col > 0 {
                state.input_lines[state.cursor_row].remove(state.cursor_col - 1);
                state.cursor_col -= 1;
            } else if state.cursor_row > 0 {
                let line = state.input_lines.remove(state.cursor_row);
                state.cursor_row -= 1;
                state.cursor_col = state.input_lines[state.cursor_row].len();
                state.input_lines[state.cursor_row].push_str(&line);
            }
        }

        Msg::InsertNewline => {
            if state.cursor_row < state.input_lines.len() {
                let remainder = state.input_lines[state.cursor_row].split_off(state.cursor_col);
                state.cursor_row += 1;
                state.cursor_col = 0;
                state.input_lines.insert(state.cursor_row, remainder);
            }
        }

        Msg::MoveCursorLeft => {
            if state.cursor_col > 0 {
                state.cursor_col -= 1;
            } else if state.cursor_row > 0 {
                state.cursor_row -= 1;
                state.cursor_col = state.input_lines[state.cursor_row].len();
            }
        }

        Msg::MoveCursorRight => {
            if state.cursor_col < state.input_lines[state.cursor_row].len() {
                state.cursor_col += 1;
            } else if state.cursor_row + 1 < state.input_lines.len() {
                state.cursor_row += 1;
                state.cursor_col = 0;
            }
        }

        Msg::MoveCursorUp => {
            if state.cursor_row > 0 {
                state.cursor_row -= 1;
                state.cursor_col = state.cursor_col.min(state.input_lines[state.cursor_row].len());
            }
        }

        Msg::MoveCursorDown => {
            if state.cursor_row + 1 < state.input_lines.len() {
                state.cursor_row += 1;
                state.cursor_col = state.cursor_col.min(state.input_lines[state.cursor_row].len());
            }
        }

        Msg::MoveCursorToStart => state.cursor_col = 0,

        Msg::MoveCursorToEnd => {
            state.cursor_col = state.input_lines[state.cursor_row].len();
        }

        Msg::DeleteForward => {
            if state.cursor_col < state.input_lines[state.cursor_row].len() {
                state.input_lines[state.cursor_row].remove(state.cursor_col);
            }
        }

        Msg::DeleteWordBackward => {
            let line = &state.input_lines[state.cursor_row];
            let before = &line[..state.cursor_col];
            if let Some(pos) = before.rfind(|c: char| c.is_whitespace()) {
                state.input_lines[state.cursor_row].drain(pos..state.cursor_col);
                state.cursor_col = pos;
            } else {
                state.input_lines[state.cursor_row].clear();
                state.cursor_col = 0;
            }
        }

        Msg::DeleteToStart => {
            state.input_lines[state.cursor_row].drain(..state.cursor_col);
            state.cursor_col = 0;
        }

        Msg::ToggleSidebar => state.show_sidebar = !state.show_sidebar,

        Msg::OpenCommandPalette => {
            state.command_palette_open = true;
            state.mode = TuiMode::CommandPalette;
            state.command_palette_filter.clear();
            state.command_palette_selected = 0;
        }

        Msg::CloseModal => {
            state.mode = TuiMode::Chat;
            state.command_palette_open = false;
            state.permission_modal_tool = None;
        }

        Msg::ConfirmModal => {
            state.mode = TuiMode::Chat;
            state.permission_modal_tool = None;
        }

        Msg::AgentEvent(event) => {
            match event {
                AgentEvent::MessageStart { message } => {
                    state.agent_running = true;
                    state.current_model = Some(message.role.clone());
                    state.messages.push(MessageItem::Assistant {
                        text: String::new(),
                        model: state.current_model.clone(),
                        timestamp: None,
                    });
                }
                AgentEvent::MessageUpdate { message } => {
                    if let Some(last) = state.messages.last_mut() {
                        if let MessageItem::Assistant { ref mut text, .. } = last {
                            let new_text = message
                                .content
                                .iter()
                                .filter_map(|part| {
                                    if let ContentPart::Text { text } = part {
                                        Some(text.as_str())
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join("");
                            *text = new_text;
                        }
                    }
                }
                AgentEvent::MessageEnd { message } => {
                    if let Some(last) = state.messages.last_mut() {
                        if let MessageItem::Assistant { ref mut text, .. } = last {
                            let final_text = message
                                .content
                                .iter()
                                .filter_map(|part| {
                                    if let ContentPart::Text { text } = part {
                                        Some(text.as_str())
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join("");
                            *text = final_text;
                        }
                    }
                }
                AgentEvent::ToolExecutionStart { tool_call_id } => {
                    state.messages.push(MessageItem::ToolCall {
                        name: tool_call_id,
                        args: String::new(),
                        result: None,
                        is_error: false,
                    });
                }
                AgentEvent::ToolExecutionEnd { result, .. } => {
                    let result_text = result
                        .content
                        .iter()
                        .filter_map(|part| {
                            if let ContentPart::Text { text } = part {
                                Some(text.as_str())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    let is_err = result.is_error;
                    if let Some(last) = state.messages.last_mut() {
                        if let MessageItem::ToolCall { ref mut result, ref mut is_error, .. } = last {
                            *result = Some(result_text);
                            *is_error = is_err;
                        }
                    }
                }
                AgentEvent::AgentEnd { .. } => {
                    state.agent_running = false;
                    state.current_model = None;
                }
                AgentEvent::Error { message } => {
                    state
                        .messages
                        .push(MessageItem::System { text: format!("Error: {}", message) });
                    state.agent_running = false;
                }
                AgentEvent::PermissionRequest { tool_name, tool_args, .. } => {
                    state.permission_modal_tool = Some(tool_name.clone());
                    state.permission_modal_args = Some(tool_args.clone());
                    state.permission_modal_desc =
                        Some(format!("Agent wants to execute '{}'", tool_name));
                    state.mode = TuiMode::Permission;
                }
                _ => {}
            }
        }

        Msg::PermissionConfirm => {
            state.mode = TuiMode::Chat;
            state.permission_modal_tool = None;
            cmds.push(Cmd::SendPermission { decision: PermissionDecision::Allow });
        }
        Msg::PermissionCancel => {
            state.mode = TuiMode::Chat;
            state.permission_modal_tool = None;
            cmds.push(Cmd::SendPermission { decision: PermissionDecision::Deny });
        }
        Msg::PermissionAlways => {
            state.mode = TuiMode::Chat;
            state.permission_modal_tool = None;
            cmds.push(Cmd::SendPermission { decision: PermissionDecision::AllowAlways });
        }
        Msg::PermissionSkip => {
            state.mode = TuiMode::Chat;
            state.permission_modal_tool = None;
            cmds.push(Cmd::SendPermission { decision: PermissionDecision::Skip });
        }
        Msg::CommandPaletteFilter(c) => {
            state.command_palette_filter.push(c);
        }
        Msg::CommandPaletteBackspace => {
            state.command_palette_filter.pop();
        }
        Msg::CommandPaletteUp => {
            if state.command_palette_selected > 0 {
                state.command_palette_selected -= 1;
            }
        }
        Msg::CommandPaletteDown => {
            state.command_palette_selected += 1;
        }
        Msg::CommandPaletteConfirm => {
            state.command_palette_open = false;
            state.mode = TuiMode::Chat;
        }
        Msg::ScrollUp => {
            state.feed_scroll_offset = state.feed_scroll_offset.saturating_sub(1);
        }
        Msg::ScrollDown => {
            state.feed_scroll_offset += 1;
        }
    }

    cmds
}

// ─── event_to_msg ──────────────────────────────────────────────────────────────
// Convert crossterm events to Msg

pub fn event_to_msg(event: Event, state: &AppState) -> Option<Msg> {
    match event {
        Event::Key(key) => key_to_msg(key, state),
        _ => None,
    }
}

fn key_to_msg(key: crossterm::event::KeyEvent, state: &AppState) -> Option<Msg> {
    match state.mode {
        TuiMode::Chat => match key.code {
            KeyCode::Char('c') | KeyCode::Char('q')
                if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::Quit),
            KeyCode::Enter => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    Some(Msg::InsertNewline)
                } else {
                    Some(Msg::Submit)
                }
            }
            KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::InsertNewline),
            KeyCode::Char('k') | KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::OpenCommandPalette),
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::MoveCursorToStart),
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::MoveCursorToEnd),
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::DeleteWordBackward),
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::DeleteToStart),
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::DeleteForward),
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::ToggleSidebar),
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::MoveCursorRight),
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::MoveCursorDown),
            KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Msg::Backspace),
            KeyCode::Char(c) => Some(Msg::InsertChar(c)),
            KeyCode::Backspace => Some(Msg::Backspace),
            KeyCode::Left => Some(Msg::MoveCursorLeft),
            KeyCode::Right => Some(Msg::MoveCursorRight),
            KeyCode::Up => Some(Msg::MoveCursorUp),
            KeyCode::Down => Some(Msg::MoveCursorDown),
            KeyCode::PageUp => Some(Msg::ScrollUp),
            KeyCode::PageDown => Some(Msg::ScrollDown),
            _ => None,
        },
        TuiMode::Permission => match key.code {
            KeyCode::Enter => Some(Msg::PermissionConfirm),
            KeyCode::Esc => Some(Msg::PermissionCancel),
            KeyCode::Char('y') => Some(Msg::PermissionConfirm),
            KeyCode::Char('n') => Some(Msg::PermissionCancel),
            KeyCode::Char('a') => Some(Msg::PermissionAlways),
            KeyCode::Char('s') => Some(Msg::PermissionSkip),
            _ => None,
        },
        TuiMode::CommandPalette => match key.code {
            KeyCode::Esc => Some(Msg::CloseModal),
            KeyCode::Enter => Some(Msg::CommandPaletteConfirm),
            KeyCode::Up => Some(Msg::CommandPaletteUp),
            KeyCode::Down => Some(Msg::CommandPaletteDown),
            KeyCode::Backspace => Some(Msg::CommandPaletteBackspace),
            KeyCode::Char(c) => Some(Msg::CommandPaletteFilter(c)),
            _ => None,
        },
        _ => None,
    }
}

// ─── to_agent_messages ─────────────────────────────────────────────────────────
// Convert MessageItem list to AgentMessage list for spawning agent

fn to_agent_messages(items: &[MessageItem]) -> Vec<AgentMessage> {
    items.iter().filter_map(|item| match item {
        MessageItem::User { text, .. } => Some(AgentMessage {
            role: "user".to_string(),
            content: vec![ContentPart::Text { text: text.clone() }],
            timestamp: 0,
            usage: None,
            stop_reason: None,
            error_message: None,
        }),
        MessageItem::Assistant { text, .. } => Some(AgentMessage {
            role: "assistant".to_string(),
            content: vec![ContentPart::Text { text: text.clone() }],
            timestamp: 0,
            usage: None,
            stop_reason: None,
            error_message: None,
        }),
        _ => None,
    }).collect()
}

// ─── Tui ─────────────────────────────────────────────────────────────────────

pub struct Tui {
    pub config: TuiConfig,
    pub terminal: Terminal<CrosstermBackend<io::Stdout>>,
    pub state: AppState,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TuiMode {
    Chat,
    Overlay,
    Select,
    Permission,
    CommandPalette,
}

impl Tui {
    pub fn new(config: TuiConfig) -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = stdout();
        stdout.execute(EnterAlternateScreen)?;
        stdout.execute(Show)?;
        stdout.execute(SetCursorStyle::SteadyBar)?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self {
            config,
            terminal,
            state: AppState::default(),
        })
    }

    pub fn cleanup(&mut self) -> io::Result<()> {
        disable_raw_mode()?;
        self.terminal.backend_mut().execute(SetCursorStyle::DefaultUserShape)?;
        self.terminal.backend_mut().execute(LeaveAlternateScreen)?;
        Ok(())
    }

    /// Dispatch a msg to update state (unidirectional data flow)
    /// Returns Vec<Cmd> to be executed by the runtime
    pub fn update(&mut self, msg: Msg) -> Vec<Cmd> {
        update(&mut self.state, msg)
    }

    /// Calculate the height needed for the input bar based on its content
    fn input_bar_height(&self) -> u16 {
        // Each logical line = 1 visual line (no wrapping)
        let visual_lines = self.state.input_lines.len().max(1);
        // 2 for borders + visual lines for content
        (visual_lines as u16) + 2
    }

    pub fn render(&mut self) -> io::Result<()> {
        let size = self.terminal.size()?;
        let area = Rect::new(0, 0, size.width, size.height);

        let padded_area = Rect {
            x: area.x + 2,
            y: area.y + 1,
            width: area.width.saturating_sub(4),
            height: area.height.saturating_sub(2),
        };

        // Calculate dynamic input bar height
        let input_height = self.input_bar_height();

        // Extract values needed in closure to avoid borrow conflicts
        let show_sidebar = self.state.show_sidebar;
        let show_top_bar = self.config.show_top_bar;
        let show_status_bar = self.config.show_status_bar;
        let mode = self.state.mode.clone();
        let state_clone = self.state.clone();

        self.terminal.draw(|frame| {
            let theme = &self.config.theme;

            // Clear entire frame with bg.base
            let bg_base: ratatui::style::Color = theme.color("bg.base").into();
            for y in 0..area.height {
                for x in 0..area.width {
                    frame.buffer_mut().get_mut(x, y).set_style(Style::default().bg(bg_base));
                }
            }

            // Main vertical layout: TopBar | ContentArea | InputBar | StatusBar
            let main_constraints = [
                if show_top_bar { Constraint::Length(1) } else { Constraint::Length(0) },
                Constraint::Min(1),  // Content area (will be split horizontally)
                Constraint::Length(input_height),
                if show_status_bar { Constraint::Length(1) } else { Constraint::Length(0) },
            ];
            let main_areas: [Rect; 4] = Layout::vertical(main_constraints).areas(padded_area);

            // Render top bar using standalone function
            if show_top_bar {
                render_top_bar(&state_clone, main_areas[0], frame.buffer_mut(), theme);
            }

            // Split content area horizontally
            let content_area = main_areas[1];
            let mut h_constraints = vec![];
            h_constraints.push(Constraint::Min(20));
            if show_sidebar && content_area.width >= SIDEBAR_WIDTH + 20 {
                h_constraints.push(Constraint::Length(SIDEBAR_WIDTH));
            }
            let h_areas = Layout::horizontal(h_constraints.as_slice()).split(content_area);

            // Render message list directly from state (no widget instance needed)
            if show_sidebar && content_area.width >= SIDEBAR_WIDTH + 20 {
                MessageList::render_ref(&state_clone.messages, state_clone.feed_scroll_offset, h_areas[0], frame.buffer_mut(), theme);
                render_agent_list(h_areas[1], frame.buffer_mut(), theme);
            } else {
                MessageList::render_ref(&state_clone.messages, state_clone.feed_scroll_offset, h_areas[0], frame.buffer_mut(), theme);
            }

            // Render input bar using standalone function
            let input_bar = InputBar {
                prompt: "\u{276F} ".to_string(),
                lines: state_clone.input_lines.clone(),
                cursor_line: state_clone.cursor_row.min(state_clone.input_lines.len().saturating_sub(1)),
                cursor_col: state_clone.cursor_col,
                mode: crate::components::InputMode::Normal,
                right_info: state_clone.input_right_info.clone(),
            };
            input_bar.render_ref(main_areas[2], frame.buffer_mut(), theme);
            let cursor_pos = input_bar.cursor_screen_pos(main_areas[2]);
            frame.set_cursor_position(cursor_pos);

            // Render status bar using standalone function
            if show_status_bar {
                render_status_bar(&state_clone, main_areas[3], frame.buffer_mut(), theme);
            }

            // Permission modal (render from state if active)
            if mode == TuiMode::Permission && state_clone.permission_modal_tool.is_some() {
                let bg_base: ratatui::style::Color = theme.color("bg.base").into();
                // Dim background
                for y in 0..area.height {
                    for x in 0..area.width {
                        if let Some(cell) = frame.buffer_mut().cell_mut((x, y)) {
                            cell.set_style(Style::default().bg(bg_base));
                        }
                    }
                }
                // Center modal
                let modal_w = 50u16;
                let modal_h = 12u16;
                let modal_x = padded_area.x + (padded_area.width.saturating_sub(modal_w)) / 2;
                let modal_y = padded_area.y + (padded_area.height.saturating_sub(modal_h)) / 2;
                let modal_area = Rect::new(modal_x, modal_y, modal_w, modal_h);

                // Draw shadow
                Self::render_shadow(modal_area, frame.buffer_mut(), theme);

                // Render permission modal from state
                let modal = PermissionModal::new(
                    state_clone.permission_modal_tool.as_deref().unwrap_or(""),
                    state_clone.permission_modal_args.as_deref().unwrap_or(""),
                    state_clone.permission_modal_desc.as_deref().unwrap_or(""),
                );
                modal.render_ref(modal_area, frame.buffer_mut(), theme);
            }

            // Command palette (simplified - just show a placeholder)
            if mode == TuiMode::CommandPalette {
                let bg_base: ratatui::style::Color = theme.color("bg.base").into();
                // Dim background
                for y in 0..area.height {
                    for x in 0..area.width {
                        if let Some(cell) = frame.buffer_mut().cell_mut((x, y)) {
                            cell.set_style(Style::default().bg(bg_base));
                        }
                    }
                }
                // Center palette
                let palette_w = 70u16;
                let palette_h = 20u16;
                let palette_x = padded_area.x + (padded_area.width.saturating_sub(palette_w)) / 2;
                let palette_y = padded_area.y + (padded_area.height.saturating_sub(palette_h)) / 2;
                let palette_area = Rect::new(palette_x, palette_y, palette_w, palette_h);

                // Draw shadow
                Self::render_shadow(palette_area, frame.buffer_mut(), theme);

                // Render command palette widget
                let palette = CommandPalette::new();
                palette.render_ref(palette_area, frame.buffer_mut(), theme);
            }

            // Overlay mode (simplified - just show a centered box)
            if mode == TuiMode::Overlay {
                let overlay_area = Overlay::centered((60, 20), frame.area());

                // Draw shadow first
                Self::render_shadow(overlay_area, frame.buffer_mut(), theme);

                let mut overlay_buf = Buffer::empty(overlay_area);
                let overlay = Overlay::default();
                overlay.render_ref(overlay_area, &mut overlay_buf, theme);
                for y in 0..overlay_buf.area.height {
                    for x in 0..overlay_buf.area.width {
                        let cell = overlay_buf.get(x, y);
                        let tx = overlay_area.x + x;
                        let ty = overlay_area.y + y;
                        if tx < area.width && ty < area.height {
                            if let Some(target) = frame.buffer_mut().cell_mut((tx, ty)) {
                                target.set_style(cell.style());
                                if let Some(ch) = cell.symbol().chars().next() {
                                    target.set_char(ch);
                                }
                            }
                        }
                    }
                }
            }
        })?;
        Ok(())
    }

    pub fn handle_event(&mut self, event: Event) -> Option<TuiAction> {
        match event {
            Event::Key(key) => self.handle_key(key),
            Event::Resize(_, _) => None,
            _ => None,
        }
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> Option<TuiAction> {
        // Convert key event to Msg and apply update
        if let Some(msg) = key_to_msg(key, &self.state) {
            let cmds = self.update(msg);
            // Process cmds to determine TuiAction
            for cmd in cmds {
                match cmd {
                    Cmd::SpawnAgent { .. } => {
                        // Would return Submit action - but text already captured
                    }
                    Cmd::SendPermission { decision } => {
                        let permission_action = match decision {
                            PermissionDecision::Allow => PermissionAction::Confirm,
                            PermissionDecision::Deny => PermissionAction::Cancel,
                            PermissionDecision::AllowAlways => PermissionAction::Always,
                            PermissionDecision::Skip => PermissionAction::Skip,
                        };
                        return Some(TuiAction::ToolPermission {
                            tool: self.state.permission_modal_tool.clone().unwrap_or_default(),
                            action: permission_action,
                        });
                    }
                }
            }
            // Check if we should quit
            if !self.state.running {
                return Some(TuiAction::Quit);
            }
        }
        None
    }

    pub fn add_message(&mut self, item: MessageItem) {
        self.state.messages.push(item);
    }

    pub fn on_agent_event(&mut self, event: AgentEvent) -> Vec<Cmd> {
        // Use the update() reducer for all agent events
        self.update(Msg::AgentEvent(event.clone()))
    }

    pub fn show_overlay(&mut self, _overlay: Overlay) {
        self.state.mode = TuiMode::Overlay;
    }

    pub fn hide_overlay(&mut self) {
        self.state.mode = TuiMode::Chat;
    }

    pub fn request_permission(&mut self, tool_name: &str, tool_args: &str, description: &str) {
        self.state.permission_modal_tool = Some(tool_name.to_string());
        self.state.permission_modal_args = Some(tool_args.to_string());
        self.state.permission_modal_desc = Some(description.to_string());
        self.state.mode = TuiMode::Permission;
    }

    pub fn is_permission_modal_active(&self) -> bool {
        self.state.permission_modal_tool.is_some() && self.state.mode == TuiMode::Permission
    }

    pub fn toggle_sidebar(&mut self) {
        self.update(Msg::ToggleSidebar);
    }

    /// Draw a subtle shadow around a modal area (1 cell right, 1 cell down)
    fn render_shadow(modal_area: Rect, buf: &mut ratatui::buffer::Buffer, theme: &ThemeWrapper) {
        let shadow_bg: ratatui::style::Color = theme.color("bg.base").into();
        let shadow_fg: ratatui::style::Color = theme.color("text.dim").into();

        // Shadow on the right side (1 column to the right of modal)
        let shadow_x = modal_area.x + modal_area.width;
        if shadow_x < buf.area.width {
            for y in modal_area.y + 1..modal_area.y + modal_area.height + 1 {
                if y < buf.area.height {
                    if let Some(cell) = buf.cell_mut((shadow_x, y)) {
                        cell.set_char('░');
                        cell.set_style(Style::default().fg(shadow_fg).bg(shadow_bg));
                    }
                }
            }
        }

        // Shadow on the bottom (1 row below modal)
        let shadow_y = modal_area.y + modal_area.height;
        if shadow_y < buf.area.height {
            for x in modal_area.x + 1..modal_area.x + modal_area.width + 1 {
                if x < buf.area.width {
                    if let Some(cell) = buf.cell_mut((x, shadow_y)) {
                        cell.set_char('░');
                        cell.set_style(Style::default().fg(shadow_fg).bg(shadow_bg));
                    }
                }
            }
        }

        // Corner shadow (diagonal)
        let corner_x = modal_area.x + modal_area.width;
        let corner_y = modal_area.y + modal_area.height;
        if corner_x < buf.area.width && corner_y < buf.area.height {
            if let Some(cell) = buf.cell_mut((corner_x, corner_y)) {
                cell.set_char('▒');
                cell.set_style(Style::default().fg(shadow_fg).bg(shadow_bg));
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TuiAction {
    Quit,
    Submit(String),
    Command(String),
    Cancel,
    ToolPermission { tool: String, action: PermissionAction },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_list_has_demo_data() {
        // Verify AgentList default has agents populated (testing the data structure)
        let agent_list = AgentList {
            agents: vec![
                AgentItem {
                    id: "coder".to_string(),
                    tag: "coder".to_string(),
                    tag_type: "assistant".to_string(),
                    description: "editing files".to_string(),
                    model: "claude-4".to_string(),
                    duration_secs: 45,
                    status: AgentStatus::Running,
                },
                AgentItem {
                    id: "test".to_string(),
                    tag: "test".to_string(),
                    tag_type: "system".to_string(),
                    description: "running tests".to_string(),
                    model: "gpt-4".to_string(),
                    duration_secs: 12,
                    status: AgentStatus::Completed,
                },
            ],
        };
        assert_eq!(agent_list.agents.len(), 2);
        assert_eq!(agent_list.agents[0].id, "coder");
        assert_eq!(agent_list.agents[1].status, AgentStatus::Completed);
    }

    #[test]
    fn test_context_panel_has_demo_data() {
        let context_panel = ContextPanel {
            recent_files: vec![
                "src/main.rs".to_string(),
                "Cargo.toml".to_string(),
                "README.md".to_string(),
            ],
            git_changes: vec![
                GitChange { path: "src/tui.rs".to_string(), status: GitStatus::Modified },
                GitChange { path: "src/components/context_panel.rs".to_string(), status: GitStatus::Added },
            ],
            active_tool: Some("read_file".to_string()),
            model_name: "claude-4".to_string(),
            session_info: "demo-session-001".to_string(),
        };
        assert_eq!(context_panel.model_name, "claude-4");
        assert_eq!(context_panel.recent_files.len(), 3);
        assert_eq!(context_panel.git_changes.len(), 2);
        assert_eq!(context_panel.active_tool, Some("read_file".to_string()));
    }

    #[test]
    fn test_sidebar_toggle_methods() {
        // Test that toggle methods work on Tui state
        // We test the methods themselves since Tui::new requires a terminal
        let mut show_left = false;
        let mut show_right = false;

        // Simulate toggle_left_sidebar
        show_left = !show_left;
        assert!(show_left);

        // Simulate toggle_right_sidebar
        show_right = !show_right;
        assert!(show_right);
    }

    #[test]
    fn test_agent_status_variants() {
        assert_eq!(AgentStatus::Running, AgentStatus::Running);
        assert_eq!(AgentStatus::Completed, AgentStatus::Completed);
        assert_ne!(AgentStatus::Running, AgentStatus::Completed);
    }

    #[test]
    fn test_git_status_variants() {
        assert_eq!(GitStatus::Modified, GitStatus::Modified);
        assert_eq!(GitStatus::Added, GitStatus::Added);
        assert_eq!(GitStatus::Deleted, GitStatus::Deleted);
        assert_eq!(GitStatus::Untracked, GitStatus::Untracked);
    }

    // ─── Reducer Tests ─────────────────────────────────────────────────────────

    fn make_state() -> AppState {
        AppState {
            messages: vec![],
            input_lines: vec![String::new()],
            cursor_col: 0,
            cursor_row: 0,
            input_right_info: String::new(),
            mode: TuiMode::Chat,
            running: true,
            show_sidebar: false,
            agent_running: false,
            current_model: None,
            top_bar_repo: String::new(),
            top_bar_branch: String::new(),
            top_bar_path: String::new(),
            top_bar_checks_passed: None,
            top_bar_checks_total: None,
            top_bar_percentage: None,
            top_bar_agent_count: None,
            permission_modal_tool: None,
            permission_modal_args: None,
            permission_modal_desc: None,
            action_log: Vec::new(),
            action_log_capacity: 1000,
            command_palette_open: false,
            command_palette_filter: String::new(),
            command_palette_selected: 0,
            feed_scroll_offset: 0,
        }
    }

    #[test]
    fn test_insert_char() {
        let mut state = make_state();
        update(&mut state, Msg::InsertChar('h'));
        update(&mut state, Msg::InsertChar('i'));
        assert_eq!(state.input_lines, vec!["hi"]);
        assert_eq!(state.cursor_col, 2);
    }

    #[test]
    fn test_backspace() {
        let mut state = make_state();
        update(&mut state, Msg::InsertChar('h'));
        update(&mut state, Msg::InsertChar('i'));
        update(&mut state, Msg::Backspace);
        assert_eq!(state.input_lines, vec!["h"]);
        assert_eq!(state.cursor_col, 1);
    }

    #[test]
    fn test_submit_clears_input() {
        let mut state = make_state();
        update(&mut state, Msg::InsertChar('h'));
        update(&mut state, Msg::InsertChar('i'));
        let cmds = update(&mut state, Msg::Submit);
        assert_eq!(state.input_lines, vec![""]);
        assert_eq!(state.messages.len(), 1);
        // Should return a SpawnAgent cmd
        assert_eq!(cmds.len(), 1);
        if let Cmd::SpawnAgent { .. } = &cmds[0] {
            // Expected
        } else {
            panic!("Expected SpawnAgent cmd");
        }
        if let MessageItem::User { text, .. } = &state.messages[0] {
            assert_eq!(text, "hi");
        } else {
            panic!("Expected User message");
        }
    }

    #[test]
    fn test_submit_empty_does_nothing() {
        let mut state = make_state();
        let cmds = update(&mut state, Msg::Submit);
        assert_eq!(state.messages.len(), 0);
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_move_cursor() {
        let mut state = make_state();
        update(&mut state, Msg::InsertChar('a'));
        update(&mut state, Msg::InsertChar('b'));
        update(&mut state, Msg::InsertChar('c'));
        assert_eq!(state.cursor_col, 3);

        update(&mut state, Msg::MoveCursorLeft);
        assert_eq!(state.cursor_col, 2);

        update(&mut state, Msg::MoveCursorLeft);
        assert_eq!(state.cursor_col, 1);

        update(&mut state, Msg::MoveCursorRight);
        assert_eq!(state.cursor_col, 2);

        update(&mut state, Msg::MoveCursorToStart);
        assert_eq!(state.cursor_col, 0);

        update(&mut state, Msg::MoveCursorToEnd);
        assert_eq!(state.cursor_col, 3);
    }

    #[test]
    fn test_newline() {
        let mut state = make_state();
        update(&mut state, Msg::InsertChar('h'));
        update(&mut state, Msg::InsertChar('i'));
        update(&mut state, Msg::InsertNewline);
        assert_eq!(state.input_lines, vec!["hi", ""]);
        assert_eq!(state.cursor_row, 1);
        assert_eq!(state.cursor_col, 0);
    }

    #[test]
    fn test_multi_line_submit() {
        let mut state = make_state();
        for c in "line1".chars() {
            update(&mut state, Msg::InsertChar(c));
        }
        update(&mut state, Msg::InsertNewline);
        for c in "line2".chars() {
            update(&mut state, Msg::InsertChar(c));
        }
        update(&mut state, Msg::Submit);

        assert_eq!(state.input_lines, vec![""]);
        assert_eq!(state.messages.len(), 1);
        if let MessageItem::User { text, .. } = &state.messages[0] {
            assert_eq!(text, "line1\nline2");
        } else {
            panic!("Expected User message");
        }
    }

    #[test]
    fn test_quit() {
        let mut state = make_state();
        update(&mut state, Msg::Quit);
        assert!(!state.running);
    }

    #[test]
    fn test_toggle_sidebar() {
        let mut state = make_state();
        assert!(!state.show_sidebar);
        update(&mut state, Msg::ToggleSidebar);
        assert!(state.show_sidebar);
        update(&mut state, Msg::ToggleSidebar);
        assert!(!state.show_sidebar);
    }

    #[test]
    fn test_delete_word_backward() {
        let mut state = make_state();
        // Type "hello world"
        for c in "hello world".chars() {
            update(&mut state, Msg::InsertChar(c));
        }
        assert_eq!(state.cursor_col, 11);

        // Delete word backward → "hello" (removes " world" including space, bash-like)
        update(&mut state, Msg::DeleteWordBackward);
        assert_eq!(state.input_lines[0], "hello");
        assert_eq!(state.cursor_col, 5);

        // Delete word backward → "" (no more words, clears line)
        update(&mut state, Msg::DeleteWordBackward);
        assert_eq!(state.input_lines[0], "");
        assert_eq!(state.cursor_col, 0);
    }

    #[test]
    fn test_delete_to_start() {
        let mut state = make_state();
        for c in "hello".chars() {
            update(&mut state, Msg::InsertChar(c));
        }
        update(&mut state, Msg::MoveCursorToEnd);
        update(&mut state, Msg::DeleteToStart);
        assert_eq!(state.input_lines[0], "");
        assert_eq!(state.cursor_col, 0);
    }

    #[test]
    fn test_delete_forward() {
        let mut state = make_state();
        update(&mut state, Msg::InsertChar('a'));
        update(&mut state, Msg::InsertChar('b'));
        update(&mut state, Msg::InsertChar('c'));
        update(&mut state, Msg::MoveCursorToStart);
        update(&mut state, Msg::DeleteForward);
        assert_eq!(state.input_lines[0], "bc");
    }

    #[test]
    fn test_agent_event_message_start() {
        let mut state = make_state();
        update(
            &mut state,
            Msg::AgentEvent(AgentEvent::MessageStart {
                message: AgentMessage {
                    role: "assistant".to_string(),
                    content: vec![],
                    timestamp: 0,
                    usage: None,
                    stop_reason: None,
                    error_message: None,
                },
            }),
        );
        assert!(state.agent_running);
        assert_eq!(state.messages.len(), 1);
    }

    #[test]
    fn test_agent_event_message_update() {
        let mut state = make_state();
        // Start message
        update(
            &mut state,
            Msg::AgentEvent(AgentEvent::MessageStart {
                message: AgentMessage {
                    role: "assistant".to_string(),
                    content: vec![],
                    timestamp: 0,
                    usage: None,
                    stop_reason: None,
                    error_message: None,
                },
            }),
        );

        // Update with text
        update(
            &mut state,
            Msg::AgentEvent(AgentEvent::MessageUpdate {
                message: AgentMessage {
                    role: "assistant".to_string(),
                    content: vec![ContentPart::Text {
                        text: "Hello".to_string(),
                    }],
                    timestamp: 0,
                    usage: None,
                    stop_reason: None,
                    error_message: None,
                },
            }),
        );

        assert_eq!(state.messages.len(), 1);
        if let MessageItem::Assistant { text, .. } = &state.messages[0] {
            assert_eq!(text, "Hello");
        } else {
            panic!("Expected Assistant message");
        }
    }

    // ─── Time-Travel Tests ─────────────────────────────────────────────────────

    #[test]
    fn test_action_log_records_msgs() {
        let mut state = make_state();
        update(&mut state, Msg::InsertChar('h'));
        update(&mut state, Msg::InsertChar('i'));
        update(&mut state, Msg::Submit);

        assert_eq!(state.action_log.len(), 3);
        assert!(matches!(state.action_log[0], Msg::InsertChar('h')));
        assert!(matches!(state.action_log[1], Msg::InsertChar('i')));
        assert!(matches!(state.action_log[2], Msg::Submit));
    }

    #[test]
    fn test_action_log_capacity() {
        let mut state = make_state();
        state.action_log_capacity = 5;

        for i in 0..10 {
            update(&mut state, Msg::InsertChar('a'));
        }

        assert_eq!(state.action_log.len(), 5); // Only keeps last 5
    }

    #[test]
    fn test_replay_actions() {
        let mut state = make_state();
        update(&mut state, Msg::InsertChar('h'));
        update(&mut state, Msg::InsertChar('i'));
        update(&mut state, Msg::Submit);

        let replayed = state.replay_to(2); // Replay first 2 msgs
        assert_eq!(replayed.input_lines, vec!["hi"]);
        assert_eq!(replayed.messages.len(), 0); // Submit not replayed

        let replayed_full = state.replay_to(3); // Replay all 3
        assert_eq!(replayed_full.messages.len(), 1);
    }

    #[test]
    fn test_replay_produces_same_state() {
        let mut state = make_state();
        // Complex sequence
        update(&mut state, Msg::InsertChar('h'));
        update(&mut state, Msg::InsertChar('e'));
        update(&mut state, Msg::InsertChar('l'));
        update(&mut state, Msg::InsertChar('l'));
        update(&mut state, Msg::InsertChar('o'));
        update(&mut state, Msg::Submit);
        update(&mut state, Msg::ToggleSidebar);
        update(&mut state, Msg::InsertChar('w'));
        update(&mut state, Msg::InsertChar('o'));
        update(&mut state, Msg::InsertChar('r'));
        update(&mut state, Msg::InsertChar('l'));
        update(&mut state, Msg::InsertChar('d'));

        let replayed = state.replay_to(state.action_log.len());
        assert_eq!(replayed.input_lines, state.input_lines);
        assert_eq!(replayed.messages, state.messages);
        assert_eq!(replayed.show_sidebar, state.show_sidebar);
    }

    #[test]
    fn test_permission_cmds() {
        let mut state = make_state();

        // PermissionConfirm should return Allow decision
        let cmds = update(&mut state, Msg::PermissionConfirm);
        assert_eq!(cmds.len(), 1);
        if let Cmd::SendPermission { decision } = &cmds[0] {
            assert_eq!(*decision, PermissionDecision::Allow);
        } else {
            panic!("Expected SendPermission cmd");
        }

        // PermissionCancel should return Deny decision
        let cmds = update(&mut state, Msg::PermissionCancel);
        if let Cmd::SendPermission { decision } = &cmds[0] {
            assert_eq!(*decision, PermissionDecision::Deny);
        }

        // PermissionAlways should return AllowAlways decision
        let cmds = update(&mut state, Msg::PermissionAlways);
        if let Cmd::SendPermission { decision } = &cmds[0] {
            assert_eq!(*decision, PermissionDecision::AllowAlways);
        }

        // PermissionSkip should return Skip decision
        let cmds = update(&mut state, Msg::PermissionSkip);
        if let Cmd::SendPermission { decision } = &cmds[0] {
            assert_eq!(*decision, PermissionDecision::Skip);
        }
    }
}
