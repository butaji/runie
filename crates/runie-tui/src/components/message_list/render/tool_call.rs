use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Line,
};
use crate::theme::ThemeWrapper;

#[derive(Debug, Clone)]
pub enum ToolStatus {
    Pending,
    Running,
    Complete,
    Error,
}

#[derive(Debug, Clone)]
pub struct ToolCallBlock {
    pub tool_name: String,
    pub args: String,
    pub status: ToolStatus,
    pub elapsed_secs: f64,
    pub total_secs: f64,
    pub bytes_in: u64,
    pub spinner_frame: usize,
}

const SPINNER_FRAMES: &[char] = &['⠦', '⠴', '⠋', '⠼', '⠦', '⠴', '⠂', '⠇'];

pub fn render_tool_call_block(
    block: &ToolCallBlock,
    area: Rect,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
    row: u16,
    margin_x: u16,
) -> u16 {
    let accent_color: Color = theme.color("accent.tool").into();
    let _text_color: Color = theme.color("text.primary").into();
    let _muted_color: Color = theme.color("text.muted").into();
    let success_color: Color = theme.color("success").into();
    let error_color: Color = theme.color("error").into();
    
    let y = area.y + row;
    if y >= area.bottom() {
        return 0;
    }
    
    match block.status {
        ToolStatus::Pending => {
            // Grok spec: pending tool calls get a "❙  " 2-cell rail prefix
            // (U+2759 HEAVY VERTICAL BAR + 2 spaces) before the "◆ ToolName ." line
            let label = format!("❙  ◆ {} {}", block.tool_name, block.args);
            let line = Line::styled(label, Style::default().fg(accent_color));
            buf.set_line(margin_x, y, &line, area.width.saturating_sub(margin_x));
        }
        ToolStatus::Running => {
            let spinner = SPINNER_FRAMES[block.spinner_frame % SPINNER_FRAMES.len()];
            let label = format!("{} Run {} `{}` {:.1}s", spinner, block.tool_name, block.args, block.elapsed_secs);
            let left = Line::styled(label, Style::default().fg(accent_color));
            buf.set_line(margin_x, y, &left, area.width.saturating_sub(margin_x));
        }
        ToolStatus::Complete => {
            // Grok spec: very short tool calls (no bytes / < 0.3s) use the
            // compact single-line form "◆ ToolName .", same as the inline form.
            if block.bytes_in == 0 || block.total_secs < 0.3 {
                let label = format!("◆ {} {}", block.tool_name, block.args);
                let left = Line::styled(label, Style::default().fg(accent_color));
                buf.set_line(margin_x, y, &left, area.width.saturating_sub(margin_x));
            } else {
                let bytes_str = format!(" ⇣{}", format_bytes(block.bytes_in));
                let label = format!("✓ {} → ok {:.1}s{} [✓]", block.tool_name, block.total_secs, bytes_str);
                let left = Line::styled(label, Style::default().fg(success_color));
                buf.set_line(margin_x, y, &left, area.width.saturating_sub(margin_x));
            }
        }
        ToolStatus::Error => {
            let label = format!("✗ {} → error {:.1}s [✗]", block.tool_name, block.total_secs);
            let left = Line::styled(label, Style::default().fg(error_color));
            buf.set_line(margin_x, y, &left, area.width.saturating_sub(margin_x));
        }
    }
    
    1
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}k", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}M", bytes as f64 / (1024.0 * 1024.0))
    }
}

pub fn render_tool_call_inline_compact(
    tool_name: &str,
    args: &str,
    area: Rect,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
    row: u16,
    margin_x: u16,
) -> u16 {
    let accent_color: Color = theme.color("accent.tool").into();
    let y = area.y + row;
    let text = format!("◆ {} {}", tool_name, args);
    let line = Line::styled(text, Style::default().fg(accent_color));
    buf.set_line(margin_x, y, &line, area.width.saturating_sub(margin_x));
    1
}