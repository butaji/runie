//! Plan mode modal — shows structured implementation plan for approval.
//!
//! In Plan mode, the agent generates a structured plan document with sections,
//! numbered implementation steps, and bullet points. The user reviews the plan
//! and approves/denies it as a batch.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::Widget,
};
use crate::style::box_chars;

/// A single item in a plan section - either a numbered step or a bullet point.
#[derive(Debug, Clone)]
pub enum PlanItem {
    /// Numbered implementation step
    Step { number: usize, text: String },
    /// Bullet point for details/context
    Bullet { text: String },
    /// Section header
    SectionHeader { title: String },
}

/// A section within the plan document.
#[derive(Debug, Clone)]
pub struct PlanSection {
    pub title: String,
    pub items: Vec<PlanItem>,
}

/// The complete plan document shown in the modal.
#[derive(Debug, Clone, Default)]
pub struct PlanDocument {
    pub sections: Vec<PlanSection>,
}

impl PlanDocument {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns all items flattened with their display line numbers
    pub fn all_items(&self) -> Vec<(usize, PlanItem)> {
        let mut items = Vec::new();
        let mut line_num = 1;
        for section in &self.sections {
            items.push((line_num, PlanItem::SectionHeader { title: section.title.clone() }));
            line_num += 1;
            for item in &section.items {
                items.push((line_num, item.clone()));
                line_num += 1;
            }
        }
        items
    }

    /// Returns the total line count
    pub fn total_lines(&self) -> usize {
        let mut count = 0;
        for section in &self.sections {
            count += 1; // section header
            count += section.items.len();
        }
        count
    }
}

/// A single tool in the plan (legacy format, used during plan mode collection).
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
    pub document: PlanDocument,
    pub selected: usize,
    pub scroll_offset: usize,
    pub user_comment: String,
}

impl PlanModal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_with_document(&mut self, document: PlanDocument) {
        self.open = true;
        self.document = document;
        self.tools.clear();
        self.selected = 0;
        self.scroll_offset = 0;
        self.user_comment.clear();
    }

    /// Open with legacy tool list format (for backward compatibility with plan mode collection)
    pub fn open_with_tools(&mut self, tools: Vec<PlanTool>) {
        self.open = true;
        self.tools = tools;
        self.selected = 0;
        self.scroll_offset = 0;
        self.user_comment.clear();
    }

    pub fn close(&mut self) {
        self.open = false;
        self.tools.clear();
        self.document = PlanDocument::default();
        self.selected = 0;
        self.scroll_offset = 0;
        self.user_comment.clear();
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        }
    }

    pub fn move_down(&mut self) {
        let max_idx = self.document.total_lines().saturating_sub(1);
        if self.selected < max_idx {
            self.selected += 1;
            let visible_lines = 10; // Approximate visible lines
            if self.selected >= self.scroll_offset + visible_lines {
                self.scroll_offset = self.selected - visible_lines + 1;
            }
        }
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
        if self.selected < self.scroll_offset {
            self.selected = self.scroll_offset;
        }
    }

    pub fn scroll_down(&mut self) {
        let max_lines = self.document.total_lines();
        if self.scroll_offset + 10 < max_lines {
            self.scroll_offset += 1;
        }
    }

    pub fn add_comment(&mut self, c: char) {
        self.user_comment.push(c);
    }

    pub fn backspace_comment(&mut self) {
        self.user_comment.pop();
    }

    pub fn clear_comment(&mut self) {
        self.user_comment.clear();
    }

    pub fn selected_item(&self) -> Option<(usize, PlanItem)> {
        self.document.all_items().get(self.selected).cloned()
    }

    pub fn all_tools(&self) -> &[PlanTool] {
        &self.tools
    }

    pub fn clear(&mut self) {
        self.tools.clear();
        self.document = PlanDocument::default();
        self.selected = 0;
        self.scroll_offset = 0;
    }
}

/// Renders the plan frame with border.
fn render_plan_frame(area: Rect, buf: &mut Buffer, bg: Color, border: Color) -> Rect {
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            buf.get_mut(x, y).set_bg(bg);
        }
    }
    // Top border
    buf.get_mut(area.left(), area.top()).set_fg(border);
    buf.get_mut(area.right().saturating_sub(1), area.top()).set_fg(border);
    // Bottom border
    buf.get_mut(area.left(), area.bottom().saturating_sub(1)).set_fg(border);
    buf.get_mut(area.right().saturating_sub(1), area.bottom().saturating_sub(1)).set_fg(border);
    // Left border
    for y in area.top()..area.bottom() {
        buf.get_mut(area.left(), y).set_fg(border);
    }
    // Right border
    for y in area.top()..area.bottom() {
        buf.get_mut(area.right().saturating_sub(1), y).set_fg(border);
    }
    Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    }
}

/// Render the centered title divider: "─── plan.md ───"
fn render_title_divider(inner: Rect, buf: &mut Buffer, text_color: Color, border_color: Color) {
    let title = "─── plan.md ───";
    let title_len = title.len() as u16;
    let x = inner.x + (inner.width.saturating_sub(title_len)) / 2;
    buf.set_string(x, inner.y, title, Style::default().fg(text_color).add_modifier(Modifier::BOLD));

    // Draw line before title
    let line_len = (inner.width.saturating_sub(title_len)) / 2;
    if line_len > 0 {
        let line = box_chars::H.to_string().repeat(line_len as usize);
        buf.set_string(inner.x, inner.y, &line, Style::default().fg(border_color));
        buf.set_string(inner.x + inner.width.saturating_sub(line_len), inner.y, &line, Style::default().fg(border_color));
    }
}

/// Render a section header in colored text
fn render_section_header(x: u16, y: u16, title: &str, _width: u16, color: Color, buf: &mut Buffer) {
    let text = format!(" {} ", title);
    buf.set_string(x, y, &text, Style::default().fg(color).add_modifier(Modifier::BOLD));
}

/// Render a numbered step
fn render_step_item(x: u16, y: u16, number: usize, text: &str, width: u16, text_color: Color, buf: &mut Buffer) {
    let prefix = format!("{}. ", number);
    let prefix_len = prefix.len() as u16;
    let max_text_len = width.saturating_sub(prefix_len) as usize;
    let display_text = if text.len() > max_text_len {
        format!("{}…", &text[..max_text_len.saturating_sub(1)])
    } else {
        text.to_string()
    };
    let line = Line::from(vec![
        ratatui::text::Span::raw("  "),
        ratatui::text::Span::styled(prefix, Style::default().fg(text_color).add_modifier(Modifier::BOLD)),
        ratatui::text::Span::styled(display_text, Style::default().fg(text_color)),
    ]);
    buf.set_line(x, y, &line, width);
}

/// Render a bullet point
fn render_bullet_item(x: u16, y: u16, text: &str, width: u16, bullet_color: Color, text_color: Color, buf: &mut Buffer) {
    let max_text_len = width.saturating_sub(4) as usize;
    let display_text = if text.len() > max_text_len {
        format!("{}…", &text[..max_text_len.saturating_sub(1)])
    } else {
        text.to_string()
    };
    let line = Line::from(vec![
        ratatui::text::Span::raw("  "),
        ratatui::text::Span::styled("• ", Style::default().fg(bullet_color)),
        ratatui::text::Span::styled(display_text, Style::default().fg(text_color)),
    ]);
    buf.set_line(x, y, &line, width);
}

/// Render the plan document content
fn render_plan_document(modal: &PlanModal, inner: Rect, buf: &mut Buffer, section_color: Color, step_color: Color, bullet_color: Color, text_primary: Color) {
    let content_x = inner.x;
    let content_width = inner.width;
    let visible_count = (inner.height.saturating_sub(4)) as usize;
    let start_y = inner.y + 2;

    let all_items = modal.document.all_items();
    let visible_items: Vec<_> = all_items
        .into_iter()
        .skip(modal.scroll_offset)
        .take(visible_count)
        .collect();

    for (i, (_line_num, item)) in visible_items.into_iter().enumerate() {
        let y = start_y + i as u16;
        if y >= inner.y + inner.height - 1 {
            break;
        }

        match item {
            PlanItem::SectionHeader { title } => {
                render_section_header(content_x, y, &title, content_width, section_color, buf);
            }
            PlanItem::Step { number, text } => {
                render_step_item(content_x, y, number, &text, content_width, step_color, buf);
            }
            PlanItem::Bullet { text } => {
                render_bullet_item(content_x, y, &text, content_width, bullet_color, text_primary, buf);
            }
        }
    }
}

/// Render user comment at bottom
fn render_user_comment(modal: &PlanModal, inner: Rect, buf: &mut Buffer, text_secondary: Color) {
    if !modal.user_comment.is_empty() {
        let comment_text = format!("Comment: {}", modal.user_comment);
        let max_len = inner.width.saturating_sub(2) as usize;
        let display_text = if comment_text.len() > max_len {
            format!("{}…", &comment_text[..max_len.saturating_sub(1)])
        } else {
            comment_text
        };
        buf.set_string(inner.x + 1, inner.y + inner.height.saturating_sub(1), &display_text, Style::default().fg(text_secondary));
    }
}

impl Widget for &PlanModal {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let bg = Color::Rgb(25, 25, 25);
        let border = Color::DarkGray;
        let accent = Color::Cyan;
        let text_primary = Color::White;
        let text_secondary = Color::Gray;
        let section_color = Color::Yellow;
        let step_color = Color::Green;
        let bullet_color = Color::Blue;
        let highlight_bg = Color::Rgb(40, 40, 40);

        let inner = render_plan_frame(area, buf, bg, border);

        // Title divider
        if inner.height > 1 {
            render_title_divider(inner, buf, accent, border);
        }

        // Content
        if inner.height > 3 {
            if self.document.sections.is_empty() && !self.tools.is_empty() {
                // Legacy tool list rendering
                render_legacy_tools(self, inner, buf, accent, highlight_bg, text_primary);
            } else {
                // New structured document rendering
                render_plan_document(self, inner, buf, section_color, step_color, bullet_color, text_primary);
            }
        }

        // Footer with shortcuts
        if inner.height > 2 {
            let footer = "Enter:approve  Esc:close  ↑↓:scroll";
            buf.set_string(inner.x, inner.y + inner.height.saturating_sub(1), footer, Style::default().fg(text_secondary));
        }

        // User comment
        if inner.height > 3 {
            render_user_comment(self, inner, buf, text_secondary);
        }
    }
}

/// Legacy rendering for tool list format
fn render_legacy_tools(modal: &PlanModal, inner: Rect, buf: &mut Buffer, accent: Color, highlight_bg: Color, text_primary: Color) {
    let list_start = inner.y + 2;
    let visible_count = inner.height.saturating_sub(4) as usize;
    for (i, tool) in modal.tools.iter().enumerate().take(visible_count) {
        let y = list_start + i as u16;
        if y >= inner.y + inner.height {
            break;
        }
        let is_selected = i == modal.selected;
        let icon = if is_selected { "▶" } else { " " };
        let text = format!("{} {} — {}", icon, tool.tool_name, tool.tool_args);
        let style = if is_selected {
            Style::default().fg(accent).bg(highlight_bg).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(text_primary)
        };
        buf.set_line(inner.x, y, &Line::from(text).style(style), inner.width);
    }
}
