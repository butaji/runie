use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use crate::components::gradient_border::render_gradient_border;
use crate::theme::ThemeWrapper;

const MAX_FILE_LINES: usize = 500;

#[derive(Clone)]
pub struct DiffViewer {
    pub filename: String,
    pub old_content: String,
    pub new_content: String,
    pub visible: bool,
    pub scroll_offset: usize,
}

impl DiffViewer {
    pub fn new(filename: String, old: String, new: String) -> Self {
        Self {
            filename,
            old_content: old,
            new_content: new,
            visible: true,
            scroll_offset: 0,
        }
    }

    pub fn compute_diff(&self) -> Vec<DiffLine> {
        let old_lines: Vec<&str> = self.old_content.lines().collect();
        let new_lines: Vec<&str> = self.new_content.lines().collect();
        let max_lines = old_lines.len().max(new_lines.len()).min(MAX_FILE_LINES);
        let mut diff = Vec::new();

        for i in 0..max_lines {
            match (old_lines.get(i), new_lines.get(i)) {
                (Some(old), Some(new)) if old == new => {
                    diff.push(DiffLine::Context(old.to_string()));
                }
                (Some(old), Some(new)) => {
                    diff.push(DiffLine::Removed(old.to_string()));
                    diff.push(DiffLine::Added(new.to_string()));
                }
                (Some(old), None) => {
                    diff.push(DiffLine::Removed(old.to_string()));
                }
                (None, Some(new)) => {
                    diff.push(DiffLine::Added(new.to_string()));
                }
                (None, None) => break,
            }
        }
        diff
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset += 1;
    }

    pub fn render_ref(&self, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
        if !self.visible {
            return;
        }
        clear_area(area, buf, theme);
        render_border(area, buf, theme, &self.filename);
        render_title(area, buf, theme, &self.filename);
        render_diff_lines(area, buf, theme, &self.compute_diff(), self.scroll_offset);
        render_footer(area, buf, theme);
    }
}

fn clear_area(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    let bg_panel: Color = theme.color("bg.panel").into();
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, y)) { cell.set_style(Style::default().bg(bg_panel)); }
        }
    }
}

fn render_border(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper, filename: &str) {
    let accent: Color = theme.color("accent.primary").into();

    // Draw gradient border
    render_gradient_border(area, buf);

    // Draw title centered on top border row
    let title = format!(" Diff: {} ", filename);
    let title_len = title.len() as u16;
    let title_x = area.x + (area.width.saturating_sub(title_len)) / 2;
    let title_line = Line::from(vec![Span::styled(
        title.as_str(),
        Style::default().fg(accent).add_modifier(Modifier::BOLD),
    )]);
    buf.set_line(title_x, area.y, &title_line, title_len);
}

fn render_title(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper, filename: &str) {
    // Title is rendered by Block in render_border, no manual rendering needed
    let _ = (area, buf, theme, filename);
}

fn render_diff_lines(
    area: Rect,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
    diff: &[DiffLine],
    scroll_offset: usize,
) {
    let removed_color: Color = theme.color("diff.removed").into();
    let added_color: Color = theme.color("diff.added").into();
    let context_color: Color = theme.color("text.secondary").into();

    let content_start_y = area.y + 1;
    let max_visible = area.height.saturating_sub(2) as usize;

    for i in 0..max_visible {
        let line_idx = scroll_offset + i;
        if line_idx >= diff.len() {
            break;
        }
        let y = content_start_y + i as u16;
        if y >= area.y + area.height - 1 {
            break;
        }
        let line = &diff[line_idx];
        render_diff_line(buf, area, y, line, removed_color, added_color, context_color);
    }
}

fn render_diff_line(
    buf: &mut Buffer,
    area: Rect,
    y: u16,
    line: &DiffLine,
    removed_color: Color,
    added_color: Color,
    context_color: Color,
) {
    let (prefix, text, color) = match line {
        DiffLine::Removed(text) => ("-", text.as_str(), removed_color),
        DiffLine::Added(text) => ("+", text.as_str(), added_color),
        DiffLine::Context(text) => (" ", text.as_str(), context_color),
    };
    if let Some(cell) = buf.cell_mut((area.x + 1, y)) {
        cell.set_char(prefix.chars().next().unwrap_or(' '));
        cell.set_style(Style::default().fg(color));
    }
    let max_text_width = (area.width - 4) as usize;
    let display_text = if text.chars().count() > max_text_width {
        let truncated: String = text.chars().take(max_text_width).collect();
        truncated
    } else {
        text.to_string()
    };
    let content_span = Span::styled(format!(" {}", display_text), Style::default().fg(color));
    let line = Line::from(vec![content_span]);
    buf.set_line(area.x + 2, y, &line, area.width - 4);
}

fn render_footer(area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    let hint_color: Color = theme.color("text.dim").into();
    let hint = Line::from(vec![Span::styled(
        "[q] quit · [j/k] scroll",
        Style::default().fg(hint_color),
    )]);
    buf.set_line(area.x + 2, area.y + area.height - 1, &hint, area.width - 4);
}

#[derive(Debug, Clone, PartialEq)]
pub enum DiffLine {
    Removed(String),
    Added(String),
    Context(String),
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_viewer_new() {
        let diff = DiffViewer::new(
            "test.rs".to_string(),
            "hello".to_string(),
            "hello world".to_string(),
        );
        assert_eq!(diff.filename, "test.rs");
        assert!(diff.visible);
        assert_eq!(diff.scroll_offset, 0);
    }

    #[test]
    fn test_compute_diff_identical() {
        let diff = DiffViewer::new(
            "test.rs".to_string(),
            "hello\nworld".to_string(),
            "hello\nworld".to_string(),
        );
        let result = diff.compute_diff();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], DiffLine::Context("hello".to_string()));
        assert_eq!(result[1], DiffLine::Context("world".to_string()));
    }

    #[test]
    fn test_compute_diff_modified() {
        let diff = DiffViewer::new(
            "test.rs".to_string(),
            "hello".to_string(),
            "hello world".to_string(),
        );
        let result = diff.compute_diff();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], DiffLine::Removed("hello".to_string()));
        assert_eq!(result[1], DiffLine::Added("hello world".to_string()));
    }

    #[test]
    fn test_compute_diff_added_lines() {
        let diff = DiffViewer::new(
            "test.rs".to_string(),
            "line1".to_string(),
            "line1\nline2\nline3".to_string(),
        );
        let result = diff.compute_diff();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], DiffLine::Context("line1".to_string()));
        assert_eq!(result[1], DiffLine::Added("line2".to_string()));
        assert_eq!(result[2], DiffLine::Added("line3".to_string()));
    }

    #[test]
    fn test_compute_diff_removed_lines() {
        let diff = DiffViewer::new(
            "test.rs".to_string(),
            "line1\nline2\nline3".to_string(),
            "line1".to_string(),
        );
        let result = diff.compute_diff();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], DiffLine::Context("line1".to_string()));
        assert_eq!(result[1], DiffLine::Removed("line2".to_string()));
        assert_eq!(result[2], DiffLine::Removed("line3".to_string()));
    }

    #[test]
    fn test_scroll() {
        let mut diff = DiffViewer::new(
            "test.rs".to_string(),
            "1\n2\n3\n4\n5".to_string(),
            "1\n2\n3\n4\n5".to_string(),
        );
        diff.scroll_down();
        assert_eq!(diff.scroll_offset, 1);
        diff.scroll_up();
        assert_eq!(diff.scroll_offset, 0);
        diff.scroll_up();
        assert_eq!(diff.scroll_offset, 0);
    }
}