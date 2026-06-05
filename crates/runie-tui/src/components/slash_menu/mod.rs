//! Inline slash menu component — grok-style autocomplete above input box.
//!
//! Shows when input starts with `/`. Up/Down navigate, Enter select, Esc close.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::Widget,
};
use crate::style::box_chars;
use crate::style::selection;
use crate::theme::ThemeWrapper;

/// Definition of a slash command for display in the menu.
#[derive(Debug, Clone)]
pub struct SlashMenuItem {
    pub command: &'static str,
    pub aliases: &'static [&'static str],
    pub description: &'static str,
    pub category: &'static str,
}

/// All available slash commands.
pub static SLASH_COMMANDS: &[SlashMenuItem] = &[
    // Session
    SlashMenuItem { command: "/new", aliases: &["/n"], description: "Start new session", category: "Session" },
    SlashMenuItem { command: "/clear", aliases: &["/c"], description: "Clear conversation", category: "Session" },
    SlashMenuItem { command: "/tree", aliases: &["/t"], description: "Open session tree", category: "Session" },
    SlashMenuItem { command: "/fork", aliases: &["/f"], description: "Fork at current position", category: "Session" },
    SlashMenuItem { command: "/home", aliases: &[], description: "Return to welcome screen", category: "Session" },
    SlashMenuItem { command: "/resume", aliases: &[], description: "Resume previous session", category: "Session" },
    SlashMenuItem { command: "/sessions", aliases: &[], description: "Browse past sessions", category: "Session" },
    SlashMenuItem { command: "/rename", aliases: &[], description: "Rename current session", category: "Session" },
    SlashMenuItem { command: "/share", aliases: &[], description: "Share session", category: "Session" },
    SlashMenuItem { command: "/session-info", aliases: &[], description: "Show session info", category: "Session" },
    // Context
    SlashMenuItem { command: "/context", aliases: &[], description: "View context usage", category: "Context" },
    SlashMenuItem { command: "/compact", aliases: &[], description: "Compact conversation history", category: "Context" },
    SlashMenuItem { command: "/compact-mode", aliases: &[], description: "Toggle denser UI layout", category: "Context" },
    SlashMenuItem { command: "/rewind", aliases: &[], description: "Rewind conversation", category: "Context" },
    SlashMenuItem { command: "/usage", aliases: &[], description: "Show token/credit usage", category: "Context" },
    // Config
    SlashMenuItem { command: "/model", aliases: &["/m"], description: "Switch model", category: "Config" },
    SlashMenuItem { command: "/onboard", aliases: &["/o"], description: "Configure provider", category: "Config" },
    SlashMenuItem { command: "/theme", aliases: &[], description: "Switch theme", category: "Config" },
    SlashMenuItem { command: "/status", aliases: &[], description: "Show current provider and model", category: "Config" },
    SlashMenuItem { command: "/models", aliases: &[], description: "Show available models", category: "Config" },
    // Tools
    SlashMenuItem { command: "/copy", aliases: &[], description: "Copy last response", category: "Tools" },
    SlashMenuItem { command: "/cost", aliases: &[], description: "Show cost stats", category: "Tools" },
    SlashMenuItem { command: "/always-approve", aliases: &[], description: "Toggle auto-approve mode", category: "Tools" },
    SlashMenuItem { command: "/multiline", aliases: &[], description: "Toggle multiline input", category: "Tools" },
    // Permission
    SlashMenuItem { command: "/plan", aliases: &[], description: "View current session plan", category: "Permission" },
    SlashMenuItem { command: "/feedback", aliases: &[], description: "Send feedback", category: "Permission" },
    // Utility
    SlashMenuItem { command: "/btw", aliases: &[], description: "Ask side question", category: "Utility" },
    SlashMenuItem { command: "/logout", aliases: &[], description: "Sign out", category: "Utility" },
    // Extensions
    SlashMenuItem { command: "/hooks", aliases: &[], description: "Open extensions (Hooks)", category: "Extensions" },
    SlashMenuItem { command: "/plugins", aliases: &[], description: "Open extensions (Plugins)", category: "Extensions" },
    SlashMenuItem { command: "/skills", aliases: &[], description: "Open extensions (Skills)", category: "Extensions" },
    SlashMenuItem { command: "/mcps", aliases: &[], description: "Open extensions (MCP Servers)", category: "Extensions" },
    // Shell
    SlashMenuItem { command: "/flush", aliases: &[], description: "Flush memory to disk", category: "Shell" },
    SlashMenuItem { command: "/memory", aliases: &[], description: "Search memory", category: "Shell" },
    SlashMenuItem { command: "/dream", aliases: &[], description: "Memory consolidation", category: "Shell" },
    SlashMenuItem { command: "/imagine", aliases: &[], description: "Generate image", category: "Shell" },
    SlashMenuItem { command: "/imagine-video", aliases: &[], description: "Generate video", category: "Shell" },
    // App
    SlashMenuItem { command: "/quit", aliases: &["/q", "/exit"], description: "Exit runie", category: "App" },
    SlashMenuItem { command: "/help", aliases: &["/h", "/?"], description: "Show help", category: "App" },
];

/// Inline slash menu state.
#[derive(Debug, Clone, Default)]
pub struct SlashMenu {
    pub open: bool,
    pub filter: String,
    pub selected: usize,
    pub filtered_indices: Vec<usize>,
}

impl SlashMenu {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open menu with current input filter (minus the leading `/`).
    pub fn open(&mut self, input: &str) {
        self.open = true;
        self.filter = input.strip_prefix('/').unwrap_or(input).to_string();
        self.selected = 0;
        self.update_filtered();
    }

    pub fn close(&mut self) {
        self.open = false;
        self.filter.clear();
        self.selected = 0;
        self.filtered_indices.clear();
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Update filter from input text (caller strips `/`).
    pub fn set_filter(&mut self, filter: &str) {
        self.filter = filter.to_string();
        self.update_filtered();
    }

    fn update_filtered(&mut self) {
        let query = self.filter.to_lowercase();
        self.filtered_indices = SLASH_COMMANDS
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                item.command.to_lowercase().contains(&query)
                    || item.description.to_lowercase().contains(&query)
                    || item.aliases.iter().any(|a| a.to_lowercase().contains(&query))
            })
            .map(|(i, _)| i)
            .collect();
        if self.selected >= self.filtered_indices.len() {
            self.selected = 0;
        }
    }

    pub fn move_up(&mut self) {
        if self.filtered_indices.len() > 1 {
            self.selected = self.selected.saturating_sub(1);
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.filtered_indices.len() {
            self.selected += 1;
        }
    }

    /// Return the selected command string (e.g. "/new"), or None.
    pub fn selected_command(&self) -> Option<String> {
        self.filtered_indices.get(self.selected).map(|&idx| SLASH_COMMANDS[idx].command.to_string())
    }

    pub fn selected_item(&self) -> Option<&SlashMenuItem> {
        self.filtered_indices.get(self.selected).map(|&idx| &SLASH_COMMANDS[idx])
    }
}

fn render_slash_border(area: Rect, buf: &mut Buffer, border: Color, bg: Color, hidden_count: usize) {
    clear_background(area, buf, bg);
    render_top_border(area, buf, border, hidden_count);
    render_bottom_border(area, buf, border);
}

fn clear_background(area: Rect, buf: &mut Buffer, bg: Color) {
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_char(' ');
                cell.set_bg(bg);
            }
        }
    }
}

fn render_top_border(area: Rect, buf: &mut Buffer, border: Color, hidden_count: usize) {
    let right_col = area.right().saturating_sub(1);
    if hidden_count > 0 && area.width >= 6 {
        render_top_border_with_count(area, buf, border, hidden_count, right_col);
    } else {
        for x in area.x..right_col {
            if let Some(cell) = buf.cell_mut((x, area.y)) {
                cell.set_char(box_chars::H);
                cell.set_fg(border);
            }
        }
    }
}

fn render_top_border_with_count(area: Rect, buf: &mut Buffer, border: Color, hidden_count: usize, right_col: u16) {
    let count_str = hidden_count.to_string();
    let left_end = right_col.saturating_sub(3);
    for x in area.x..left_end {
        if let Some(cell) = buf.cell_mut((x, area.y)) {
            cell.set_char(box_chars::H);
            cell.set_fg(border);
        }
    }
    for (i, ch) in count_str.chars().enumerate() {
        let x = left_end.saturating_add(i as u16);
        if x <= right_col {
            if let Some(cell) = buf.cell_mut((x, area.y)) {
                cell.set_char(ch);
                cell.set_fg(border);
            }
        }
    }
    if let Some(cell) = buf.cell_mut((right_col, area.y)) {
        cell.set_char(box_chars::H);
        cell.set_fg(border);
    }
}

fn render_bottom_border(area: Rect, buf: &mut Buffer, border: Color) {
    for x in area.x..area.right() {
        if let Some(cell) = buf.cell_mut((x, area.bottom() - 1)) {
            cell.set_char(box_chars::H);
            cell.set_fg(border);
        }
    }
}

fn render_slash_item(cmd: &SlashMenuItem, is_selected: bool, inner_x: u16, inner_w: u16, y: u16, buf: &mut Buffer, accent: Color, text_primary: Color, text_muted: Color) {
    let indicator = if is_selected { selection::SELECTED.to_string() } else { selection::UNSELECTED.to_string() };
    let indicator_style = if is_selected { Style::default().fg(accent).add_modifier(Modifier::BOLD) } else { Style::default().fg(text_muted) };
    buf.set_string(inner_x, y, &indicator, indicator_style);
    let label_x = inner_x + 2;
    let label_style = if is_selected { Style::default().fg(text_primary).add_modifier(Modifier::BOLD) } else { Style::default().fg(text_primary) };
    buf.set_string(label_x, y, cmd.command, label_style);
    // Right-align description
    let desc_len = cmd.description.len() as u16;
    let desc_x = inner_x + inner_w.saturating_sub(desc_len + 1);
    buf.set_string(desc_x, y, cmd.description, Style::default().fg(text_muted));
}

impl Widget for &SlashMenu {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border = Color::DarkGray;
        let text_primary = Color::White;
        let text_muted = Color::DarkGray;
        let accent = Color::Cyan;
        let bg = Color::Black;
        let visible_count = (area.height as usize).saturating_sub(2);
        let hidden_count = self.filtered_indices.len().saturating_sub(visible_count);
        render_slash_border(area, buf, border, bg, hidden_count);
        let inner_x = area.x + 2;
        let inner_w = area.width.saturating_sub(4);
        let mut y = area.y + 1;
        let max_y = area.bottom() - 1;
        for (display_idx, &cmd_idx) in self.filtered_indices.iter().enumerate() {
            if y >= max_y { break; }
            let cmd = &SLASH_COMMANDS[cmd_idx];
            render_slash_item(cmd, display_idx == self.selected, inner_x, inner_w, y, buf, accent, text_primary, text_muted);
            y += 1;
        }
    }
}

pub fn render_slash_menu(menu: &SlashMenu, area: Rect, buf: &mut Buffer, _theme: &ThemeWrapper) {
    menu.render(area, buf);
}
