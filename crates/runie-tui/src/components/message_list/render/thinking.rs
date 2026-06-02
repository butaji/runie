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

pub fn render_thinking_block(
    block: &ThinkingBlock,
    area: Rect,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
    row: u16,
    margin_x: u16,
) -> u16 {
    let accent_color: Color = theme.color("accent.thinking").into();
    let bg_color: Color = theme.color("bg.panel").into();
    let text_color: Color = theme.color("text.primary").into();
    let _muted_color: Color = theme.color("text.muted").into();
    
    let y = area.y + row;
    if y >= area.bottom() {
        return 0;
    }
    
    // Animated accent bar
    let bar_char = if block.animation_frame % 4 < 2 { '┃' } else { '│' };
    let bar_style = Style::default().fg(accent_color).bg(bg_color);
    if let Some(cell) = buf.cell_mut((margin_x, y)) {
        cell.set_char(bar_char);
        cell.set_style(bar_style);
    }
    
    // Header
    let header_text = if block.collapsed {
        format!("{} ◆ Thought for {:.1}s ▶", glyphs::CHEVRON, block.duration_secs)
    } else {
        format!("{} ◆ Thinking…", glyphs::CHEVRON)
    };
    let header = Line::styled(header_text, Style::default().fg(accent_color).bg(bg_color).add_modifier(Modifier::BOLD));
    buf.set_line(margin_x + 2, y, &header, area.width.saturating_sub(margin_x + 4));
    
    if block.collapsed {
        return 1;
    }
    
    // Content lines
    let text_width = (area.width - margin_x - 6) as usize;
    let content = block.content.trim();
    let lines: Vec<&str> = content.lines().collect();
    let mut rendered = 2; // header + spacing
    
    for (i, line_text) in lines.iter().enumerate() {
        let line_y = y + 2 + i as u16;
        if line_y >= area.bottom() {
            break;
        }
        // Accent bar on each line
        if let Some(cell) = buf.cell_mut((margin_x, line_y)) {
            cell.set_char('┃');
            cell.set_style(Style::default().fg(accent_color).bg(bg_color));
        }
        // Background
        for x in (margin_x as usize)..(area.right() as usize) {
            if let Some(cell) = buf.cell_mut((x as u16, line_y)) {
                cell.set_style(Style::default().bg(bg_color));
            }
        }
        let line = Line::styled(*line_text, Style::default().fg(text_color).bg(bg_color));
        buf.set_line(margin_x + 2, line_y, &line, text_width as u16);
        rendered += 1;
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