use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
};

/// Draw a horizontal gradient border around an area.
/// Uses rounded corner chars: ╭─╮│╰─╯
pub fn render_gradient_border(area: Rect, buf: &mut Buffer, start_color: Color, end_color: Color) {
    if area.width < 2 || area.height < 2 {
        return;
    }
    render_top_border(area, buf, start_color, end_color);
    render_bottom_border(area, buf, start_color, end_color);
    render_side_borders(area, buf, start_color, end_color);
}

fn render_top_border(area: Rect, buf: &mut Buffer, start_color: Color, end_color: Color) {
    let left = area.x;
    let right = area.x + area.width - 1;
    let top = area.y;

    for x in left..=right {
        let t = (x - left) as f32 / (area.width.saturating_sub(1)).max(1) as f32;
        let color = interpolate_color(start_color, end_color, t);
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

fn render_bottom_border(area: Rect, buf: &mut Buffer, start_color: Color, end_color: Color) {
    let left = area.x;
    let right = area.x + area.width - 1;
    let bottom = area.y + area.height - 1;

    for x in left..=right {
        let t = (x - left) as f32 / (area.width.saturating_sub(1)).max(1) as f32;
        let color = interpolate_color(start_color, end_color, t);
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

fn render_side_borders(area: Rect, buf: &mut Buffer, start_color: Color, end_color: Color) {
    let left = area.x;
    let right = area.x + area.width - 1;
    let top = area.y;
    let bottom = area.y + area.height - 1;

    for y in (top + 1)..bottom {
        // Left border - start color
        if let Some(cell) = buf.cell_mut((left, y)) {
            cell.set_char('│');
            cell.set_style(Style::default().fg(start_color));
        }
        // Right border - end color
        if let Some(cell) = buf.cell_mut((right, y)) {
            cell.set_char('│');
            cell.set_style(Style::default().fg(end_color));
        }
    }
}

fn interpolate_color(from: Color, to: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    let (fr, fg, fb) = match from {
        Color::Rgb(r, g, b) => (r as f32, g as f32, b as f32),
        _ => (0.0, 0.0, 0.0),
    };
    let (tr, tg, tb) = match to {
        Color::Rgb(r, g, b) => (r as f32, g as f32, b as f32),
        _ => (255.0, 255.0, 255.0),
    };
    Color::Rgb(
        (fr + (tr - fr) * t) as u8,
        (fg + (tg - fg) * t) as u8,
        (fb + (tb - fb) * t) as u8,
    )
}
