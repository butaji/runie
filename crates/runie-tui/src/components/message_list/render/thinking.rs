use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
};
use crate::theme::ThemeWrapper;
use crate::glyphs;

#[derive(Debug, Clone)]
pub struct ThinkingBlock {
    pub content: String,
    pub duration_secs: f64,
    pub collapsed: bool,
    pub animation_frame: usize,
}

fn draw_horizontal_border(y: u16, _area: Rect, buf: &mut Buffer, left_margin: u16, width: u16, color: Color, bg_color: Color) {
    let border_width = width.saturating_sub(2) as usize;
    let top = format!("┌{}{}", "─".repeat(border_width), "┐");
    let line = Line::styled(top, Style::default().fg(color).bg(bg_color));
    buf.set_line(left_margin, y, &line, width);
}

fn draw_bottom_border(y: u16, _area: Rect, buf: &mut Buffer, left_margin: u16, width: u16, color: Color, bg_color: Color) {
    let border_width = width.saturating_sub(2) as usize;
    let bottom = format!("└{}{}", "─".repeat(border_width), "┘");
    let line = Line::styled(bottom, Style::default().fg(color).bg(bg_color));
    buf.set_line(left_margin, y, &line, width);
}

pub fn render_thinking_block(
    block: &ThinkingBlock,
    area: Rect,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
    row: u16,
    margin_x: u16,
) -> u16 {
    let accent_color: Color = theme.color("accent.thinking").into();
    let border_color: Color = theme.color("border.unfocused").into();
    let bg_color: Color = theme.color("bg.panel").into();
    let text_color: Color = theme.color("text.primary").into();
    
    let y = area.y + row;
    if y >= area.bottom() {
        return 0;
    }
    
    if block.collapsed {
        let bar_char = if block.animation_frame % 4 < 2 { '┃' } else { '│' };
        if let Some(cell) = buf.cell_mut((margin_x, y)) {
            cell.set_char(bar_char);
            cell.set_style(Style::default().fg(accent_color).bg(bg_color));
        }
        let header_text = format!("{} ◆ Thought for {:.1}s ▶", glyphs::CHEVRON, block.duration_secs);
        let header = Line::styled(header_text, Style::default().fg(accent_color).bg(bg_color).add_modifier(Modifier::BOLD));
        buf.set_line(margin_x + 2, y, &header, area.width.saturating_sub(margin_x + 4));
        return 1;
    }
    
    // Expanded: draw box with borders
    let content_width = area.width;
    let top_y = y;
    let header_y = y + 1;
    
    // Top border
    draw_horizontal_border(top_y, area, buf, margin_x, content_width, border_color, bg_color);
    
    // Header with side borders
    let header_text = format!("│ {} ◆ Thought for {:.1}s │", glyphs::CHEVRON, block.duration_secs);
    let header = Line::styled(header_text, Style::default().fg(accent_color).bg(bg_color).add_modifier(Modifier::BOLD));
    buf.set_line(margin_x, header_y, &header, content_width);
    
    // Content lines
    let content = block.content.trim();
    let lines: Vec<&str> = content.lines().collect();
    let mut rendered = 3; // top border + header + bottom border + content
    
    for (i, line_text) in lines.iter().enumerate() {
        let line_y = header_y + 1 + i as u16;
        if line_y >= area.bottom() {
            break;
        }
        let content_line = format!("│ {} │", line_text);
        let line = Line::styled(content_line, Style::default().fg(text_color).bg(bg_color));
        buf.set_line(margin_x, line_y, &line, content_width);
        rendered += 1;
    }
    
    // Bottom border
    let bottom_y = header_y + 1 + lines.len() as u16;
    if bottom_y < area.bottom() {
        draw_bottom_border(bottom_y, area, buf, margin_x, content_width, border_color, bg_color);
    }
    
    rendered as u16
}

pub fn render_thought_indicator(
    duration_secs: f64,
    area: Rect,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
    row: u16,
    margin_x: u16,
) -> u16 {
    let accent_color: Color = theme.color("accent.thinking").into();
    let y = area.y + row;
    let text = format!("◆ Thought for {:.1}s", duration_secs);
    let line = Line::styled(text, Style::default().fg(accent_color));
    buf.set_line(margin_x, y, &line, area.width.saturating_sub(margin_x));
    1
}