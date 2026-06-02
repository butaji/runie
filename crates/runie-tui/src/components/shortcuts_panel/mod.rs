//! Keyboard shortcuts panel — grok-style `Ctrl+.` help overlay.
//!
//! Sections: Essentials, Input, Navigation, Actions, Panels, Session.
//! Features: filter mode (`f`), search (`/`), expand/collapse (`e`/`Enter`/`Space`).

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::Widget,
};
use crate::theme::ThemeWrapper;

/// A single keyboard shortcut entry.
#[derive(Debug, Clone)]
pub struct ShortcutDef {
    pub action: &'static str,
    pub keys: &'static str,
    pub section: &'static str,
}

/// All keyboard shortcuts organized by section.
pub static SHORTCUTS: &[ShortcutDef] = &[
    // Essentials
    ShortcutDef { action: "Send", keys: "Enter", section: "Essentials" },
    ShortcutDef { action: "Focus prompt", keys: "Tab / Space", section: "Essentials" },
    ShortcutDef { action: "Focus scrollback", keys: "Esc / Tab", section: "Essentials" },
    ShortcutDef { action: "Cancel turn", keys: "Ctrl+C", section: "Essentials" },
    ShortcutDef { action: "Cycle mode", keys: "Shift+Tab", section: "Essentials" },
    ShortcutDef { action: "Quit", keys: "Ctrl+Q / Ctrl+D", section: "Essentials" },
    ShortcutDef { action: "Command palette", keys: "Ctrl+K / Ctrl+P / ?", section: "Essentials" },
    ShortcutDef { action: "Keyboard shortcuts", keys: "Ctrl+.", section: "Essentials" },
    ShortcutDef { action: "Settings", keys: "F2 / Ctrl+, / ,", section: "Essentials" },
    // Input
    ShortcutDef { action: "New line", keys: "Shift+Enter", section: "Input" },
    ShortcutDef { action: "Interject", keys: "Ctrl+Enter", section: "Input" },
    ShortcutDef { action: "Search history", keys: "Ctrl+R", section: "Input" },
    ShortcutDef { action: "Multiline", keys: "Ctrl+M", section: "Input" },
    ShortcutDef { action: "Shell mode", keys: "!", section: "Input" },
    // Navigation
    ShortcutDef { action: "Scroll up", keys: "↑ / Ctrl+K", section: "Navigation" },
    ShortcutDef { action: "Scroll down", keys: "↓ / Ctrl+J", section: "Navigation" },
    ShortcutDef { action: "Page up", keys: "PageUp / Ctrl+U", section: "Navigation" },
    ShortcutDef { action: "Page down", keys: "PageDown / Ctrl+D", section: "Navigation" },
    ShortcutDef { action: "Next turn", keys: "Shift+→", section: "Navigation" },
    ShortcutDef { action: "Previous turn", keys: "Shift+←", section: "Navigation" },
    // Actions
    ShortcutDef { action: "Toggle thoughts", keys: "Ctrl+Shift+E", section: "Actions" },
    ShortcutDef { action: "Copy last response", keys: "Ctrl+Y / Ctrl+O", section: "Actions" },
    ShortcutDef { action: "Clear chat", keys: "Ctrl+L", section: "Actions" },
    ShortcutDef { action: "Toggle sidebar", keys: "Ctrl+B", section: "Actions" },
    // Session
    ShortcutDef { action: "New session", keys: "/new", section: "Session" },
    ShortcutDef { action: "Clear session", keys: "/clear", section: "Session" },
    ShortcutDef { action: "Show cost", keys: "/cost", section: "Session" },
    ShortcutDef { action: "Help", keys: "/help", section: "Session" },
];

/// Panel state.
#[derive(Debug, Clone, Default)]
pub struct ShortcutsPanel {
    pub open: bool,
    pub filter_mode: bool,
    pub filter: String,
    pub selected: usize,
    pub expanded_sections: Vec<&'static str>,
    pub filtered_indices: Vec<usize>,
}

impl ShortcutsPanel {
    pub fn new() -> Self {
        Self {
            expanded_sections: vec!["Essentials", "Input", "Navigation", "Actions", "Session"],
            ..Default::default()
        }
    }

    pub fn open(&mut self) {
        self.open = true;
        self.filter.clear();
        self.filter_mode = false;
        self.selected = 0;
        self.update_filtered();
    }

    pub fn close(&mut self) {
        self.open = false;
        self.filter_mode = false;
        self.filter.clear();
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn toggle_filter(&mut self) {
        self.filter_mode = !self.filter_mode;
        if !self.filter_mode {
            self.filter.clear();
        }
        self.update_filtered();
    }

    pub fn set_filter(&mut self, filter: &str) {
        self.filter = filter.to_string();
        self.update_filtered();
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.filtered_indices.len() {
            self.selected += 1;
        }
    }

    pub fn toggle_selected_section(&mut self) {
        if let Some(&idx) = self.filtered_indices.get(self.selected) {
            let section = SHORTCUTS[idx].section;
            if let Some(pos) = self.expanded_sections.iter().position(|&s| s == section) {
                self.expanded_sections.remove(pos);
            } else {
                self.expanded_sections.push(section);
            }
            self.update_filtered();
        }
    }

    fn update_filtered(&mut self) {
        if self.filter_mode && !self.filter.is_empty() {
            let query = self.filter.to_lowercase();
            self.filtered_indices = SHORTCUTS
                .iter()
                .enumerate()
                .filter(|(_, s)| {
                    s.action.to_lowercase().contains(&query)
                        || s.keys.to_lowercase().contains(&query)
                        || s.section.to_lowercase().contains(&query)
                })
                .map(|(i, _)| i)
                .collect();
        } else {
            // Show all shortcuts from expanded sections
            self.filtered_indices = SHORTCUTS
                .iter()
                .enumerate()
                .filter(|(_, s)| self.expanded_sections.contains(&s.section))
                .map(|(i, _)| i)
                .collect();
        }
        if self.selected >= self.filtered_indices.len() {
            self.selected = 0;
        }
    }
}

fn render_panel_border(area: Rect, buf: &mut Buffer, border: Color) {
    for x in area.x..area.right() {
        buf.get_mut(x, area.y).set_char('─').set_fg(border);
        buf.get_mut(x, area.bottom() - 1).set_char('─').set_fg(border);
    }
    for y in area.y..area.bottom() {
        buf.get_mut(area.x, y).set_char('│').set_fg(border);
        buf.get_mut(area.right() - 1, y).set_char('│').set_fg(border);
    }
    buf.get_mut(area.x, area.y).set_char('┌');
    buf.get_mut(area.right() - 1, area.y).set_char('┐');
    buf.get_mut(area.x, area.bottom() - 1).set_char('└');
    buf.get_mut(area.right() - 1, area.bottom() - 1).set_char('┘');
}

fn render_panel_header(area: Rect, buf: &mut Buffer, text_primary: Color, text_muted: Color) {
    let title = " Keyboard Shortcuts ";
    buf.set_line(area.x + 2, area.y, &Line::raw(title).style(Style::default().fg(text_primary).add_modifier(Modifier::BOLD)), area.width - 4);
    let close = " [✗] ";
    let close_x = area.right() - close.len() as u16 - 2;
    buf.set_line(close_x, area.y, &Line::raw(close).style(Style::default().fg(text_muted)), area.width);
}

fn render_panel_footer(panel: &ShortcutsPanel, area: Rect, buf: &mut Buffer, text_muted: Color) {
    let filter_hint = if panel.filter_mode { format!(" search: {} ", panel.filter) } else { " / to search | f filter | e expand | Esc close ".to_string() };
    buf.set_line(area.x + 2, area.bottom() - 1, &Line::raw(filter_hint).style(Style::default().fg(text_muted)), area.width - 4);
}

fn render_shortcut_row(shortcut: &ShortcutDef, is_selected: bool, inner_x: u16, inner_w: u16, y: u16, buf: &mut Buffer, accent: Color, text_primary: Color, text_muted: Color) {
    let indicator = if is_selected { "▸" } else { " " };
    let indicator_style = if is_selected { Style::default().fg(accent).add_modifier(Modifier::BOLD) } else { Style::default().fg(text_muted) };
    buf.set_string(inner_x, y, indicator, indicator_style);
    let action_x = inner_x + 2;
    let action_style = if is_selected { Style::default().fg(text_primary).add_modifier(Modifier::BOLD) } else { Style::default().fg(text_primary) };
    buf.set_string(action_x, y, shortcut.action, action_style);
    let keys_len = shortcut.keys.len() as u16;
    let keys_x = inner_x + inner_w - keys_len;
    if keys_x > action_x + shortcut.action.len() as u16 {
        buf.set_string(keys_x, y, shortcut.keys, Style::default().fg(text_muted));
    }
}

impl Widget for &ShortcutsPanel {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border = Color::DarkGray;
        let text_primary = Color::White;
        let text_muted = Color::DarkGray;
        let accent = Color::Cyan;
        let section_color = Color::Yellow;
        render_panel_border(area, buf, border);
        render_panel_header(area, buf, text_primary, text_muted);
        render_panel_footer(self, area, buf, text_muted);
        let inner_x = area.x + 2;
        let inner_w = area.width.saturating_sub(4);
        let mut y = area.y + 1;
        let max_y = area.bottom() - 1;
        let mut last_section: Option<&str> = None;
        for (display_idx, &idx) in self.filtered_indices.iter().enumerate() {
            if y >= max_y { break; }
            let shortcut = &SHORTCUTS[idx];
            let is_selected = display_idx == self.selected;
            if last_section != Some(shortcut.section) {
                if y > area.y + 1 { y += 1; }
                if y >= max_y { break; }
                let is_expanded = self.expanded_sections.contains(&shortcut.section);
                let arrow = if is_expanded { "▼" } else { "▶" };
                let count = SHORTCUTS.iter().filter(|s| s.section == shortcut.section).count();
                let section_text = format!("{} {} ({})", arrow, shortcut.section, count);
                let section_style = Style::default().fg(section_color).add_modifier(Modifier::BOLD);
                buf.set_line(inner_x, y, &Line::raw(section_text).style(section_style), inner_w);
                y += 1;
                if y >= max_y { break; }
                last_section = Some(shortcut.section);
            }
            render_shortcut_row(shortcut, is_selected, inner_x, inner_w, y, buf, accent, text_primary, text_muted);
            y += 1;
        }
    }
}

pub fn render_shortcuts_panel(panel: &ShortcutsPanel, area: Rect, buf: &mut Buffer, _theme: &ThemeWrapper) {
    panel.render(area, buf);
}
