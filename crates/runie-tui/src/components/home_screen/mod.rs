//! Home screen — grok-style welcome overlay.
//!
//! Shows when no session is active. Menu items with keyboard hints.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::Widget,
};
use crate::theme::{ThemeColors, ThemeWrapper};

/// Home screen menu items.
pub static HOME_MENU_ITEMS: &[(&str, &str, &str)] = &[
    ("New Session", "Start a new chat", "n"),
    ("Resume Last Session", "Continue where you left off", "r"),
    ("Settings", "Configure preferences", "s"),
    ("Help", "Show keyboard shortcuts", "h"),
    ("Quit", "Exit runie", "q"),
];

/// Rotating tips for the home screen.
pub static HOME_TIPS: &[&str] = &[
    "Press Ctrl-W to start a parallel task",
    "Press Ctrl-Space to toggle permission mode",
    "Use /plan to enter plan mode before executing",
    "Press Tab to auto-complete paths and commands",
    "Use /attach to attach files to your message",
    "Press Ctrl-K to open the command palette",
];

/// Home screen state.
#[derive(Debug, Clone, Default)]
pub struct HomeScreen {
    pub visible: bool,
    pub selected: usize,
    pub tip_index: usize,
    pub recent_sessions: Vec<RecentSession>,
}

/// A recently used session.
#[derive(Debug, Clone)]
pub struct RecentSession {
    pub id: String,
    pub title: String,
    pub timestamp: String,
}

impl HomeScreen {
    pub fn new() -> Self {
        Self {
            visible: true,
            selected: 0,
            tip_index: 0,
            recent_sessions: Vec::new(),
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.selected = 0;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < HOME_MENU_ITEMS.len() {
            self.selected += 1;
        }
    }

    pub fn selected_action(&self) -> &str {
        HOME_MENU_ITEMS.get(self.selected).map(|(action, _, _)| *action).unwrap_or("")
    }

    /// Rotate to the next tip.
    pub fn rotate_tip(&mut self) {
        self.tip_index = (self.tip_index + 1) % HOME_TIPS.len();
    }

    /// Get the current tip.
    pub fn current_tip(&self) -> &str {
        HOME_TIPS[self.tip_index]
    }
}

fn render_home_bg(area: Rect, buf: &mut Buffer, bg: Color) {
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            if let Some(cell) = buf.cell_mut((x, y)) { cell.set_bg(bg); }
        }
    }
}

/// Render ASCII art logo - simplified version.
fn render_ascii_logo(content_x: u16, content_y: u16, buf: &mut Buffer, accent_primary: Color) {
    let logo_lines = [
        "  ███╗   ███╗ █████╗ ██╗███╗   ██╗    ██████╗  █████╗  ██████╗██╗  ██╗███████╗██████╗ ",
        "  ████╗ ████║██╔══██╗██║████╗  ██║    ██╔══██╗██╔══██╗██╔════╝██║ ██╔╝██╔════╝██╔══██╗",
        "  ██╔████╔██║███████║██║██╔██╗ ██║    ██████╔╝███████║██║     █████╔╝ █████╗  ██████╔╝",
        "  ██║╚██╔╝██║██╔══██║██║██║╚██╗██║    ██╔═══╝ ██╔══██║██║     ██╔═██╗ ██╔══╝  ██╔══██╗",
        "  ██║ ╚═╝ ██║██║  ██║██║██║ ╚████║    ██║     ██║  ██║╚██████╗██║  ██╗███████╗██║  ██║",
        "  ╚═╝     ╚═╝╚═╝  ╚═╝╚═╝╚═╝  ╚═══╝    ╚═╝     ╚═╝  ╚═╝ ╚═════╝╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝",
    ];

    let start_x = content_x + 1;
    let line_height = 1;

    for (i, line) in logo_lines.iter().enumerate() {
        let y = content_y + (i as u16) * line_height;
        if y >= content_y + 8 { break; }
        buf.set_string(start_x, y, line, Style::default().fg(accent_primary));
    }
}

fn render_home_title(content_x: u16, content_y: u16, content_width: u16, buf: &mut Buffer, text_primary: Color, text_muted: Color) {
    let title = "runie";
    let title_x = content_x + (content_width - title.len() as u16) / 2;
    buf.set_line(title_x, content_y, &Line::raw(title).style(Style::default().fg(text_primary).add_modifier(Modifier::BOLD)), content_width);
    let subtitle = "Your coding companion";
    let subtitle_x = content_x + (content_width - subtitle.len() as u16) / 2;
    buf.set_line(subtitle_x, content_y + 1, &Line::raw(subtitle).style(Style::default().fg(text_muted)), content_width);
}

fn render_recent_sessions(screen: &HomeScreen, content_x: u16, start_y: u16, content_width: u16, buf: &mut Buffer, text_muted: Color) {
    if screen.recent_sessions.is_empty() {
        return;
    }

    // Section header
    buf.set_string(content_x + 2, start_y, "Recent", Style::default().fg(text_muted).add_modifier(Modifier::BOLD));

    let mut y = start_y + 2;
    for session in screen.recent_sessions.iter().take(3) {
        if y >= start_y + 10 { break; }
        buf.set_string(content_x + 4, y, &session.title, Style::default().fg(text_muted));
        buf.set_string(content_x + content_width - session.timestamp.len() as u16 - 4, y, &session.timestamp, Style::default().fg(text_muted));
        y += 2;
    }
}

fn render_home_menu(screen: &HomeScreen, area: Rect, content_x: u16, div_y: u16, content_width: u16, buf: &mut Buffer, text_primary: Color, text_muted: Color, accent: Color) {
    let mut y = div_y + 2;
    for (i, (name, desc, hint)) in HOME_MENU_ITEMS.iter().enumerate() {
        if y >= area.bottom() { break; }
        let is_selected = i == screen.selected;
        let indicator = if is_selected { "▸" } else { " " };
        let indicator_style = if is_selected { Style::default().fg(accent).add_modifier(Modifier::BOLD) } else { Style::default().fg(text_muted) };
        buf.set_string(content_x + 2, y, indicator, indicator_style);
        let name_style = if is_selected { Style::default().fg(text_primary).add_modifier(Modifier::BOLD) } else { Style::default().fg(text_primary) };
        buf.set_string(content_x + 4, y, name, name_style);
        let hint_text = format!("{}", hint);
        let hint_x = content_x + content_width - hint_text.len() as u16 - 2;
        buf.set_string(hint_x, y, &hint_text, Style::default().fg(text_muted));
        let desc_y = y + 1;
        if desc_y < area.bottom() {
            buf.set_string(content_x + 4, desc_y, desc, Style::default().fg(text_muted));
        }
        y += 3;
    }
}

impl Widget for &HomeScreen {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = ThemeWrapper::default();
        let colors = ThemeColors::from(&theme);

        let bg = colors.bg_base;
        let border = colors.border_unfocused;
        let text_primary = colors.text_primary;
        let text_muted = colors.text_dim;
        let accent = colors.accent_primary;

        render_home_bg(area, buf, bg);

        let content_width = 80u16;
        let content_height = 26u16;
        let content_x = area.x + (area.width.saturating_sub(content_width)) / 2;
        let content_y = area.y + (area.height.saturating_sub(content_height)) / 2;

        // Render ASCII logo
        render_ascii_logo(content_x, content_y, buf, accent);

        // Title and subtitle below logo
        render_home_title(content_x, content_y + 8, content_width, buf, text_primary, text_muted);

        // Divider
        let div_y = content_y + 11;
        for x in content_x..content_x + content_width {
            buf.cell_mut((x, div_y)).map(|cell| cell.set_char('─').set_fg(border));
        }

        // Recent sessions (if any)
        render_recent_sessions(self, content_x, div_y + 2, content_width, buf, text_muted);

        // Menu
        let menu_start_y = if self.recent_sessions.is_empty() {
            div_y + 2
        } else {
            div_y + 10
        };
        render_home_menu(self, area, content_x, menu_start_y, content_width, buf, text_primary, text_muted, accent);

        // Footer with navigation hints
        let footer = "↑/↓ navigate · Enter select · q quit";
        let footer_y = area.bottom().saturating_sub(3);
        let footer_x = content_x + (content_width - footer.len() as u16) / 2;
        buf.set_line(footer_x, footer_y, &Line::raw(footer).style(Style::default().fg(text_muted)), content_width);

        // Tip line
        let tip = format!("Tip: {}", self.current_tip());
        let tip_x = content_x + (content_width - tip.len() as u16) / 2;
        buf.set_line(tip_x, footer_y + 1, &Line::raw(tip).style(Style::default().fg(text_muted)), content_width);

        // Version badge (bottom-right)
        let version = env!("CARGO_PKG_VERSION");
        let version_badge = format!("{} Beta", version);
        let badge_x = area.right().saturating_sub(version_badge.len() as u16 + 2);
        let badge_y = area.bottom().saturating_sub(1);
        buf.set_string(badge_x, badge_y, &version_badge, Style::default().fg(text_muted));
    }
}

pub fn render_home_screen(screen: &HomeScreen, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    screen.render(area, buf);
}
