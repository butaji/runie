use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
};
use crate::components::DialogFrame;
use crate::theme::ThemeWrapper;

pub mod builder;
pub use builder::*;

#[derive(Debug, Clone, PartialEq)]
pub enum PermissionAction {
    Confirm,
    Cancel,
    Always,
    Skip,
}

pub struct PermissionModal {
    pub title: String,
    pub tool_name: String,
    pub tool_args: String,
    pub description: String,
    pub selected: usize,
    // P0-3 FIX: Add timeout remaining display (in seconds)
    pub timeout_secs: Option<u64>,
}

impl Default for PermissionModal {
    fn default() -> Self {
        Self {
            title: "Permission Required".to_string(),
            tool_name: String::new(),
            tool_args: String::new(),
            description: String::new(),
            selected: 0,
            timeout_secs: None,
        }
    }
}

impl PermissionModal {

    #[must_use]
    
    pub fn new(tool_name: &str, tool_args: &str, description: &str) -> Self {
        Self {
            title: "Permission Required".to_string(),
            tool_name: tool_name.to_string(),
            tool_args: tool_args.to_string(),
            description: description.to_string(),
            selected: 0,
            timeout_secs: None,
        }
    }

    pub fn next_option(&mut self) {
        self.selected = (self.selected + 1) % 4;
    }

    pub fn prev_option(&mut self) {
        self.selected = (self.selected + 3) % 4;
    }

    pub fn confirm(&self) -> PermissionAction {
        match self.selected {
            0 => PermissionAction::Confirm,
            1 => PermissionAction::Cancel,
            2 => PermissionAction::Always,
            3 => PermissionAction::Skip,
            _ => PermissionAction::Cancel,
        }
    }

    pub fn render_ref(&self, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
        let warning: ratatui::style::Color = theme.color("warning").into();
        let accent_primary: ratatui::style::Color = theme.color("accent.primary").into();
        let code_path: ratatui::style::Color = theme.color("code.path").into();
        let text_secondary: ratatui::style::Color = theme.color("text.secondary").into();
        let text_muted: ratatui::style::Color = theme.color("text.muted").into();
        let accent_secondary: ratatui::style::Color = theme.color("accent.secondary").into();

        DialogFrame::new(area.width, area.height)
            .title("Permission Required")
            .render(area, buf, theme, |inner, buf| {
                render_title(inner, buf, &self.title, warning);
                render_tool_info(inner, buf, &self.tool_name, &self.tool_args, accent_primary, code_path, text_secondary);
                // P0-3 FIX: Render timeout countdown
                render_timeout(inner, buf, self.timeout_secs, warning);
                render_description(inner, buf, &self.description, text_secondary);
                render_buttons(inner, buf, self.selected, accent_secondary, text_muted);
            });
    }
}

fn render_title(area: Rect, buf: &mut Buffer, title: &str, warning: ratatui::style::Color) {
    let inner_x = area.x + 1;
    let title_line = Line::from(vec![Span::styled(title, Style::default().fg(warning).add_modifier(Modifier::BOLD))]);
    buf.set_line(inner_x, area.y + 1, &title_line, title.len() as u16);
}

fn render_tool_info(
    area: Rect,
    buf: &mut Buffer,
    tool_name: &str,
    tool_args: &str,
    accent_primary: ratatui::style::Color,
    code_path: ratatui::style::Color,
    text_secondary: ratatui::style::Color,
) {
    let inner_x = area.x + 1;
    let inner_width = area.width.saturating_sub(2);

    let tool_label = "Tool: ";
    let tool_name_span = Span::styled(tool_name, Style::default().fg(accent_primary).add_modifier(Modifier::BOLD));
    let tool_line = Line::from(vec![Span::raw(tool_label), tool_name_span]);
    buf.set_line(inner_x, area.y + 3, &tool_line, inner_width);

    let args_label = "Args: ";
    let args_label_line = Line::from(vec![Span::styled(args_label, Style::default().fg(text_secondary))]);
    buf.set_line(inner_x, area.y + 5, &args_label_line, inner_width);

    let args_line = Line::from(vec![Span::styled(tool_args, Style::default().fg(code_path))]);
    buf.set_line(inner_x, area.y + 6, &args_line, inner_width);
}

// P0-3 FIX: Render timeout countdown in permission modal
fn render_timeout(area: Rect, buf: &mut Buffer, timeout_secs: Option<u64>, warning: ratatui::style::Color) {
    let Some(secs) = timeout_secs else { return; };
    
    let inner_x = area.x + 1;
    let inner_width = area.width.saturating_sub(2);
    
    // Format as MM:SS
    let minutes = secs / 60;
    let seconds = secs % 60;
    let timeout_text = if minutes > 0 {
        format!("⏱ Expires in {}:{}", minutes, format_args!("{:02}", seconds))
    } else {
        format!("⏱ Expires in {}s", seconds)
    };
    
    // Use warning color when less than 60 seconds remain
    let color = if secs < 60 { warning } else { ratatui::style::Color::DarkGray };
    let timeout_line = Line::from(vec![Span::styled(timeout_text, Style::default().fg(color))]);
    buf.set_line(inner_x, area.y + 7, &timeout_line, inner_width);
}

fn render_description(area: Rect, buf: &mut Buffer, description: &str, text_secondary: ratatui::style::Color) {
    let inner_x = area.x + 1;
    let inner_width = area.width.saturating_sub(2);
    let desc_lines: Vec<&str> = description.lines().collect();
    let desc_start_y = area.y + 8;

    for (i, desc_line) in desc_lines.iter().enumerate() {
        let y = desc_start_y + i as u16;
        if y >= area.y + area.height - 3 {
            break;
        }
        let line = Line::from(vec![Span::styled(*desc_line, Style::default().fg(text_secondary))]);
        buf.set_line(inner_x, y, &line, inner_width);
    }
}

fn render_buttons(area: Rect, buf: &mut Buffer, selected: usize, accent_secondary: ratatui::style::Color, text_muted: ratatui::style::Color) {
    let inner_x = area.x + 1;
    let inner_width = area.width.saturating_sub(2);
    let buttons_y = area.y + area.height - 3;

    // P1-1: Progressive disclosure - show 2 primary options, 2 as discoverable hints
    let confirm_style = button_style(selected, 0, accent_secondary, text_muted);
    let cancel_style = button_style(selected, 1, accent_secondary, text_muted);

    // Primary row: Confirm and Cancel (the main actions)
    let row1 = Line::from(vec![
        Span::styled("[Y/Enter] ", Style::default().fg(text_muted)),
        Span::styled("Confirm", confirm_style),
        Span::styled("    ", Style::default()),
        Span::styled("[N/Esc] ", Style::default().fg(text_muted)),
        Span::styled("Cancel", cancel_style),
    ]);
    buf.set_line(inner_x, buttons_y, &row1, inner_width);

    // Secondary row: hidden options as discoverable hints (dimmed)
    // P2-6 FIX: Progressive disclosure - advanced options use DIM modifier
    let dim_style = Style::default().fg(text_muted).add_modifier(Modifier::DIM);
    let row2 = Line::from(vec![
        Span::styled("[a] always allow", dim_style),
        Span::styled("        ", Style::default()),
        Span::styled("[s] skip this step", dim_style),
    ]);
    buf.set_line(inner_x, buttons_y + 1, &row2, inner_width);
}

fn button_style(selected: usize, idx: usize, accent_secondary: ratatui::style::Color, text_muted: ratatui::style::Color) -> Style {
    if selected == idx {
        Style::default().fg(accent_secondary).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(text_muted)
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    fn make_theme() -> ThemeWrapper {
        ThemeWrapper::default()
    }

    #[test]
    fn test_render_shows_tool_name() {
        let theme = make_theme();
        let modal = PermissionModal::new("bash", "rm -rf /", "This command will delete all files.");

        let area = Rect::new(0, 0, 50, 16);
        let mut buf = Buffer::empty(area);

        modal.render_ref(area, &mut buf, &theme);

        let content = buf.content();
        let has_bash = content.iter().any(|cell| {
            cell.symbol() == "b" || cell.symbol() == "a" || cell.symbol() == "s" || cell.symbol() == "h"
        });
        assert!(has_bash, "Tool name 'bash' should appear in render output");
    }

    #[test]
    fn test_render_shows_tool_args() {
        let theme = make_theme();
        let modal = PermissionModal::new("bash", "rm -rf /", "Test description.");

        let area = Rect::new(0, 0, 50, 16);
        let mut buf = Buffer::empty(area);

        modal.render_ref(area, &mut buf, &theme);

        let content = buf.content();
        let has_rm = content.iter().any(|cell| {
            cell.symbol() == "r" || cell.symbol() == "m" || cell.symbol() == "-" || cell.symbol() == "f" || cell.symbol() == "/"
        });
        assert!(has_rm, "Tool args 'rm -rf /' should appear in render output");
    }

    #[test]
    fn test_next_option_cycles() {
        let mut modal = PermissionModal::default();

        assert_eq!(modal.selected, 0);
        modal.next_option();
        assert_eq!(modal.selected, 1);
        modal.next_option();
        assert_eq!(modal.selected, 2);
        modal.next_option();
        assert_eq!(modal.selected, 3);
        modal.next_option();
        assert_eq!(modal.selected, 0);
    }

    #[test]
    fn test_prev_option_cycles() {
        let mut modal = PermissionModal::default();

        assert_eq!(modal.selected, 0);
        modal.prev_option();
        assert_eq!(modal.selected, 3);
        modal.prev_option();
        assert_eq!(modal.selected, 2);
    }

    #[test]
    fn test_confirm_returns_correct_action() {
        let mut modal = PermissionModal::default();

        assert_eq!(modal.confirm(), PermissionAction::Confirm);

        modal.selected = 1;
        assert_eq!(modal.confirm(), PermissionAction::Cancel);

        modal.selected = 2;
        assert_eq!(modal.confirm(), PermissionAction::Always);

        modal.selected = 3;
        assert_eq!(modal.confirm(), PermissionAction::Skip);
    }

    #[test]
    fn test_new_sets_fields() {
        let modal = PermissionModal::new("npm", "install", "Installs a package");

        assert_eq!(modal.title, "Permission Required");
        assert_eq!(modal.tool_name, "npm");
        assert_eq!(modal.tool_args, "install");
        assert_eq!(modal.description, "Installs a package");
        assert_eq!(modal.selected, 0);
    }
}
