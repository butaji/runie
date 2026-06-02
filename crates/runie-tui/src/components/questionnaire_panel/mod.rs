//! Interactive Questionnaire Panel - Grok Build style

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Widget, Clear, Block, Borders},
};
use crate::style::box_chars::H;
use crate::style::selection::STATUS_ACTIVE;
use crate::style::selection::STATUS_IDLE;
use crate::style::selection::RADIO_SELECTED;
use crate::style::selection::RADIO_UNSELECTED;
use crate::style::StyleSet;
use crate::theme::ThemeWrapper;

#[derive(Debug, Clone)]
pub struct QuestionOption {
    pub label: String,
    pub subtitle: String,
    pub selected: bool,
}

#[derive(Debug, Clone)]
pub struct Question {
    pub prompt: String,
    pub options: Vec<QuestionOption>,
    pub custom_input: Option<String>,
    pub custom_mode: bool,
}

#[derive(Debug, Clone)]
pub struct QuestionnaireState {
    pub questions: Vec<Question>,
    pub current_question: usize,
    pub selected_option: usize,
    pub visible: bool,
    pub turn_duration: std::time::Duration,
}

impl QuestionnaireState {
    pub fn new(questions: Vec<Question>) -> Self {
        Self {
            questions,
            current_question: 0,
            selected_option: 0,
            visible: false,
            turn_duration: std::time::Duration::from_secs(0),
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn next_option(&mut self) {
        if let Some(q) = self.questions.get_mut(self.current_question) {
            let max = q.options.len() + if q.custom_mode { 0 } else { 1 };
            self.selected_option = (self.selected_option + 1) % max.max(1);
        }
    }

    pub fn prev_option(&mut self) {
        if let Some(q) = self.questions.get_mut(self.current_question) {
            let max = q.options.len() + if q.custom_mode { 0 } else { 1 };
            if self.selected_option == 0 {
                self.selected_option = max.saturating_sub(1);
            } else {
                self.selected_option -= 1;
            }
        }
    }

    pub fn next_question(&mut self) {
        if self.current_question + 1 < self.questions.len() {
            self.current_question += 1;
            self.selected_option = 0;
        }
    }

    pub fn prev_question(&mut self) {
        if self.current_question > 0 {
            self.current_question -= 1;
            self.selected_option = 0;
        }
    }

    pub fn select_current(&mut self) {
        if let Some(q) = self.questions.get_mut(self.current_question) {
            for (i, opt) in q.options.iter_mut().enumerate() {
                opt.selected = i == self.selected_option;
            }
        }
    }

    pub fn toggle_custom(&mut self) {
        if let Some(q) = self.questions.get_mut(self.current_question) {
            q.custom_mode = !q.custom_mode;
        }
    }
}

impl Default for QuestionnaireState {
    fn default() -> Self {
        Self::new(vec![])
    }
}

impl Widget for &QuestionnaireState {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.visible || self.questions.is_empty() {
            return;
        }
        render_questionnaire(self, area, buf);
    }
}

fn render_questionnaire(state: &QuestionnaireState, area: Rect, buf: &mut Buffer) {
    let theme = ThemeWrapper::default();
    let styles = StyleSet::from_theme(&theme);

    Clear.render(area, buf);
    Block::default().borders(Borders::ALL).border_style(Style::default().fg(styles.accent.fg.unwrap())).render(area, buf);

    let inner = Rect::new(area.x + 2, area.y + 1, area.width.saturating_sub(4), area.height.saturating_sub(2));
    let total = state.questions.len();
    let current = state.current_question + 1;

    render_questionnaire_header(state, inner, current, total, &styles, buf);
    let y = render_questionnaire_options(state, inner, &styles, buf);
    render_questionnaire_footer(inner, y, current, total, &styles, buf);
}

fn render_questionnaire_header(state: &QuestionnaireState, inner: Rect, current: usize, total: usize, styles: &StyleSet, buf: &mut Buffer) {
    let mut dots = String::new();
    for i in 0..total {
        dots.push(if i < current { STATUS_ACTIVE } else { STATUS_IDLE });
        if i < total - 1 {
            dots.push(' ');
        }
    }
    let header = format!("{}  Waiting on answers for {} questions              [turn: {:.1}s]", dots, total, state.turn_duration.as_secs_f32());
    let header_line = Line::styled(header, styles.text_primary);
    buf.set_line(inner.x, inner.y, &header_line, inner.width);

    let div: String = std::iter::repeat(H).take(inner.width as usize).collect();
    buf.set_line(inner.x, inner.y + 1, &Line::styled(div, styles.muted), inner.width);
}

fn render_questionnaire_options(state: &QuestionnaireState, inner: Rect, styles: &StyleSet, buf: &mut Buffer) -> u16 {
    let question = &state.questions[state.current_question];
    let mut y = inner.y + 3;

    for (i, opt) in question.options.iter().enumerate() {
        let radio = if opt.selected { RADIO_SELECTED } else { RADIO_UNSELECTED };
        let num = format!("{:2}", i + 1);
        let is_selected = i == state.selected_option;
        let style = if is_selected {
            styles.accent.add_modifier(Modifier::BOLD)
        } else {
            styles.text_primary
        };
        let line = Line::from(vec![
            Span::styled(format!("{}  {}  ", num, radio), style),
            Span::styled(&opt.label, style),
        ]);
        buf.set_line(inner.x, y, &line, inner.width);
        y += 1;
        let sub = Line::styled(format!("        {}", opt.subtitle), styles.muted);
        buf.set_line(inner.x, y, &sub, inner.width);
        y += 2;
    }

    if question.custom_mode || !question.custom_input.is_none() {
        let radio = if state.selected_option == question.options.len() { RADIO_SELECTED } else { RADIO_UNSELECTED };
        let custom_text = question.custom_input.as_deref().unwrap_or("Type your answer here");
        let line = Line::from(vec![
            Span::styled(format!(" z  {}  ", radio), styles.text_primary),
            Span::styled(custom_text, styles.muted),
        ]);
        buf.set_line(inner.x, y, &line, inner.width);
    }
    y
}

fn render_questionnaire_footer(inner: Rect, footer_y: u16, current: usize, total: usize, styles: &StyleSet, buf: &mut Buffer) {
    let footer = format!("  [{}/{}]  ↑/↓ navigate  ←/→ question  Enter:select", current, total);
    let footer_line = Line::styled(footer, styles.muted);
    buf.set_line(inner.x, footer_y, &footer_line, inner.width);
}