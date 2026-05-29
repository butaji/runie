use ratatui::{buffer::Buffer, layout::Rect, style::{Style, Modifier, Color}};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

const LANE_WIDTH: u16 = 2;

#[derive(Debug, Clone)]
pub struct MatrixRain {
    columns: Vec<RainColumn>,
    tick: u64,
    seed: u64,
}

#[derive(Debug, Clone)]
struct RainColumn {
    lane: u16,
    head: f32,
    length: u16,
    char_pool: Vec<char>,
    char_offset: usize,
}

impl MatrixRain {
    pub fn new(_width: u16, height: u16) -> Self {
        let mut rng = StdRng::seed_from_u64(42);
        let mut columns = Vec::new();

        let max_lanes = 200;
        for lane in 0..max_lanes {
            for _ in 0..2 {
                columns.push(RainColumn {
                    lane,
                    head: rng.gen::<f32>() * height as f32,
                    length: 6 + rng.gen::<u16>() % 10,
                    char_pool: {
                        let chars: Vec<char> = "⠁⠂⠄⠈⠐⠠⡀⢀⠃⠅⠆⠉⠊⠋⠌⠍⠎⠏".chars().collect();
                        (0..20).map(|_| chars[rng.gen::<usize>() % chars.len()]).collect()
                    },
                    char_offset: rng.gen::<usize>() % 20,
                });
            }
        }

        Self { columns, tick: 0, seed: 42 }
    }

    pub fn tick(&mut self) {
        self.tick += 1;
        // Update once per second (~12-13 ticks at 80ms interval)
        const UPDATE_INTERVAL: u64 = 12;
        if self.tick % UPDATE_INTERVAL == 0 {
            if !self.columns.is_empty() {
                // Pick exactly ONE random column to advance
                let mut rng = StdRng::seed_from_u64(self.tick + self.seed);
                let col_idx = rng.gen::<usize>() % self.columns.len();
                let col = &mut self.columns[col_idx];
                col.head += 1.0; // Move down by exactly one cell
                // Occasionally shift character offset for subtle variation
                if self.tick % (UPDATE_INTERVAL * 7) == 0 {
                    col.char_offset = (col.char_offset + 1) % col.char_pool.len().max(1);
                }
            }
        }
    }

    pub fn render(
        &self,
        buf: &mut Buffer,
        area: Rect,
        _accent_color: Color,
        bg_color: Color,
    ) {
        let lanes = (area.width / LANE_WIDTH).max(1);

        // Fill entire background
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char(' ');
                    cell.set_style(Style::default().bg(bg_color));
                }
            }
        }

        for col in &self.columns {
            if col.lane >= lanes {
                continue;
            }

            let base_x = area.x + col.lane * LANE_WIDTH;
            if base_x >= area.x + area.width {
                continue;
            }

            // Wrap head around screen height
            let cycle = area.height as f32 + col.length as f32 * 2.0;
            let mut effective_head = col.head % cycle;
            if effective_head < 0.0 {
                effective_head += cycle;
            }
            let head = effective_head as i16 - col.length as i16;

            for i in 0..col.length {
                let y = head + i as i16;

                let wrapped_y = if y < 0 {
                    y + area.height as i16 + col.length as i16
                } else if y >= area.height as i16 {
                    y % area.height as i16
                } else {
                    y
                };

                if wrapped_y < 0 || wrapped_y >= area.height as i16 {
                    continue;
                }

                let render_y = area.y + wrapped_y as u16;

                let visible_head = (effective_head as i16).clamp(0, area.height as i16);
                let is_head = (wrapped_y as i16 - visible_head).abs() < 2;

                // Coherent stream
                let char_idx = (col.char_offset + i as usize) % col.char_pool.len();
                let ch = col.char_pool[char_idx];

                let style = if is_head {
                    Style::default()
                        .fg(Color::Rgb(160, 160, 160))
                        .add_modifier(Modifier::BOLD)
                } else if i < 3 {
                    Style::default()
                        .fg(Color::Rgb(80, 80, 80))
                } else if i < 5 {
                    Style::default()
                        .fg(Color::Rgb(45, 45, 45))
                } else {
                    Style::default()
                        .fg(Color::Rgb(20, 20, 20))
                        .add_modifier(Modifier::DIM)
                };

                if let Some(cell) = buf.cell_mut((base_x, render_y)) {
                    cell.set_char(ch);
                    cell.set_style(style);
                }
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
