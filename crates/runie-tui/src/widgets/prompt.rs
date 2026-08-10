//! Prompt widget: append-only text input with actor-projected state.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Widget};
use runie_core::types::ThemeKind;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::appearance;
pub use runie_tui_model::{InputMode, PromptOutcome, PromptSnapshot};
#[derive(Clone)]
pub struct PromptWidget {
    buffer: String,
    focused: bool,
    /// Submitted prompts, newest last.
    history: Vec<String>,
    /// History index; `None` means fresh editing.
    history_index: Option<usize>,
    history_search: bool,
    mode: InputMode,
    model_caption: String,
    show_placeholder: bool,
    file_candidates: Vec<String>,
    file_candidate_index: usize,
    selected_file: Option<String>,
    viewer_lines: Vec<String>,
    theme: ThemeKind,
}
impl PromptWidget {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            focused: true,
            history: Vec::new(),
            history_index: None,
            history_search: false,
            mode: InputMode::Normal,
            model_caption: "Grok 4.5 (high)".into(),
            show_placeholder: true,
            file_candidates: Vec::new(),
            file_candidate_index: 0,
            selected_file: None,
            viewer_lines: Vec::new(),
            theme: ThemeKind::GrokNight,
        }
    }
    /// Rehydrate from one actor-owned prompt projection.
    pub fn from_model_snapshot(snapshot: PromptSnapshot) -> Self {
        Self {
            buffer: snapshot.text,
            focused: snapshot.focused,
            history: snapshot.history,
            history_index: snapshot.history_index,
            history_search: snapshot.history_search,
            mode: snapshot.mode,
            model_caption: snapshot.model_caption,
            show_placeholder: snapshot.show_placeholder,
            file_candidates: snapshot.file_candidates,
            file_candidate_index: snapshot.file_candidate_index,
            selected_file: snapshot.selected_file,
            viewer_lines: snapshot.viewer_lines,
            theme: snapshot.theme,
        }
    }
    pub fn history(&self) -> &[String] {
        &self.history
    }
    /// Record a deduplicated submitted prompt.
    pub fn push_history(&mut self, text: &str) {
        let trimmed = text.trim().to_string();
        if !trimmed.is_empty() && self.history.last() != Some(&trimmed) {
            self.history.push(trimmed);
        }
        self.history_index = None;
        self.history_search = false;
    }
    pub fn text(&self) -> String {
        self.buffer.clone()
    }

    pub fn model_snapshot(&self) -> PromptSnapshot {
        PromptSnapshot {
            text: self.buffer.clone(),
            focused: self.focused,
            history: self.history.clone(),
            history_index: self.history_index,
            history_search: self.history_search,
            mode: self.mode,
            model_caption: self.model_caption.clone(),
            show_placeholder: self.show_placeholder,
            file_candidates: self.file_candidates.clone(),
            file_candidate_index: self.file_candidate_index,
            selected_file: self.selected_file.clone(),
            viewer_lines: self.viewer_lines.clone(),
            theme: self.theme,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.trim().is_empty()
    }

    pub fn render_height(&self) -> u16 {
        if self.file_viewer_active() {
            return self.viewer_lines.len().saturating_add(2) as u16;
        }
        let prompt_lines = self.buffer.lines().count();
        let candidate_lines = if self.file_search_active() {
            self.file_matches().len().min(5)
        } else {
            0
        };
        prompt_lines
            .saturating_add(candidate_lines)
            .saturating_add(2) as u16
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    pub fn focused(&self) -> bool {
        self.focused
    }

    pub fn set_focused(&mut self, on: bool) {
        self.focused = on;
    }

    pub fn mode(&self) -> InputMode {
        self.mode
    }

    pub fn cycle_mode(&mut self) {
        self.mode = runie_tui_model::cycle_input_mode(self.mode);
    }

    /// Apply an actor-owned asynchronous file-search result.
    pub async fn open_file_search_async(&mut self) {
        if let Some(path) = self.selected_file.clone() {
            self.viewer_lines = match tokio::fs::read_to_string(&path).await {
                Ok(contents) => contents.lines().take(20).map(str::to_owned).collect(),
                Err(error) => vec![format!("unable to read {path}: {error}")],
            };
            self.mode = InputMode::FileViewer;
            return;
        }
        self.mode = InputMode::FileSearch;
        self.buffer.clear();
        self.file_candidate_index = 0;
        self.file_candidates.clear();
        if let Ok(mut entries) = tokio::fs::read_dir(".").await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Ok(name) = entry.file_name().into_string() {
                    if !name.starts_with('.') {
                        self.file_candidates.push(name);
                    }
                }
            }
        }
        self.file_candidates.sort();
    }

    pub fn file_search_active(&self) -> bool {
        self.mode == InputMode::FileSearch
    }

    pub fn file_viewer_active(&self) -> bool {
        self.mode == InputMode::FileViewer
    }

    pub fn set_model_caption(&mut self, caption: impl Into<String>) {
        let caption = caption.into();
        if !caption.trim().is_empty() {
            self.model_caption = caption;
        }
    }

    pub fn set_placeholder_visible(&mut self, visible: bool) {
        self.show_placeholder = visible;
    }

    pub fn set_theme(&mut self, theme: ThemeKind) {
        self.theme = theme;
    }

    pub fn history_search_active(&self) -> bool {
        self.history_search
    }

    /// Return the cursor position; the prompt prefix occupies two columns.
    pub fn cursor_position(&self, area: Rect) -> ratatui::layout::Position {
        let width = area.width.saturating_sub(5).max(1) as usize;
        let mut column = 3usize;
        for ch in self.buffer.chars() {
            if ch == '\n' {
                continue;
            }
            let char_width = UnicodeWidthChar::width(ch).unwrap_or(0);
            column += char_width;
        }
        ratatui::layout::Position::new(area.x + 1 + column.min(width + 2) as u16, area.y + 1)
    }
    pub fn handle_key(&mut self, key: KeyEvent) -> PromptOutcome {
        if self.file_search_active() || self.file_viewer_active() {
            if let Some(outcome) = self.handle_file_search_key(key) {
                return outcome;
            }
        }
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.clear_prompt()
            }
            KeyCode::Enter if key.modifiers == KeyModifiers::NONE => self.submit_prompt(),
            // Shift/Alt-Enter inserts a newline.
            KeyCode::Enter
                if key.modifiers.contains(KeyModifiers::SHIFT)
                    || key.modifiers.contains(KeyModifiers::ALT) =>
            {
                self.buffer.push('\n');
                PromptOutcome::Edited
            }
            // Up goes back; Down goes forward.
            KeyCode::Up => self.history_up(),
            KeyCode::Down => self.history_down(),
            KeyCode::Backspace => {
                if !self.buffer.is_empty() {
                    self.buffer.pop();
                    PromptOutcome::Edited
                } else {
                    PromptOutcome::Ignored
                }
            }
            KeyCode::Esc => {
                self.history_search = false;
                self.history_index = None;
                PromptOutcome::Ignored
            }
            KeyCode::Char(c) => self.insert_char(c),
            _ => PromptOutcome::Ignored,
        }
    }

    fn handle_file_search_key(&mut self, key: KeyEvent) -> Option<PromptOutcome> {
        if self.file_viewer_active() {
            if key.code == KeyCode::Esc && key.modifiers == KeyModifiers::NONE {
                self.mode = InputMode::Normal;
                self.viewer_lines.clear();
                return Some(PromptOutcome::Edited);
            }
            return Some(PromptOutcome::Ignored);
        }
        if key.modifiers != KeyModifiers::NONE {
            return None;
        }
        let outcome = match key.code {
            KeyCode::Tab | KeyCode::Enter => self.accept_file_candidate(),
            KeyCode::Up => self.file_search_move(-1),
            KeyCode::Down => self.file_search_move(1),
            KeyCode::Esc => {
                self.mode = InputMode::Normal;
                PromptOutcome::Edited
            }
            _ => return None,
        };
        Some(outcome)
    }

    fn clear_prompt(&mut self) -> PromptOutcome {
        if self.buffer.is_empty() {
            return PromptOutcome::Ignored;
        }
        self.buffer.clear();
        self.history_index = None;
        PromptOutcome::Edited
    }

    fn submit_prompt(&mut self) -> PromptOutcome {
        if self.buffer.trim().is_empty() {
            return PromptOutcome::Ignored;
        }
        let text = std::mem::take(&mut self.buffer);
        self.push_history(&text);
        PromptOutcome::Submitted(text)
    }

    fn history_up(&mut self) -> PromptOutcome {
        if self.history.is_empty() {
            return PromptOutcome::Ignored;
        }
        if self.history_search {
            if let Some((idx, value)) = self
                .history
                .iter()
                .enumerate()
                .rev()
                .find(|(_, value)| value.contains(&self.buffer))
            {
                self.history_index = Some(idx);
                self.buffer = value.clone();
                return PromptOutcome::Edited;
            }
            return PromptOutcome::Ignored;
        }
        let idx = self
            .history_index
            .map_or(self.history.len().saturating_sub(1), |i| {
                i.saturating_sub(1)
            });
        self.history_index = Some(idx);
        self.buffer = self.history[idx].clone();
        PromptOutcome::Edited
    }

    fn history_down(&mut self) -> PromptOutcome {
        let Some(index) = self.history_index else {
            return PromptOutcome::Ignored;
        };
        if index + 1 < self.history.len() {
            self.history_index = Some(index + 1);
            self.buffer = self.history[index + 1].clone();
        } else {
            self.history_index = None;
            self.buffer.clear();
        }
        PromptOutcome::Edited
    }

    fn insert_char(&mut self, c: char) -> PromptOutcome {
        self.buffer.push(c);
        if self.buffer == "/history" {
            self.buffer.clear();
            self.history_search = true;
        }
        PromptOutcome::Edited
    }

    fn file_search_move(&mut self, delta: isize) -> PromptOutcome {
        let matches = self.file_matches();
        if matches.is_empty() {
            return PromptOutcome::Ignored;
        }
        self.file_candidate_index =
            runie_tui_model::wrap_dialog_selection(self.file_candidate_index, delta, matches.len());
        PromptOutcome::Edited
    }

    fn accept_file_candidate(&mut self) -> PromptOutcome {
        let matches = self.file_matches();
        let Some(candidate) = matches
            .get(self.file_candidate_index)
            .map(|candidate| (*candidate).clone())
        else {
            return PromptOutcome::Ignored;
        };
        self.buffer = candidate;
        self.selected_file = Some(self.buffer.clone());
        self.mode = InputMode::Normal;
        self.file_candidate_index = 0;
        PromptOutcome::Edited
    }

    fn file_matches(&self) -> Vec<&String> {
        self.file_candidates
            .iter()
            .filter(|candidate| candidate.contains(&self.buffer))
            .collect()
    }
}

impl Widget for PromptWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 2 || area.height < 1 {
            return;
        }
        let border = if self.mode == InputMode::Plan {
            appearance::warning_style_for(self.theme)
        } else {
            appearance::prompt_border_style_for(self.theme)
        };
        draw_prompt_border(area, buf, border);
        let bottom = area.y + area.height.saturating_sub(1);
        let right = area.x + area.width.saturating_sub(1);
        self.draw_caption(area, bottom, right, buf);
        let input_area = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2).max(1),
        };
        for y in input_area.y..input_area.y.saturating_add(input_area.height) {
            for x in input_area.x..input_area.x.saturating_add(input_area.width) {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_style(appearance::base_style_for(self.theme));
                }
            }
        }
        Widget::render(Paragraph::new(self.input_lines()), input_area, buf);
    }
}

impl PromptWidget {
    fn draw_caption(&self, area: Rect, bottom: u16, right: u16, buf: &mut Buffer) {
        let mode_caption = match self.mode {
            InputMode::Normal => self.model_caption.clone(),
            InputMode::Alternate => format!("alternate · {}", self.model_caption),
            InputMode::Plan => format!("plan · {}", self.model_caption),
            InputMode::FileSearch => format!("file search · {}", self.model_caption),
            InputMode::FileViewer => format!("file viewer · {}", self.model_caption),
        };
        let caption = if self.history_search {
            format!("history search · {mode_caption}")
        } else if self.history_index.is_some() {
            format!("history · {mode_caption}")
        } else if self.buffer.contains('\n') {
            format!("multiline · {mode_caption}")
        } else {
            mode_caption.to_string()
        };
        let caption_width = UnicodeWidthStr::width(caption.as_str()) as u16 + 2;
        if caption_width + 2 < area.width {
            let caption_x = right.saturating_sub(caption_width + 1);
            let spans = caption_spans(&caption, self.theme);
            buf.set_line(caption_x, bottom, &Line::from(spans), caption_width);
        }
    }

    fn input_lines(&self) -> Vec<Line<'static>> {
        let glyph = " ❯ ";
        if self.buffer.is_empty() && self.show_placeholder {
            return vec![Line::from(vec![
                ratatui::text::Span::styled(glyph, appearance::base_style_for(self.theme)),
                ratatui::text::Span::styled(
                    "Type your message...",
                    appearance::muted_style_for(self.theme),
                ),
            ])];
        }
        let mut lines: Vec<Line<'static>> = self
            .buffer
            .split('\n')
            .enumerate()
            .map(|(index, text)| {
                let prefix = if index == 0 {
                    glyph.to_owned()
                } else {
                    "   ".to_owned()
                };
                Line::from(vec![
                    ratatui::text::Span::styled(prefix, appearance::base_style_for(self.theme)),
                    ratatui::text::Span::styled(
                        text.to_owned(),
                        appearance::base_style_for(self.theme),
                    ),
                ])
            })
            .collect();
        self.append_file_search_lines(&mut lines);
        lines
    }

    fn append_file_search_lines(&self, lines: &mut Vec<Line<'static>>) {
        if self.file_viewer_active() {
            *lines = self
                .viewer_lines
                .iter()
                .map(|line| Line::from(line.clone()))
                .collect();
            return;
        }
        if self.file_search_active() {
            for (index, candidate) in self.file_matches().into_iter().take(5).enumerate() {
                let marker = if index == self.file_candidate_index {
                    "› "
                } else {
                    "  "
                };
                lines.push(Line::from(format!("{marker}{candidate}")));
            }
        }
    }
}

#[path = "prompt_render.rs"]
mod prompt_render;
use prompt_render::{caption_spans, draw_prompt_border};
impl Default for PromptWidget {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "prompt_tests.rs"]
mod tests;
