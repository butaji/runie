use ratatui::{buffer::Buffer, layout::Rect, style::{Style, Modifier, Color}};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

const LANE_WIDTH: u16 = 3;
const CHARS: &str = "01アイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワヲン";

#[derive(Debug, Clone)]
pub struct MatrixRain {
    columns: Vec<RainColumn>,
    width: u16,
    height: u16,
    tick: u64,
}

#[derive(Debug, Clone)]
struct RainColumn {
    lane: u16,
    head: f32,
    speed: f32,
    length: u16,
    chars: Vec<char>,
}

impl MatrixRain {
    pub fn new(width: u16, height: u16) -> Self {
        let lanes = (width / LANE_WIDTH).max(1);
        let mut rng = StdRng::seed_from_u64(12345);
        let mut columns = Vec::new();

        for lane in 0..lanes {
            for _ in 0..2 {
                columns.push(RainColumn {
                    lane,
                    head: -(rng.gen::<f32>() * height as f32 * 2.0),
                    speed: 0.3 + rng.gen::<f32>() * 0.8,
                    length: 8 + rng.gen::<u16>() % 15,
                    chars: {
                        let chars: Vec<char> = CHARS.chars().collect();
                        (0..50).map(|_| chars[rng.gen::<usize>() % chars.len()]).collect()
                    },
                });
            }
        }

        Self { columns, width, height, tick: 0 }
    }

    pub fn tick(&mut self) {
        self.tick += 1;
        for col in &mut self.columns {
            col.head += col.speed;
            if col.head - col.length as f32 > self.height as f32 {
                col.head = -(col.length as f32);
            }
        }
    }

    pub fn render(&self,
        buf: &mut Buffer,
        area: Rect,
        accent_color: Color,
        bg_color: Color,
    ) {
        // Fill background
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char(' ');
                    cell.set_style(Style::default().bg(bg_color));
                }
            }
        }

        for col in &self.columns {
            let base_x = area.x + col.lane * LANE_WIDTH + 1;
            if base_x >= area.x + area.width {
                continue;
            }

            let head = col.head as i16;

            for i in 0..col.length {
                let y = head - i as i16;
                if y < 0 || y >= area.height as i16 {
                    continue;
                }
                let y = area.y + y as u16;

                // Pick char from the column's char stream
                let char_idx = ((self.tick as usize + col.lane as usize + i as usize) % col.chars.len());
                let ch = col.chars[char_idx];

                // Color based on position in trail
                let is_head = i == 0;
                let style = if is_head {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else if i < 3 {
                    Style::default()
                        .fg(Color::Rgb(0, 255, 0))
                } else if i < 6 {
                    Style::default()
                        .fg(Color::Rgb(0, 180, 0))
                } else {
                    Style::default()
                        .fg(Color::Rgb(0, 100, 0))
                        .add_modifier(Modifier::DIM)
                };

                if let Some(cell) = buf.cell_mut((base_x, y)) {
                    cell.set_char(ch);
                    cell.set_style(style);
                }
            }
        }
    }
}

/// Center logo ASCII art
const LOGO_ART: &str = r#" _____        _______
| _   |      |   _   |
|.|   |      |.  |   |
`-.   |      |.  |   |
  :   |      |:  |   |
  ::. |      |::.. . |
 `---'       `-------'"#;

pub fn render_ascii_art(buf: &mut Buffer, area: Rect, color: Color, bg_color: Color) {
    let lines: Vec<&str> = LOGO_ART.lines().collect();
    let art_height = lines.len() as u16;
    let art_width = lines.iter().map(|l| l.len()).max().unwrap_or(0) as u16;

    let start_x = area.x + (area.width.saturating_sub(art_width)) / 2;
    let start_y = area.y + (area.height.saturating_sub(art_height)) / 2;

    for (i, line) in lines.iter().enumerate() {
        let y = start_y + i as u16;
        if y >= area.y + area.height || line.is_empty() {
            continue;
        }
        for (col, ch) in line.chars().enumerate() {
            let sx = start_x + col as u16;
            if sx >= area.x + area.width || ch == ' ' {
                continue;
            }
            if let Some(cell) = buf.cell_mut((sx, y)) {
                cell.set_char(ch);
                cell.set_fg(color);
                cell.set_bg(bg_color);
            }
        }
    }
}

/// Full onboarding screen
pub fn render_onboarding_screen(
    rain: &MatrixRain,
    buf: &mut Buffer,
    area: Rect,
    accent_color: Color,
    bg_color: Color,
) {
    rain.render(buf, area, accent_color, bg_color);
    render_ascii_art(buf, area, accent_color, bg_color);

    let prompt = "Press Enter to start";
    let prompt_x = area.x + (area.width.saturating_sub(prompt.len() as u16)) / 2;
    let prompt_y = area.y + area.height.saturating_sub(2);

    if prompt_y >= area.y && prompt_y < area.y + area.height {
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, prompt_y)) {
                cell.set_char(' ');
                cell.set_style(Style::default().bg(bg_color));
            }
        }
        buf.set_string(prompt_x, prompt_y, prompt, Style::default().fg(accent_color).bg(bg_color));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_rain() {
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        let mut rain = MatrixRain::new(80, 24);
        rain.tick();
        rain.render(&mut buf, area, Color::White, Color::Black);
    }
}
