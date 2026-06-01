//! File picker component — triggered by `@` prefix in input.
//!
//! Shows a searchable list of files in the current directory.
//! Up/Down navigate, Enter select, Esc close.

use std::path::PathBuf;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::Widget,
};
use crate::theme::ThemeWrapper;

/// File picker state.
#[derive(Debug, Clone, Default)]
pub struct FilePicker {
    pub open: bool,
    pub filter: String,
    pub selected: usize,
    pub files: Vec<PathBuf>,
    pub filtered_indices: Vec<usize>,
    pub cwd: PathBuf,
}

impl FilePicker {
    pub fn new() -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            open: false,
            filter: String::new(),
            selected: 0,
            files: Vec::new(),
            filtered_indices: Vec::new(),
            cwd,
        }
    }

    /// Open file picker and scan current directory.
    pub fn open(&mut self) {
        self.open = true;
        self.filter.clear();
        self.selected = 0;
        self.scan_files();
        self.update_filtered();
    }

    pub fn close(&mut self) {
        self.open = false;
        self.filter.clear();
        self.selected = 0;
        self.files.clear();
        self.filtered_indices.clear();
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Append character to filter.
    pub fn push_filter(&mut self, ch: char) {
        self.filter.push(ch);
        self.update_filtered();
    }

    /// Remove last character from filter.
    pub fn pop_filter(&mut self) {
        self.filter.pop();
        self.update_filtered();
    }

    /// Move selection up.
    pub fn move_up(&mut self) {
        if self.filtered_indices.len() > 1 {
            self.selected = self.selected.saturating_sub(1);
        }
    }

    /// Move selection down.
    pub fn move_down(&mut self) {
        if self.selected + 1 < self.filtered_indices.len() {
            self.selected += 1;
        }
    }

    /// Return the selected file path, if any.
    pub fn selected_file(&self) -> Option<String> {
        self.filtered_indices.get(self.selected).map(|&idx| {
            self.files[idx].to_string_lossy().to_string()
        })
    }

    fn scan_files(&mut self) {
        self.files.clear();
        if let Ok(entries) = std::fs::read_dir(&self.cwd) {
            let mut paths: Vec<PathBuf> = entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .collect();
            paths.sort();
            self.files = paths;
        }
    }

    pub fn update_filtered(&mut self) {
        let query = self.filter.to_lowercase();
        self.filtered_indices = self.files
            .iter()
            .enumerate()
            .filter(|(_, path)| {
                path.to_string_lossy().to_lowercase().contains(&query)
            })
            .map(|(i, _)| i)
            .collect();
        if self.selected >= self.filtered_indices.len() {
            self.selected = 0;
        }
    }
}

fn render_file_picker_frame(area: Rect, buf: &mut Buffer, bg: Color, border: Color) -> Rect {
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            buf.get_mut(x, y).set_bg(bg);
        }
    }
    for x in area.left()..area.right() {
        buf.get_mut(x, area.top()).set_fg(border);
        buf.get_mut(x, area.bottom().saturating_sub(1)).set_fg(border);
    }
    for y in area.top()..area.bottom() {
        buf.get_mut(area.left(), y).set_fg(border);
        buf.get_mut(area.right().saturating_sub(1), y).set_fg(border);
    }
    Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    }
}

fn render_file_list(picker: &FilePicker, inner: Rect, buf: &mut Buffer, accent: Color, highlight_bg: Color, text_primary: Color) {
    let list_start = inner.y + 3;
    let visible_count = inner.height.saturating_sub(3) as usize;
    for (i, &file_idx) in picker.filtered_indices.iter().enumerate().take(visible_count) {
        let y = list_start + i as u16;
        if y >= inner.y + inner.height { break; }
        let path = &picker.files[file_idx];
        let is_dir = path.is_dir();
        let name = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        let icon = if is_dir { "📁" } else { "📄" };
        let text = format!("{} {}", icon, name);
        let style = if i == picker.selected {
            Style::default().fg(accent).bg(highlight_bg).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(text_primary)
        };
        buf.set_line(inner.x, y, &Line::from(text).style(style), inner.width);
    }
}

impl Widget for &FilePicker {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let bg = Color::Black;
        let border = Color::DarkGray;
        let text_primary = Color::White;
        let text_secondary = Color::Gray;
        let accent = Color::Cyan;
        let highlight_bg = Color::Rgb(40, 40, 40);

        let inner = render_file_picker_frame(area, buf, bg, border);

        let header = format!("@ {} ({} files)", self.cwd.display(), self.files.len());
        let header_line = Line::from(header).style(Style::default().fg(accent).add_modifier(Modifier::BOLD));
        if inner.height > 0 { buf.set_line(inner.x, inner.y, &header_line, inner.width); }

        let filter_text = if self.filter.is_empty() { "Type to filter...".to_string() } else { format!("> {}", self.filter) };
        let filter_line = Line::from(filter_text).style(Style::default().fg(text_secondary));
        if inner.height > 1 { buf.set_line(inner.x, inner.y + 1, &filter_line, inner.width); }

        if inner.height > 2 {
            let divider = "─".repeat(inner.width as usize);
            buf.set_string(inner.x, inner.y + 2, divider, Style::default().fg(border));
        }

        render_file_list(self, inner, buf, accent, highlight_bg, text_primary);

        if self.filtered_indices.is_empty() && inner.height > 3 {
            let empty_text = if self.filter.is_empty() { "No files in directory" } else { "No files match filter" };
            buf.set_line(inner.x, inner.y + 3, &Line::from(empty_text).style(Style::default().fg(text_secondary)), inner.width);
        }
    }
}
