//! Prompt widget: simple text input with no external crate dependency.
//!
//! Avoids `tui-textarea` because its render path panics on certain
//! invariant states (empty buffers, no cursor-sync integration). The
//! trade-off: no advanced editing features (cursor movement, selection),
//! but the prompt is append-only + Enter-submits which is all this
//! minimal TUI needs.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Widget};
use runie_core::types::ThemeKind;
use unicode_width::UnicodeWidthStr;

use crate::appearance;
pub use runie_tui_model::{InputMode, PromptOutcome, PromptSnapshot};

#[derive(Clone)]
pub struct PromptWidget {
    buffer: String,
    focused: bool,
    /// Submitted prompts, newest last (grok prompt history).
    history: Vec<String>,
    /// Index into `history` during Up/Down recall; `None` = editing fresh.
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

    /// Rehydrate the terminal widget from one actor-owned prompt projection.
    /// The adapter is renderer-local; the projection remains the sole source
    /// of prompt facts for the frame being painted.
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

    /// Record a submitted prompt into the history (deduped, newest last).
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
        self.mode = match self.mode {
            InputMode::Normal => InputMode::Alternate,
            InputMode::Alternate => InputMode::Plan,
            InputMode::Plan => InputMode::Normal,
            InputMode::FileSearch => InputMode::FileSearch,
            InputMode::FileViewer => InputMode::FileViewer,
        };
    }

    pub fn open_file_search(&mut self) {
        if let Some(path) = self.selected_file.clone() {
            self.viewer_lines = std::fs::read_to_string(&path)
                .map(|contents| contents.lines().take(20).map(str::to_owned).collect())
                .unwrap_or_else(|error| vec![format!("unable to read {path}: {error}")]);
            self.mode = InputMode::FileViewer;
            return;
        }
        self.mode = InputMode::FileSearch;
        self.buffer.clear();
        self.file_candidate_index = 0;
        self.file_candidates = std::fs::read_dir(".")
            .ok()
            .into_iter()
            .flat_map(|entries| entries.flatten())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| !name.starts_with('.'))
            .collect();
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

    /// Return the terminal cursor position for the current prompt text.
    /// The prompt prefix occupies two terminal columns (`❯ `).
    pub fn cursor_position(&self, area: Rect) -> ratatui::layout::Position {
        let width = area.width.saturating_sub(5).max(1) as usize;
        let column = 3 + UnicodeWidthStr::width(self.buffer.as_str());
        ratatui::layout::Position::new(
            area.x + 1 + (column % width) as u16,
            area.y + 1 + (column / width).min(area.height.saturating_sub(3) as usize) as u16,
        )
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
            // Multiline: Shift/Alt-Enter inserts a newline (grok prompt_widget).
            KeyCode::Enter
                if key.modifiers.contains(KeyModifiers::SHIFT)
                    || key.modifiers.contains(KeyModifiers::ALT) =>
            {
                self.buffer.push('\n');
                PromptOutcome::Edited
            }
            // History recall: Up goes back, Down goes forward (grok /history).
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
        let last = matches.len().saturating_sub(1);
        self.file_candidate_index = if delta.is_negative() {
            self.file_candidate_index
                .saturating_sub(delta.unsigned_abs())
        } else {
            self.file_candidate_index
                .saturating_add(delta as usize)
                .min(last)
        };
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
            appearance::muted_style_for(self.theme)
        };
        draw_prompt_border(area, buf, border);
        let bottom = area.y + area.height.saturating_sub(1);
        let right = area.x + area.width.saturating_sub(1);
        self.draw_caption(area, bottom, right, border, buf);
        let input_area = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2).max(1),
        };
        Widget::render(Paragraph::new(self.input_lines()), input_area, buf);
    }
}

impl PromptWidget {
    fn draw_caption(&self, area: Rect, bottom: u16, right: u16, border: Style, buf: &mut Buffer) {
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
            buf.set_string(caption_x, bottom, format!(" {caption} "), border);
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

fn draw_prompt_border(area: Rect, buf: &mut Buffer, border: Style) {
    let top = area.y;
    let bottom = area.y + area.height.saturating_sub(1);
    let right = area.x + area.width.saturating_sub(1);
    for x in area.x..area.x + area.width {
        set_border_cell(
            buf,
            x,
            top,
            if x == area.x {
                '╭'
            } else if x == right {
                '╮'
            } else {
                '─'
            },
            border,
        );
        if bottom != top {
            set_border_cell(
                buf,
                x,
                bottom,
                if x == area.x {
                    '╰'
                } else if x == right {
                    '╯'
                } else {
                    '─'
                },
                border,
            );
        }
    }
    for y in top.saturating_add(1)..bottom {
        set_border_cell(buf, area.x, y, '│', border);
        set_border_cell(buf, right, y, '│', border);
    }
}

fn set_border_cell(buf: &mut Buffer, x: u16, y: u16, character: char, style: Style) {
    if let Some(cell) = buf.cell_mut((x, y)) {
        cell.set_char(character);
        cell.set_style(style);
    }
}

impl Default for PromptWidget {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use ratatui::style::Color;

    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

    fn key(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: mods,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        }
    }

    #[test]
    fn empty_enter_is_ignored() {
        let mut p = PromptWidget::new();
        let out = p.handle_key(key(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(out, PromptOutcome::Ignored);
    }

    #[test]
    fn ctrl_c_clears_non_empty_prompt() {
        let mut p = PromptWidget::new();
        p.handle_key(key(KeyCode::Char('x'), KeyModifiers::NONE));
        assert_eq!(
            p.handle_key(key(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            PromptOutcome::Edited
        );
        assert!(p.is_empty());
    }

    #[test]
    fn mode_cycles_through_normal_alternate_and_plan() {
        let mut p = PromptWidget::new();
        assert_eq!(p.mode(), InputMode::Normal);
        p.cycle_mode();
        assert_eq!(p.mode(), InputMode::Alternate);
        p.cycle_mode();
        assert_eq!(p.mode(), InputMode::Plan);
        p.cycle_mode();
        assert_eq!(p.mode(), InputMode::Normal);
    }

    #[test]
    fn plan_mode_uses_gold_prompt_border_and_caption() {
        let mut p = PromptWidget::new();
        p.cycle_mode();
        p.cycle_mode();
        let mut buffer = Buffer::empty(Rect::new(0, 0, 40, 4));
        p.clone().render(Rect::new(0, 0, 40, 4), &mut buffer);
        assert_eq!(
            buffer.cell((0, 0)).expect("top border").fg,
            appearance::warning_style().fg.expect("warning token")
        );
        let text: String = (0..40)
            .map(|x| buffer.cell((x, 3)).expect("caption row").symbol())
            .collect();
        assert!(text.contains("plan"));
    }

    #[test]
    fn file_search_mode_is_owned_by_prompt_and_esc_exits_it() {
        let mut p = PromptWidget::new();
        p.open_file_search();
        assert!(p.file_search_active());
        assert_eq!(
            p.handle_key(key(KeyCode::Esc, KeyModifiers::NONE)),
            PromptOutcome::Edited
        );
        assert!(!p.file_search_active());
    }

    #[test]
    fn file_search_accepts_a_selected_candidate() {
        let mut p = PromptWidget::new();
        p.open_file_search();
        assert!(!p.file_candidates.is_empty());
        let expected = p.file_matches()[0].clone();
        assert_eq!(
            p.handle_key(key(KeyCode::Tab, KeyModifiers::NONE)),
            PromptOutcome::Edited
        );
        assert_eq!(p.text(), expected);
        assert!(!p.file_search_active());
    }

    #[test]
    fn file_search_hands_selected_file_to_bounded_viewer() {
        let mut p = PromptWidget::new();
        p.selected_file = Some("Cargo.toml".into());
        p.open_file_search();
        assert!(p.file_viewer_active());
        assert!(!p.viewer_lines.is_empty());
        assert!(p.render_height() >= 2);
        assert_eq!(
            p.handle_key(key(KeyCode::Esc, KeyModifiers::NONE)),
            PromptOutcome::Edited
        );
        assert!(!p.file_viewer_active());
    }

    #[test]
    fn multiline_chrome_is_visible() {
        let mut p = PromptWidget::new();
        p.handle_key(key(KeyCode::Enter, KeyModifiers::SHIFT));
        let area = Rect::new(0, 0, 60, 4);
        let mut buffer = Buffer::empty(area);
        Widget::render(p, area, &mut buffer);
        let row = (0..area.width)
            .map(|x| buffer.cell((x, 3)).expect("caption cell").symbol())
            .collect::<String>();
        assert!(row.contains("multiline"));
    }

    #[test]
    fn model_caption_is_read_only_projection_input() {
        let mut p = PromptWidget::new();
        p.set_model_caption("test-model (high)");
        let area = Rect::new(0, 0, 60, 4);
        let mut buffer = Buffer::empty(area);
        Widget::render(p, area, &mut buffer);
        let row = (0..area.width)
            .map(|x| buffer.cell((x, 3)).expect("caption cell").symbol())
            .collect::<String>();
        assert!(row.contains("test-model (high)"));
    }

    #[test]
    fn renderer_adapter_preserves_prompt_projection_fields() {
        let mut source = PromptWidget::new();
        source.set_model_caption("adapter-model");
        source.handle_key(key(KeyCode::Char('x'), KeyModifiers::NONE));
        source.cycle_mode();
        source.push_history("previous");
        let snapshot = source.model_snapshot();
        let adapted = PromptWidget::from_model_snapshot(snapshot.clone());
        assert_eq!(adapted.model_snapshot(), snapshot);
    }

    #[test]
    fn history_chrome_is_visible_while_browsing() {
        let mut p = PromptWidget::new();
        p.push_history("previous");
        p.handle_key(key(KeyCode::Up, KeyModifiers::NONE));
        let area = Rect::new(0, 0, 60, 4);
        let mut buffer = Buffer::empty(area);
        Widget::render(p, area, &mut buffer);
        let row = (0..area.width)
            .map(|x| buffer.cell((x, 3)).expect("caption cell").symbol())
            .collect::<String>();
        assert!(row.contains("history"));
    }

    #[test]
    fn history_command_enters_search_and_filters() {
        let mut p = PromptWidget::new();
        p.push_history("alpha file");
        p.push_history("beta note");
        for ch in "/history".chars() {
            p.handle_key(key(KeyCode::Char(ch), KeyModifiers::NONE));
        }
        assert!(p.history_search_active());
        p.handle_key(key(KeyCode::Char('f'), KeyModifiers::NONE));
        p.handle_key(key(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(p.text(), "alpha file");
    }

    #[test]
    fn char_then_enter_submits() {
        let mut p = PromptWidget::new();
        p.handle_key(key(KeyCode::Char('h'), KeyModifiers::NONE));
        p.handle_key(key(KeyCode::Char('i'), KeyModifiers::NONE));
        let out = p.handle_key(key(KeyCode::Enter, KeyModifiers::NONE));
        match out {
            PromptOutcome::Submitted(s) => assert_eq!(s, "hi"),
            other => panic!("expected Submitted, got {other:?}"),
        }
        assert!(p.is_empty());
    }

    #[test]
    fn backspace_pops_last_char() {
        let mut p = PromptWidget::new();
        p.handle_key(key(KeyCode::Char('a'), KeyModifiers::NONE));
        p.handle_key(key(KeyCode::Char('b'), KeyModifiers::NONE));
        p.handle_key(key(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(p.text(), "a");
    }

    #[test]
    fn shift_alt_enter_inserts_newline_instead_of_submitting() {
        let mut p = PromptWidget::new();
        p.handle_key(key(KeyCode::Char('a'), KeyModifiers::NONE));
        let out = p.handle_key(key(KeyCode::Enter, KeyModifiers::SHIFT));
        assert_eq!(out, PromptOutcome::Edited);
        assert_eq!(p.text(), "a\n");
        // Bare Enter still submits.
        p.handle_key(key(KeyCode::Char('b'), KeyModifiers::NONE));
        let out = p.handle_key(key(KeyCode::Enter, KeyModifiers::NONE));
        assert!(matches!(out, PromptOutcome::Submitted(_)));
    }

    #[test]
    fn multiline_prompt_renders_each_line_with_one_gutter_prefix() {
        let mut p = PromptWidget::new();
        p.buffer = "first\nsecond".into();
        let lines = p.input_lines();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].to_string(), " ❯ first");
        assert_eq!(lines[1].to_string(), "   second");
    }

    #[test]
    fn submitted_prompts_are_recorded_in_history() {
        let mut p = PromptWidget::new();
        for s in ["one", "two", "two"] {
            for ch in s.chars() {
                p.handle_key(key(KeyCode::Char(ch), KeyModifiers::NONE));
            }
            p.handle_key(key(KeyCode::Enter, KeyModifiers::NONE));
        }
        // Consecutive duplicate deduped, newest last.
        assert_eq!(p.history(), &["one".to_string(), "two".to_string()]);
    }

    #[test]
    fn up_arrow_recalls_history_then_down_clears() {
        let mut p = PromptWidget::new();
        for s in ["alpha", "beta"] {
            for ch in s.chars() {
                p.handle_key(key(KeyCode::Char(ch), KeyModifiers::NONE));
            }
            p.handle_key(key(KeyCode::Enter, KeyModifiers::NONE));
        }
        p.handle_key(key(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(p.text(), "beta");
        p.handle_key(key(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(p.text(), "alpha");
        p.handle_key(key(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(p.text(), "beta");
        p.handle_key(key(KeyCode::Down, KeyModifiers::NONE));
        assert!(p.text().is_empty(), "down past newest clears the buffer");
    }

    #[test]
    fn empty_prompt_uses_bare_cursor_glyph() {
        let p = PromptWidget::new();
        let mut buffer = Buffer::empty(Rect {
            x: 0,
            y: 0,
            width: 30,
            height: 3,
        });
        p.clone().render(
            Rect {
                x: 0,
                y: 0,
                width: 30,
                height: 3,
            },
            &mut buffer,
        );
        assert_eq!(buffer.cell((2, 1)).expect("cursor cell").symbol(), "❯");
        assert_eq!(
            buffer.cell((2, 1)).expect("cursor cell").fg,
            Color::Rgb(225, 225, 225)
        );
        let row = (0..30)
            .map(|x| buffer.cell((x, 1)).expect("prompt cell").symbol())
            .collect::<String>();
        assert!(row.contains('T'), "placeholder row: {row:?}");
    }

    #[test]
    fn prompt_theme_projects_day_tokens() {
        let mut prompt = PromptWidget::new();
        prompt.set_theme(ThemeKind::GrokDay);
        let mut buffer = Buffer::empty(Rect {
            x: 0,
            y: 0,
            width: 30,
            height: 3,
        });
        prompt.render(
            Rect {
                x: 0,
                y: 0,
                width: 30,
                height: 3,
            },
            &mut buffer,
        );
        assert_eq!(
            buffer.cell((2, 1)).expect("cursor cell").fg,
            Color::Rgb(38, 38, 38)
        );
    }

    #[test]
    fn cursor_position_counts_unicode_display_width() {
        let mut p = PromptWidget::new();
        p.handle_key(key(KeyCode::Char('界'), KeyModifiers::NONE));
        let pos = p.cursor_position(Rect {
            x: 4,
            y: 7,
            width: 20,
            height: 3,
        });
        assert_eq!(pos, ratatui::layout::Position::new(10, 8));
    }

    #[test]
    fn test_backend_receives_prompt_cursor_position() {
        use ratatui::backend::Backend;
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let mut prompt = PromptWidget::new();
        prompt.handle_key(key(KeyCode::Char('a'), KeyModifiers::NONE));
        let mut terminal = Terminal::new(TestBackend::new(20, 4)).expect("terminal");
        terminal
            .draw(|frame| {
                let area = frame.area();
                frame.render_widget(prompt.clone(), area);
                frame.set_cursor_position(prompt.cursor_position(area));
            })
            .expect("draw prompt");
        assert_eq!(
            terminal
                .backend_mut()
                .get_cursor_position()
                .expect("cursor"),
            ratatui::layout::Position::new(5, 1)
        );
    }
}
