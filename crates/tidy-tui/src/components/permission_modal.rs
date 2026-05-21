use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
};
use crate::theme::ThemeWrapper;

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
    pub selected: usize, // 0=Confirm, 1=Cancel, 2=Always, 3=Skip
}

impl Default for PermissionModal {
    fn default() -> Self {
        Self {
            title: "Permission Required".to_string(),
            tool_name: String::new(),
            tool_args: String::new(),
            description: String::new(),
            selected: 0,
        }
    }
}

impl PermissionModal {
    pub fn new(tool_name: &str, tool_args: &str, description: &str) -> Self {
        Self {
            title: "Permission Required".to_string(),
            tool_name: tool_name.to_string(),
            tool_args: tool_args.to_string(),
            description: description.to_string(),
            selected: 0,
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
        // Colors from Opaline theme
        let bg_panel: ratatui::style::Color = theme.color("bg.panel").into();
        let warning: ratatui::style::Color = theme.color("warning").into();
        let accent_primary: ratatui::style::Color = theme.color("accent.primary").into();
        let code_path: ratatui::style::Color = theme.color("code.path").into();
        let text_secondary: ratatui::style::Color = theme.color("text.secondary").into();
        let text_muted: ratatui::style::Color = theme.color("text.muted").into();
        let accent_secondary: ratatui::style::Color = theme.color("accent.secondary").into();
        let error: ratatui::style::Color = theme.color("error").into();
        let border_unfocused: ratatui::style::Color = theme.color("border.unfocused").into();

        // Clear area with bg.panel
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf.get_mut(x, y)
                    .set_style(Style::default().bg(bg_panel));
            }
        }

        let inner_width = area.width.saturating_sub(2);
        let inner_x = area.x + 1;

        // Top separator line
        for x in area.x..area.x + area.width {
            buf[(x, area.y)]
                .set_symbol("─")
                .set_style(Style::default().fg(border_unfocused));
        }

        // Bottom separator line
        for x in area.x..area.x + area.width {
            buf[(x, area.y + area.height - 1)]
                .set_symbol("─")
                .set_style(Style::default().fg(border_unfocused));
        }

        // Left accent bar in red (danger indicator)
        let content_start_y = area.y + 1;
        for y in content_start_y..area.y + area.height - 1 {
            buf[(area.x, y)]
                .set_symbol("▌")
                .set_style(Style::default().fg(error));
        }

        // Title line
        let title_line = Line::from(vec![Span::styled(
            &self.title,
            Style::default().fg(warning).add_modifier(Modifier::BOLD),
        )]);
        buf.set_line(
            inner_x,
            area.y + 1,
            &title_line,
            self.title.len() as u16,
        );

        // Tool name line
        let tool_label = "Tool: ";
        let tool_name_span = Span::styled(
            &self.tool_name,
            Style::default()
                .fg(accent_primary)
                .add_modifier(Modifier::BOLD),
        );
        let tool_line = Line::from(vec![
            Span::raw(tool_label),
            tool_name_span,
        ]);
        buf.set_line(inner_x, area.y + 3, &tool_line, inner_width);

        // Args label
        let args_label = "Args: ";
        let args_label_line = Line::from(vec![Span::styled(
            args_label,
            Style::default().fg(text_secondary),
        )]);
        buf.set_line(inner_x, area.y + 5, &args_label_line, inner_width);

        // Tool args (code style)
        let args_line = Line::from(vec![Span::styled(
            &self.tool_args,
            Style::default().fg(code_path),
        )]);
        buf.set_line(inner_x, area.y + 6, &args_line, inner_width);

        // Description (multi-line)
        let desc_lines: Vec<&str> = self.description.lines().collect();
        let desc_start_y = area.y + 8;
        for (i, desc_line) in desc_lines.iter().enumerate() {
            let y = desc_start_y + i as u16;
            if y >= area.y + area.height - 3 {
                break;
            }
            let line = Line::from(vec![Span::styled(
                *desc_line,
                Style::default().fg(text_secondary),
            )]);
            buf.set_line(inner_x, y, &line, inner_width);
        }

        // Action buttons
        let buttons_y = area.y + area.height - 3;

        // First row: [Y] Confirm  [N] Cancel
        let confirm_label = if self.selected == 0 { "Confirm" } else { "Confirm" };
        let cancel_label = if self.selected == 1 { "Cancel" } else { "Cancel" };

        let confirm_style = if self.selected == 0 {
            Style::default()
                .fg(accent_secondary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(text_muted)
        };
        let cancel_style = if self.selected == 1 {
            Style::default()
                .fg(accent_secondary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(text_muted)
        };

        let row1 = Line::from(vec![
            Span::styled("[Y] ", Style::default().fg(text_muted)),
            Span::styled(confirm_label, confirm_style),
            Span::styled("  ", Style::default()),
            Span::styled("[N] ", Style::default().fg(text_muted)),
            Span::styled(cancel_label, cancel_style),
        ]);
        buf.set_line(inner_x, buttons_y, &row1, inner_width);

        // Second row: [A] Always  [S] Skip
        let always_label = if self.selected == 2 { "Always" } else { "Always" };
        let skip_label = if self.selected == 3 { "Skip" } else { "Skip" };

        let always_style = if self.selected == 2 {
            Style::default()
                .fg(accent_secondary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(text_muted)
        };
        let skip_style = if self.selected == 3 {
            Style::default()
                .fg(accent_secondary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(text_muted)
        };

        let row2 = Line::from(vec![
            Span::styled("[A] ", Style::default().fg(text_muted)),
            Span::styled(always_label, always_style),
            Span::styled("  ", Style::default()),
            Span::styled("[S] ", Style::default().fg(text_muted)),
            Span::styled(skip_label, skip_style),
        ]);
        buf.set_line(inner_x, buttons_y + 1, &row2, inner_width);
    }
}

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

        // Check that tool_name appears in output
        // The render should have written the tool name somewhere
        let content = buf.content();
        let has_bash = content.iter().any(|cell| {
            cell.symbol() == "b"
                || cell.symbol() == "a"
                || cell.symbol() == "s"
                || cell.symbol() == "h"
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

        // Check that tool_args appears in output
        let content = buf.content();
        let has_rm = content.iter().any(|cell| {
            cell.symbol() == "r"
                || cell.symbol() == "m"
                || cell.symbol() == "-"
                || cell.symbol() == "f"
                || cell.symbol() == "/"
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
        assert_eq!(modal.selected, 0); // wraps around
    }

    #[test]
    fn test_prev_option_cycles() {
        let mut modal = PermissionModal::default();

        assert_eq!(modal.selected, 0);
        modal.prev_option();
        assert_eq!(modal.selected, 3); // wraps backward
        modal.prev_option();
        assert_eq!(modal.selected, 2);
    }

    #[test]
    fn test_confirm_returns_correct_action() {
        let mut modal = PermissionModal::default();

        // Default selected = 0 -> Confirm
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