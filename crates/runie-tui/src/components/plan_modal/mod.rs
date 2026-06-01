//! Plan mode modal — shows queued tool calls for batch approval.
//!
//! In Plan mode, tool calls are collected into a queue instead of
//! showing individual permission modals. The user reviews all tools
//! in the plan and approves/denies them as a batch.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::Widget,
};
use crate::theme::ThemeWrapper;

/// A single tool in the plan.
#[derive(Debug, Clone)]
pub struct PlanTool {
    pub tool_call_id: String,
    pub tool_name: String,
    pub tool_args: String,
}

/// Plan modal state.
#[derive(Debug, Clone, Default)]
pub struct PlanModal {
    pub open: bool,
    pub tools: Vec<PlanTool>,
    pub selected: usize,
}

impl PlanModal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self, tools: Vec<PlanTool>) {
        self.open = true;
        self.tools = tools;
        self.selected = 0;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.tools.clear();
        self.selected = 0;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.tools.len() {
            self.selected += 1;
        }
    }

    pub fn selected_tool(&self) -> Option<&PlanTool> {
        self.tools.get(self.selected)
    }

    pub fn all_tools(&self) -> &[PlanTool] {
        &self.tools
    }

    pub fn clear(&mut self) {
        self.tools.clear();
        self.selected = 0;
    }
}

fn render_plan_frame(area: Rect, buf: &mut Buffer, bg: Color, border: Color) -> Rect {
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() { buf.get_mut(x, y).set_bg(bg); }
    }
    for x in area.left()..area.right() {
        buf.get_mut(x, area.top()).set_fg(border);
        buf.get_mut(x, area.bottom().saturating_sub(1)).set_fg(border);
    }
    for y in area.top()..area.bottom() {
        buf.get_mut(area.left(), y).set_fg(border);
        buf.get_mut(area.right().saturating_sub(1), y).set_fg(border);
    }
    Rect { x: area.x + 1, y: area.y + 1, width: area.width.saturating_sub(2), height: area.height.saturating_sub(2) }
}

fn render_plan_tools(modal: &PlanModal, inner: Rect, buf: &mut Buffer, accent: Color, highlight_bg: Color, text_primary: Color) {
    let list_start = inner.y + 2;
    let visible_count = inner.height.saturating_sub(4) as usize;
    for (i, tool) in modal.tools.iter().enumerate().take(visible_count) {
        let y = list_start + i as u16;
        if y >= inner.y + inner.height { break; }
        let is_selected = i == modal.selected;
        let icon = if is_selected { "▶" } else { " " };
        let text = format!("{} {} — {}", icon, tool.tool_name, tool.tool_args);
        let style = if is_selected { Style::default().fg(accent).bg(highlight_bg).add_modifier(Modifier::BOLD) } else { Style::default().fg(text_primary) };
        buf.set_line(inner.x, y, &Line::from(text).style(style), inner.width);
    }
}

impl Widget for &PlanModal {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let bg = Color::Rgb(25, 25, 25);
        let border = Color::DarkGray;
        let accent = Color::Cyan;
        let text_primary = Color::White;
        let text_secondary = Color::Gray;
        let highlight_bg = Color::Rgb(40, 40, 40);
        let inner = render_plan_frame(area, buf, bg, border);
        let header = format!(" Plan Mode — {} tool(s) ", self.tools.len());
        let header_style = Style::default().fg(accent).add_modifier(Modifier::BOLD);
        buf.set_string(inner.x + inner.width.saturating_sub(header.len() as u16) / 2, inner.y, header, header_style);
        if inner.height > 1 {
            let divider = "─".repeat(inner.width as usize);
            buf.set_string(inner.x, inner.y + 1, &divider, Style::default().fg(border));
        }
        render_plan_tools(self, inner, buf, accent, highlight_bg, text_primary);
        if inner.height > 2 {
            let footer = "y:approve all  n:deny all  ↑↓:navigate  esc:cancel";
            buf.set_string(inner.x, inner.y + inner.height.saturating_sub(1), footer, Style::default().fg(text_secondary));
        }
    }
}
