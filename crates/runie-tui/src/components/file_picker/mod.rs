//! File picker component — triggered by `@` prefix in input.
//!
//! Shows a searchable list of files in the current directory.
//! Up/Down navigate, Enter select, Esc close.

use std::path::PathBuf;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier},
    text::Line,
    widgets::Widget,
};
use crate::style::box_chars::H;
use crate::style::layout::{PADDING_X, PADDING_WIDTH};
use crate::style::StyleSet;
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

fn render_file_picker_frame(area: Rect, buf: &mut Buffer, styles: &StyleSet) -> Rect {
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            buf.get_mut(x, y).set_bg(styles.muted.bg.unwrap_or(Color::Black));
        }
    }
    let border_color = styles.border.fg.unwrap_or(Color::DarkGray);
    for x in area.left()..area.right() {
        if let Some(cell) = buf.cell_mut((x, area.top())) {
            cell.set_fg(border_color);
        }
        if let Some(cell) = buf.cell_mut((x, area.bottom().saturating_sub(1))) {
            cell.set_fg(border_color);
        }
    }
    for y in area.top()..area.bottom() {
        if let Some(cell) = buf.cell_mut((area.left(), y)) {
            cell.set_fg(border_color);
        }
        if let Some(cell) = buf.cell_mut((area.right().saturating_sub(1), y)) {
            cell.set_fg(border_color);
        }
    }
    Rect {
        x: area.x + PADDING_X,
        y: area.y + 1,
        width: area.width.saturating_sub(PADDING_WIDTH),
        height: area.height.saturating_sub(2),
    }
}

fn render_file_list(picker: &FilePicker, inner: Rect, buf: &mut Buffer, styles: &StyleSet) {
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
            styles.accent.bg(styles.muted.bg.unwrap_or(Color::Black)).add_modifier(Modifier::BOLD)
        } else {
            styles.text_primary
        };
        buf.set_line(inner.x, y, &Line::from(text).style(style), inner.width);
    }
}

impl Widget for &FilePicker {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = ThemeWrapper::default();
        let styles = StyleSet::from_theme(&theme);

        let inner = render_file_picker_frame(area, buf, &styles);
        render_picker_header(self, inner, buf, &styles);
        render_picker_filter(self, inner, buf, &styles);
        render_picker_divider(inner, buf, &styles);
        render_file_list(self, inner, buf, &styles);
        render_picker_empty(self, inner, buf, &styles);
    }
}

fn render_picker_header(picker: &FilePicker, inner: Rect, buf: &mut Buffer, styles: &StyleSet) {
    if inner.height == 0 { return; }
    let header = format!("@ {} ({} files)", picker.cwd.display(), picker.files.len());
    let header_line = Line::from(header).style(styles.accent.add_modifier(Modifier::BOLD));
    buf.set_line(inner.x, inner.y, &header_line, inner.width);
}

fn render_picker_filter(picker: &FilePicker, inner: Rect, buf: &mut Buffer, styles: &StyleSet) {
    if inner.height <= 1 { return; }
    let filter_text = if picker.filter.is_empty() { "Type to filter...".to_string() } else { format!("> {}", picker.filter) };
    let filter_line = Line::from(filter_text).style(styles.muted);
    buf.set_line(inner.x, inner.y + 1, &filter_line, inner.width);
}

fn render_picker_divider(inner: Rect, buf: &mut Buffer, styles: &StyleSet) {
    if inner.height <= 2 { return; }
    let divider: String = std::iter::repeat(H).take(inner.width as usize).collect();
    buf.set_string(inner.x, inner.y + 2, divider, styles.border);
}

fn render_picker_empty(picker: &FilePicker, inner: Rect, buf: &mut Buffer, styles: &StyleSet) {
    if !picker.filtered_indices.is_empty() || inner.height <= 3 { return; }
    let empty_text = if picker.filter.is_empty() { "No files in directory" } else { "No files match filter" };
    buf.set_line(inner.x, inner.y + 3, &Line::from(empty_text).style(styles.muted), inner.width);
}
