//! Home screen — grok-style welcome overlay.
//!
//! Shows when no session is active. Simple 3-item menu with keyboard hints.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};
use crate::theme::{ThemeColors, ThemeWrapper};

/// Home screen menu items: (label, hint)
pub static HOME_MENU_ITEMS: &[(&str, &str)] = &[
    ("New worktree", "ctrl-w"),
    ("Resume session", "ctrl-s"),
    ("Quit", "ctrl-q"),
];

/// Home screen state.
#[derive(Debug, Clone, Default)]
pub struct HomeScreen {
    pub visible: bool,
    pub selected: usize,
    /// When true, show session list instead of welcome menu
    pub show_sessions: bool,
}

impl HomeScreen {
    pub fn new() -> Self {
        Self {
            visible: true,
            selected: 0,
            show_sessions: false,
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.selected = 0;
        self.show_sessions = false;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn move_up(&mut self) {
        if self.show_sessions {
            // Session list has its own navigation
            return;
        }
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.show_sessions {
            // Session list has its own navigation
            return;
        }
        if self.selected + 1 < HOME_MENU_ITEMS.len() {
            self.selected += 1;
        }
    }

    pub fn toggle_sessions(&mut self) {
        self.show_sessions = !self.show_sessions;
        self.selected = 0;
    }

    pub fn selected_action(&self) -> &str {
        HOME_MENU_ITEMS.get(self.selected).map(|(action, _)| *action).unwrap_or("")
    }
}

fn fill_bg(area: Rect, buf: &mut Buffer, bg: Color) {
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_bg(bg);
            }
        }
    }
}

fn draw_divider(x: u16, y: u16, content_x: u16, content_width: u16, buf: &mut Buffer, color: Color) {
    let divider_x = x + 2; // Align with menu text
    let divider_width = content_width.saturating_sub(3);
    for dx in divider_x..divider_x + divider_width {
        if let Some(cell) = buf.cell_mut((dx, y)) {
            cell.set_char('─').set_fg(color);
        }
    }
}

fn draw_menu_item(
    name: &str,
    hint: &str,
    x: u16,
    y: u16,
    content_width: u16,
    buf: &mut Buffer,
    text_primary: Color,
    text_muted: Color,
    _accent: Color,
    is_selected: bool,
) {
    let name_style = if is_selected {
        Style::default().fg(text_primary).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(text_primary)
    };
    // FIXED left alignment (not centered) - matches Grok
    buf.set_string(x + 2, y, name, name_style);
    let hint_len = hint.len() as u16;
    let hint_x = x + content_width.saturating_sub(hint_len + 1);
    buf.set_string(hint_x, y, hint, Style::default().fg(text_muted));
}

fn render_menu(
    screen: &HomeScreen,
    content_x: u16,
    start_y: u16,
    content_width: u16,
    buf: &mut Buffer,
    text_primary: Color,
    text_muted: Color,
    accent: Color,
    border: Color,
) {
    let mut y = start_y;
    let item_count = HOME_MENU_ITEMS.len();

    for (i, (name, hint)) in HOME_MENU_ITEMS.iter().enumerate() {
        let is_selected = i == screen.selected;
        draw_menu_item(name, hint, content_x, y, content_width, buf, text_primary, text_muted, accent, is_selected);
        y += 1;
        if i < item_count - 1 {
            draw_divider(content_x, y, content_x, content_width, buf, border);
            y += 1;
        }
    }
}

impl Widget for &HomeScreen {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let colors = ThemeColors::from(&ThemeWrapper::default());
        let (border, text_primary, text_muted, accent) = (
            colors.border_unfocused,
            colors.text_primary,
            colors.text_dim,
            colors.accent_primary,
        );

        let content_width = 40u16;
        let content_height = 14u16;
        let content_x = area.x + (area.width.saturating_sub(content_width)) / 2;
        let content_y = area.y + (area.height.saturating_sub(content_height)) / 2;

        let menu_start_y = content_y;
        render_menu(
            self,
            content_x,
            menu_start_y,
            content_width,
            buf,
            text_primary,
            text_muted,
            accent,
            border,
        );

        let tip = "Tip: Press Ctrl-W to start a parallel task in its own worktree.";
        buf.set_string(area.x, content_y + 11, tip, Style::default().fg(text_muted));
    }
}

pub fn render_home_screen(screen: &HomeScreen, area: Rect, buf: &mut Buffer, _theme: &ThemeWrapper) {
    screen.render(area, buf);
}
