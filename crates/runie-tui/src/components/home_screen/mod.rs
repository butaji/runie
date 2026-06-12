//! Home screen — grok-style welcome overlay.

use ratatui::{buffer::Buffer, layout::Rect, style::Style, widgets::Widget};
use crate::theme::ThemeWrapper;
use crate::style::box_chars::H as BOX_H;

pub static HOME_MENU_ITEMS: &[(&str, &str, &str)] = &[
    ("New session", "Start a new session", "ctrl-n"),
    ("New worktree", "Start a parallel task", "ctrl-w"),
    ("Resume session", "Continue where you left off", "ctrl-s"),
    ("Quit", "Exit runie", "ctrl-q"),
];

/// Tip text displayed below the menu
pub const TIP_TEXT: &str = "Tip: Press Ctrl-W to start a parallel task in its own worktree.";

// Logo centered in 78 columns, each line padded to max width
const LOGO_MAX_WIDTH: usize = 48;
const LOGO: &[&str] = &[
    "                              $$                              ",
    "                              \\__|                             ",
    " $$$$$$\\  $$\\   $$\\ $$$$$$$\\  $$\\  $$$$$$                      ",
    "$$  __$$\\ $$ |  $$ |$$  __$$\\ $$ |$$  __$$\\                    ",
    "$$ |  \\__|$$ |  $$ |$$ |  $$ |$$ |$$$$$$$$ |                   ",
    "$$ |      $$ |  $$ |$$ |  $$ |$$ |$$   ____|                   ",
    "$$ |      \\$$$$$$  |$$ |  $$ |$$ |\\$$$$$$$\\ $$\\                 ",
    "\\__|       \\______/ \\__|  \\__|\\__| \\_______|\\__|               ",
    "                                                              ",
    "                                                              ",
];

#[derive(Debug, Clone, Default)]
pub struct HomeScreen {
    pub visible: bool,
    pub selected: usize,
    pub show_sessions: bool,
}

impl HomeScreen {
    pub fn new() -> Self { Self { visible: true, selected: 0, show_sessions: false } }
    pub fn show(&mut self) { self.visible = true; self.selected = 0; self.show_sessions = false; }
    pub fn hide(&mut self) { self.visible = false; }
    pub fn is_visible(&self) -> bool { self.visible }
    pub fn move_up(&mut self) { if !self.show_sessions && self.selected > 0 { self.selected -= 1; } }
    pub fn move_down(&mut self) { if !self.show_sessions && self.selected + 1 < HOME_MENU_ITEMS.len() { self.selected += 1; } }
    pub fn toggle_sessions(&mut self) { self.show_sessions = !self.show_sessions; self.selected = 0; }
    pub fn selected_action(&self) -> &str { HOME_MENU_ITEMS.get(self.selected).map(|(a, _, _)| *a).unwrap_or("") }
}

fn draw_divider(x: u16, y: u16, content_width: u16, buf: &mut Buffer, style: Style) {
    for dx in x..x.saturating_add(content_width.saturating_sub(3)) {
        if let Some(cell) = buf.cell_mut((dx, y)) { cell.set_char(BOX_H).set_style(style); }
    }
}

fn draw_menu_item(name: &str, hint: &str, x: u16, y: u16, content_width: u16, buf: &mut Buffer, unselected_style: Style, hint_style: Style) {
    buf.set_string(x, y, name, unselected_style);
    buf.set_string(x.saturating_add(content_width.saturating_sub(hint.len() as u16 + 3)), y, hint, hint_style);
}

fn render_menu(screen: &HomeScreen, content_x: u16, start_y: u16, content_width: u16, buf: &mut Buffer, _selected_style: Style, unselected_style: Style, hint_style: Style, divider_style: Style) {
    let mut y = start_y;
    for (i, (name, _, hint)) in HOME_MENU_ITEMS.iter().enumerate() {
        draw_menu_item(name, hint, content_x, y, content_width, buf, unselected_style, hint_style);
        y += 1;
        if i < HOME_MENU_ITEMS.len() - 1 { draw_divider(content_x, y, content_width, buf, divider_style); y += 1; }
    }
}

fn render_tip(area: Rect, y: u16, buf: &mut Buffer, tip_style: Style) {
    // Center the tip text across the full terminal width
    let tip_x = (area.width.saturating_sub(TIP_TEXT.len() as u16)) / 2;
    buf.set_string(tip_x, y, TIP_TEXT, tip_style);
}

fn render_logo(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper, start_y: u16) -> u16 {
    let logo_height = LOGO.len() as u16;
    let logo_width = LOGO_MAX_WIDTH as u16;
    let logo_x = area.x.saturating_add(area.width.saturating_sub(logo_width) / 2);
    let logo_style = theme.menu_unselected_style();
    for (i, line) in LOGO.iter().enumerate() { buf.set_string(logo_x, start_y.saturating_add(i as u16), *line, logo_style); }
    logo_height
}

impl Widget for &HomeScreen {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = ThemeWrapper::default();
        render_home_screen(self, area, buf, &theme);
    }
}

pub fn render_home_screen(screen: &HomeScreen, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    use crate::style::layout::MENU_WIDTH;
    let content_width = MENU_WIDTH;
    let logo_lines = LOGO.len() as u16;
    let menu_height = 4 + 3; // 4 menu items + 3 dividers
    let spacing = 3;
    let tip_height = 1;
    let total_height = logo_lines + spacing + menu_height + spacing + tip_height;
    let top_margin = (area.height.saturating_sub(1).saturating_sub(total_height)) / 2;
    let content_start_y = area.y.saturating_add(top_margin);
    render_logo(area, buf, theme, content_start_y);
    let content_x = area.x.saturating_add(area.width.saturating_sub(content_width) / 2 + 2);
    render_menu(screen, content_x, content_start_y + logo_lines + spacing, content_width, buf,
        theme.menu_selected_style(), theme.menu_unselected_style(), theme.muted_style(), theme.divider_style());
    // Render tip text below the menu - centered on full terminal width
    let tip_y = content_start_y + logo_lines + spacing + menu_height + spacing;
    render_tip(area, tip_y, buf, theme.muted_style());
}

#[cfg(test)] mod mod_test;
#[cfg(test)] mod render_test;
