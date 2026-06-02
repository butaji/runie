//! Home screen — grok-style welcome overlay.
//!
//! Shows when no session is active. Simple 3-item menu with keyboard hints.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::Widget,
};
use crate::theme::ThemeWrapper;
use crate::style::box_chars::H as BOX_H;

/// Home screen menu items: (name, description, hint)
pub static HOME_MENU_ITEMS: &[(&str, &str, &str)] = &[
    ("New Session", "Start a new chat", "ctrl-n"),
    ("Resume Last Session", "Continue where you left off", "ctrl-r"),
    ("Settings", "Configure preferences", "ctrl-s"),
    ("Help", "Show keyboard shortcuts", "ctrl-h"),
    ("Quit", "Exit runie", "ctrl-q"),
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
        HOME_MENU_ITEMS.get(self.selected).map(|(action, _, _)| *action).unwrap_or("")
    }
}

fn draw_divider(x: u16, y: u16, content_x: u16, content_width: u16, buf: &mut Buffer, style: Style) {
    let divider_x = x + 2; // Align with menu text
    let divider_width = content_width.saturating_sub(3);
    for dx in divider_x..divider_x + divider_width {
        if let Some(cell) = buf.cell_mut((dx, y)) {
            cell.set_char(BOX_H).set_style(style);
        }
    }
}

fn draw_menu_item(
    name: &str,
    desc: &str,
    hint: &str,
    x: u16,
    y: u16,
    content_width: u16,
    buf: &mut Buffer,
    selected_style: Style,
    unselected_style: Style,
    hint_style: Style,
    is_selected: bool,
) {
    let indicator = if is_selected { "▸" } else { " " };
    let indicator_style = if is_selected { selected_style } else { unselected_style };
    buf.set_string(x + 2, y, indicator, indicator_style);

    let name_style = if is_selected { selected_style } else { unselected_style };
    buf.set_string(x + 4, y, name, name_style);

    let hint_text = format!(" ({}) ", hint);
    let hint_len = hint_text.len() as u16;
    let hint_x = x + content_width.saturating_sub(hint_len + 2);
    buf.set_string(hint_x, y, &hint_text, hint_style);

    let desc_y = y + 1;
    let max_desc_width = content_width - 8 - hint_len - 2;
    let desc_text = if desc.len() as u16 > max_desc_width {
        format!("{}...", &desc[..max_desc_width as usize - 3])
    } else {
        desc.to_string()
    };
    buf.set_string(x + 4, desc_y, &desc_text, unselected_style);
}

fn render_menu(
    screen: &HomeScreen,
    content_x: u16,
    start_y: u16,
    content_width: u16,
    buf: &mut Buffer,
    selected_style: Style,
    unselected_style: Style,
    hint_style: Style,
    divider_style: Style,
) {
    let mut y = start_y;
    let item_count = HOME_MENU_ITEMS.len();

    for (i, (name, desc, hint)) in HOME_MENU_ITEMS.iter().enumerate() {
        let is_selected = i == screen.selected;
        draw_menu_item(name, desc, hint, content_x, y, content_width, buf, selected_style, unselected_style, hint_style, is_selected);
        y += 2;
        if i < item_count - 1 {
            draw_divider(content_x, y, content_x, content_width, buf, divider_style);
            y += 1;
        }
    }
}

impl Widget for &HomeScreen {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = ThemeWrapper::default();
        render_home_screen(self, area, buf, &theme);
    }
}

pub fn render_home_screen(screen: &HomeScreen, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    use crate::style::layout::{MENU_WIDTH, MENU_HEIGHT};

    let content_width = MENU_WIDTH;
    let content_height = MENU_HEIGHT;
    let content_x = area.x + (area.width.saturating_sub(content_width)) / 2;
    let content_y = area.y + (area.height.saturating_sub(content_height)) / 2;

    let menu_start_y = content_y;
    render_menu(
        screen,
        content_x,
        menu_start_y,
        content_width,
        buf,
        theme.menu_selected_style(),
        theme.menu_unselected_style(),
        theme.muted_style(),
        theme.divider_style(),
    );

    let tip = "Tip: Press Ctrl-W to start a parallel task in its own worktree.";
    buf.set_string(area.x + 2, content_y + 11, tip, theme.tip_style());
}
