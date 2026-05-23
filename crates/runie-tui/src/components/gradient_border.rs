use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
};

/// Colors for gradient: left gray, right orange
const GRAY: (u8, u8, u8) = (128, 128, 128);
const ORANGE: (u8, u8, u8) = (255, 140, 0);

/// Draw a horizontal gradient border around an area.
/// Left side = gray, right side = orange, gradient in between.
/// Uses rounded corner chars: ╭─╮│╰─╯
pub fn render_gradient_border(area: Rect, buf: &mut Buffer) {
    if area.width < 2 || area.height < 2 {
        return;
    }
    render_top_border(area, buf);
    render_bottom_border(area, buf);
    render_side_borders(area, buf);
}

fn render_top_border(area: Rect, buf: &mut Buffer) {
    let left = area.x;
    let right = area.x + area.width - 1;
    let top = area.y;

    for x in left..=right {
        let t = (x - left) as f32 / (area.width.saturating_sub(1)).max(1) as f32;
        let color = interpolate_color(GRAY, ORANGE, t);
        let ch = match x {
            _ if x == left => '╭',
            _ if x == right => '╮',
            _ => '─',
        };
        if let Some(cell) = buf.cell_mut((x, top)) {
            cell.set_char(ch);
            cell.set_style(Style::default().fg(color));
        }
    }
}

fn render_bottom_border(area: Rect, buf: &mut Buffer) {
    let left = area.x;
    let right = area.x + area.width - 1;
    let bottom = area.y + area.height - 1;

    for x in left..=right {
        let t = (x - left) as f32 / (area.width.saturating_sub(1)).max(1) as f32;
        let color = interpolate_color(GRAY, ORANGE, t);
        let ch = match x {
            _ if x == left => '╰',
            _ if x == right => '╯',
            _ => '─',
        };
        if let Some(cell) = buf.cell_mut((x, bottom)) {
            cell.set_char(ch);
            cell.set_style(Style::default().fg(color));
        }
    }
}

fn render_side_borders(area: Rect, buf: &mut Buffer) {
    let left = area.x;
    let right = area.x + area.width - 1;
    let top = area.y;
    let bottom = area.y + area.height - 1;

    for y in (top + 1)..bottom {
        // Left border - gray
        if let Some(cell) = buf.cell_mut((left, y)) {
            cell.set_char('│');
            cell.set_style(Style::default().fg(Color::Rgb(GRAY.0, GRAY.1, GRAY.2)));
        }
        // Right border - orange
        if let Some(cell) = buf.cell_mut((right, y)) {
            cell.set_char('│');
            cell.set_style(Style::default().fg(Color::Rgb(ORANGE.0, ORANGE.1, ORANGE.2)));
        }
    }
}

fn interpolate_color(from: (u8, u8, u8), to: (u8, u8, u8), t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    let r = (from.0 as f32 + (to.0 as f32 - from.0 as f32) * t) as u8;
    let g = (from.1 as f32 + (to.1 as f32 - from.1 as f32) * t) as u8;
    let b = (from.2 as f32 + (to.2 as f32 - from.2 as f32) * t) as u8;
    Color::Rgb(r, g, b)
}