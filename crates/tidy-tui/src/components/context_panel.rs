use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};
use crate::theme::ThemeWrapper;

#[derive(Clone)]
pub struct ContextPanel {
    pub recent_files: Vec<String>,
    pub git_changes: Vec<GitChange>,
    pub active_tool: Option<String>,
    pub model_name: String,
    pub session_info: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GitStatus {
    Modified,
    Added,
    Deleted,
    Untracked,
}

#[derive(Debug, Clone)]
pub struct GitChange {
    pub path: String,
    pub status: GitStatus,
}

impl Default for ContextPanel {
    fn default() -> Self {
        Self {
            recent_files: vec![],
            git_changes: vec![],
            active_tool: None,
            model_name: "claude-4".to_string(),
            session_info: "new session".to_string(),
        }
    }
}

impl ContextPanel {
    /// Render the context panel to a buffer
    pub fn render_ref(&self, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
        // Fill background with panel color
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
        let warning: ratatui::style::Color = theme.color("warning").into();
        let success: ratatui::style::Color = theme.color("success").into();
        let error: ratatui::style::Color = theme.color("error").into();
        let border_unfocused: ratatui::style::Color = theme.color("border.unfocused").into();

        let left_margin = 1u16;
        let max_width = area.width.saturating_sub(left_margin + 1);
        let mut y = area.y;

        // Section 1: Model
        if y < area.y + area.height {
            let model_label = Span::styled("Model: ", Style::default().fg(text_muted));
            let model_name = Span::styled(&self.model_name, Style::default().fg(accent_secondary));
            let line = Line::from(vec![model_label, model_name]);
            buf.set_line(area.x + left_margin, y, &line, max_width);
            y += 1;
        }

        // Section 2: Session
        if y < area.y + area.height {
            let session_label = Span::styled("Session: ", Style::default().fg(text_muted));
            let session_info = Span::styled(&self.session_info, Style::default().fg(text_secondary));
            let line = Line::from(vec![session_label, session_info]);
            buf.set_line(area.x + left_margin, y, &line, max_width);
            y += 1;
        }

        // Section 3: Active Tool (if any)
        if let Some(ref tool) = self.active_tool {
            if y < area.y + area.height {
                let tool_span = Span::styled(
                    format!("● {}", tool),
                    Style::default().fg(warning),
                );
                let line = Line::from(vec![tool_span]);
                buf.set_line(area.x + left_margin, y, &line, max_width);
                y += 1;
            }
        }

        // Separator line
        if y < area.y + area.height {
            let sep = Span::styled(
                "─".repeat(max_width as usize),
                Style::default().fg(border_unfocused),
            );
            let line = Line::from(vec![sep]);
            buf.set_line(area.x + left_margin, y, &line, max_width);
            y += 1;
        }

        // Section 4: Recent Files
        if !self.recent_files.is_empty() && y < area.y + area.height {
            // Header
            let header = Span::styled(
                "RECENT",
                Style::default().fg(accent_primary).add_modifier(Modifier::BOLD),
            );
            let line = Line::from(vec![header]);
            buf.set_line(area.x + left_margin, y, &line, max_width);
            y += 1;

            // File list
            for file in &self.recent_files {
                if y >= area.y + area.height {
                    break;
                }
                let display_name = if file.len() > (max_width as usize - 2) {
                    let mut truncated = file.clone();
                    truncated.truncate(max_width as usize - 5);
                    truncated.push_str("...");
                    truncated
                } else {
                    file.clone()
                };
                let file_span = Span::styled(
                    format!("▸ {}", display_name),
                    Style::default().fg(text_secondary),
                );
                let line = Line::from(vec![file_span]);
                buf.set_line(area.x + left_margin, y, &line, max_width);
                y += 1;
            }
        }

        // Section 5: Git Changes
        if !self.git_changes.is_empty() {
            // Separator line before section
            if y < area.y + area.height {
                let sep = Span::styled(
                    "─".repeat(max_width as usize),
                    Style::default().fg(border_unfocused),
                );
                let line = Line::from(vec![sep]);
                buf.set_line(area.x + left_margin, y, &line, max_width);
                y += 1;
            }

            if y < area.y + area.height {
                // Header
                let header = Span::styled(
                    "CHANGES",
                    Style::default().fg(accent_primary).add_modifier(Modifier::BOLD),
                );
                let line = Line::from(vec![header]);
                buf.set_line(area.x + left_margin, y, &line, max_width);
                y += 1;

                // Change list
                for change in &self.git_changes {
                    if y >= area.y + area.height {
                        break;
                    }
                    let (symbol, color) = match change.status {
                        GitStatus::Modified => ("~", warning),
                        GitStatus::Added => ("+", success),
                        GitStatus::Deleted => ("-", error),
                        GitStatus::Untracked => ("?", text_muted),
                    };
                    let display_path = if change.path.len() > (max_width as usize - 3) {
                        let mut truncated = change.path.clone();
                        truncated.truncate(max_width as usize - 6);
                        truncated.push_str("...");
                        truncated
                    } else {
                        change.path.clone()
                    };
                    let change_span = Span::styled(
                        format!("{} {}", symbol, display_path),
                        Style::default().fg(color),
                    );
                    let line = Line::from(vec![change_span]);
                    buf.set_line(area.x + left_margin, y, &line, max_width);
                    y += 1;
                }
            }
        }
    }
}

impl Widget for ContextPanel {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = ThemeWrapper::default();
        self.render_ref(area, buf, &theme);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    fn create_test_theme() -> ThemeWrapper {
        ThemeWrapper::default()
    }

    #[test]
    fn test_render_shows_model_name() {
        let panel = ContextPanel {
            model_name: "claude-5".to_string(),
            ..Default::default()
        };
        let area = Rect::new(0, 0, 28, 20);
        let mut buf = Buffer::empty(area);
        let theme = create_test_theme();

        panel.render_ref(area, &mut buf, &theme);

        // Check that "Model: " label is present at position (1, 0)
        // The model name follows immediately after
        assert_eq!(buf.cell((1, 0)).unwrap().symbol(), "M");
        assert_eq!(buf.cell((2, 0)).unwrap().symbol(), "o");
        assert_eq!(buf.cell((3, 0)).unwrap().symbol(), "d");
        assert_eq!(buf.cell((4, 0)).unwrap().symbol(), "e");
        assert_eq!(buf.cell((5, 0)).unwrap().symbol(), "l");
        assert_eq!(buf.cell((6, 0)).unwrap().symbol(), ":");
        assert_eq!(buf.cell((7, 0)).unwrap().symbol(), " ");
    }

    #[test]
    fn test_render_shows_git_changes() {
        let panel = ContextPanel {
            git_changes: vec![
                GitChange { path: "main.rs".to_string(), status: GitStatus::Modified },
                GitChange { path: "Cargo.toml".to_string(), status: GitStatus::Added },
                GitChange { path: "old.rs".to_string(), status: GitStatus::Deleted },
                GitChange { path: "new.rs".to_string(), status: GitStatus::Untracked },
            ],
            ..Default::default()
        };
        let area = Rect::new(0, 0, 28, 20);
        let mut buf = Buffer::empty(area);
        let theme = create_test_theme();

        panel.render_ref(area, &mut buf, &theme);

        // Collect all cell symbols to verify content
        let mut content = String::new();
        for y in 0..area.height {
            for x in 0..area.width {
                if let Some(cell) = buf.cell((x, y)) {
                    content.push_str(cell.symbol());
                }
            }
            content.push('\n');
        }

        // Check for change symbols
        assert!(content.contains("~"), "Modified files should show ~");
        assert!(content.contains("+"), "Added files should show +");
        assert!(content.contains("-"), "Deleted files should show -");
        assert!(content.contains("?"), "Untracked files should show ?");
    }

    #[test]
    fn test_render_shows_recent_files() {
        let panel = ContextPanel {
            recent_files: vec![
                "lib.rs".to_string(),
                "main.rs".to_string(),
            ],
            ..Default::default()
        };
        let area = Rect::new(0, 0, 28, 20);
        let mut buf = Buffer::empty(area);
        let theme = create_test_theme();

        panel.render_ref(area, &mut buf, &theme);

        // Collect cell symbols to verify content
        let mut content = String::new();
        for y in 0..area.height {
            for x in 0..area.width {
                if let Some(cell) = buf.cell((x, y)) {
                    content.push_str(cell.symbol());
                }
            }
            content.push('\n');
        }

        // Check for RECENT header and file entries
        assert!(content.contains("RECENT"), "RECENT header should appear");
        assert!(content.contains("▸"), "File indicator should appear");
        assert!(content.contains("lib.rs"), "lib.rs should appear");
        assert!(content.contains("main.rs"), "main.rs should appear");
    }
}
