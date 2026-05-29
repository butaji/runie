use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Margin, Rect},
    style::{Color, Style},
};

pub enum PanelBorder {
    Gradient { start: Color, end: Color },
    Solid(Color),
}

pub struct Panel<'a> {
    title: Option<&'a str>,
    border: PanelBorder,
    title_color: Color,
    title_alignment: Alignment,
    show_close_hint: bool,
    close_hint_color: Color,
    bg_color: Option<Color>,
}

impl<'a> Panel<'a> {
    pub fn new() -> Self {
        Self {
            title: None,
            border: PanelBorder::Solid(Color::Gray),
            title_color: Color::Gray,
            title_alignment: Alignment::Left,
            show_close_hint: false,
            close_hint_color: Color::DarkGray,
            bg_color: None,
        }
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    pub fn border_gradient(mut self, start: Color, end: Color) -> Self {
        self.border = PanelBorder::Gradient { start, end };
        self
    }

    pub fn border_solid(mut self, color: Color) -> Self {
        self.border = PanelBorder::Solid(color);
        self
    }

    pub fn title_color(mut self, color: Color) -> Self {
        self.title_color = color;
        self
    }

    pub fn title_left(mut self) -> Self {
        self.title_alignment = Alignment::Left;
        self
    }

    pub fn title_center(mut self) -> Self {
        self.title_alignment = Alignment::Center;
        self
    }

    pub fn title_right(mut self) -> Self {
        self.title_alignment = Alignment::Right;
        self
    }

    pub fn show_close_hint(mut self, color: Color) -> Self {
        self.show_close_hint = true;
        self.close_hint_color = color;
        self
    }

    pub fn bg(mut self, color: Color) -> Self {
        self.bg_color = Some(color);
        self
    }

    pub fn render<F>(self, area: Rect, buf: &mut Buffer, render_content: F)
    where
        F: FnOnce(Rect, &mut Buffer),
    {
        if area.width < 4 || area.height < 3 {
            return;
        }

        // Optional background fill — clear chars and set bg to block rain bleed-through
        if let Some(bg) = self.bg_color {
            for y in area.y..area.y + area.height {
                for x in area.x..area.x + area.width {
                    if let Some(cell) = buf.cell_mut((x, y)) {
                        cell.set_char(' ');
                        cell.set_style(Style::default().bg(bg));
                    }
                }
            }
        }

        // Draw border
        match self.border {
            PanelBorder::Gradient { start, end } => {
                render_gradient_border_custom(area, buf, start, end);
            }
            PanelBorder::Solid(color) => {
                render_solid_border(area, buf, color);
            }
        }

        // Draw title in top border
        if let Some(title) = self.title {
            let title_text = format!(" {} ", title);
            let title_x = match self.title_alignment {
                Alignment::Left => area.x + 2,
                Alignment::Center => area.x + (area.width.saturating_sub(title_text.len() as u16)) / 2,
                Alignment::Right => area.x + area.width.saturating_sub(title_text.len() as u16 + 2),
            };
            let max_len = (area.width.saturating_sub(4)).min(title_text.len() as u16) as usize;
            if max_len > 0 {
                buf.set_string(title_x, area.y, &title_text[..max_len], Style::default().fg(self.title_color));
            }
        }

        // Draw close hint in bottom border
        if self.show_close_hint {
            let hint = " [Esc] close ";
            let hint_x = area.x + area.width.saturating_sub(hint.len() as u16 + 1);
            if hint_x > area.x + 1 {
                buf.set_string(hint_x, area.y + area.height - 1, hint, Style::default().fg(self.close_hint_color));
            }
        }

        // Content area with 1-cell margin
        let inner = area.inner(Margin::new(1, 1));
        if inner.width > 0 && inner.height > 0 {
            render_content(inner, buf);
        }
    }
}

fn render_solid_border(area: Rect, buf: &mut Buffer, color: Color) {
    let left = area.x;
    let right = area.x + area.width - 1;
    let top = area.y;
    let bottom = area.y + area.height - 1;

    // Corners
    for (x, y, ch) in [(left, top, '╭'), (right, top, '╮'), (left, bottom, '╰'), (right, bottom, '╯')] {
        if let Some(cell) = buf.cell_mut((x, y)) {
            cell.set_char(ch);
            cell.set_style(Style::default().fg(color));
        }
    }

    // Horizontal borders
    for x in (left + 1)..right {
        for y in [top, bottom] {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_char('─');
                cell.set_style(Style::default().fg(color));
            }
        }
    }

    // Vertical borders
    for y in (top + 1)..bottom {
        for x in [left, right] {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_char('│');
                cell.set_style(Style::default().fg(color));
            }
        }
    }
}

fn render_gradient_border_custom(area: Rect, buf: &mut Buffer, start: Color, end: Color) {
    let left = area.x;
    let right = area.x + area.width - 1;
    let top = area.y;
    let bottom = area.y + area.height - 1;

    // Helper to interpolate between two RGB colors
    fn interp(start: Color, end: Color, t: f32) -> Color {
        let (sr, sg, sb) = match start {
            Color::Rgb(r, g, b) => (r as f32, g as f32, b as f32),
            _ => (128.0, 128.0, 128.0),
        };
        let (er, eg, eb) = match end {
            Color::Rgb(r, g, b) => (r as f32, g as f32, b as f32),
            _ => (255.0, 140.0, 0.0),
        };
        let t = t.clamp(0.0, 1.0);
        Color::Rgb(
            (sr + (er - sr) * t) as u8,
            (sg + (eg - sg) * t) as u8,
            (sb + (eb - sb) * t) as u8,
        )
    }

    let width_f = (area.width.saturating_sub(1)).max(1) as f32;

    // Top border
    for x in left..=right {
        let t = (x - left) as f32 / width_f;
        let color = interp(start, end, t);
        let ch = if x == left { '╭' } else if x == right { '╮' } else { '─' };
        if let Some(cell) = buf.cell_mut((x, top)) {
            cell.set_char(ch);
            cell.set_style(Style::default().fg(color));
        }
    }

    // Bottom border
    for x in left..=right {
        let t = (x - left) as f32 / width_f;
        let color = interp(start, end, t);
        let ch = if x == left { '╰' } else if x == right { '╯' } else { '─' };
        if let Some(cell) = buf.cell_mut((x, bottom)) {
            cell.set_char(ch);
            cell.set_style(Style::default().fg(color));
        }
    }

    // Left border (start color)
    for y in (top + 1)..bottom {
        if let Some(cell) = buf.cell_mut((left, y)) {
            cell.set_char('│');
            cell.set_style(Style::default().fg(start));
        }
    }

    // Right border (end color)
    for y in (top + 1)..bottom {
        if let Some(cell) = buf.cell_mut((right, y)) {
            cell.set_char('│');
            cell.set_style(Style::default().fg(end));
        }
    }
}