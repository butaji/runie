//! Terminal capability detection and deterministic color quantization.
//!
//! Grok resolves semantic theme colors against the terminal's advertised
//! capability. The projection is pure once the capability is supplied, which
//! keeps YAML/TestBackend replays deterministic while the live binary can
//! detect its PTY environment.

use ratatui::style::{Color, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorLevel {
    None,
    Basic,
    Ansi256,
    TrueColor,
}

impl ColorLevel {
    pub fn from_environment() -> Self {
        let colorterm = std::env::var("COLORTERM").unwrap_or_default();
        if matches!(colorterm.as_str(), "truecolor" | "24bit") {
            return Self::TrueColor;
        }
        let term = std::env::var("TERM").unwrap_or_default();
        if term == "dumb" {
            Self::None
        } else if term.contains("256color") {
            Self::Ansi256
        } else {
            Self::Basic
        }
    }
}

pub fn quantize_style(style: Style, level: ColorLevel) -> Style {
    Style {
        fg: style.fg.map(|color| quantize_color(color, level)),
        bg: style.bg.map(|color| quantize_color(color, level)),
        ..style
    }
}

pub fn quantize_buffer(buffer: &mut ratatui::buffer::Buffer, level: ColorLevel) {
    let area = buffer.area;
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            if let Some(cell) = buffer.cell_mut((x, y)) {
                cell.set_style(quantize_style(cell.style(), level));
            }
        }
    }
}

pub fn quantize_color(color: Color, level: ColorLevel) -> Color {
    match (color, level) {
        (Color::Rgb(red, green, blue), ColorLevel::TrueColor) => Color::Rgb(red, green, blue),
        (Color::Rgb(red, green, blue), ColorLevel::Ansi256) => {
            let red = u16::from(red);
            let green = u16::from(green);
            let blue = u16::from(blue);
            let red = ((red * 5 + 127) / 255) as u8;
            let green = ((green * 5 + 127) / 255) as u8;
            let blue = ((blue * 5 + 127) / 255) as u8;
            Color::Indexed(16 + 36 * red + 6 * green + blue)
        }
        (Color::Rgb(red, green, blue), ColorLevel::Basic) => {
            let candidates = [
                (Color::Black, 0_u32, 0, 0),
                (Color::Red, 205, 0, 0),
                (Color::Green, 0, 205, 0),
                (Color::Yellow, 205, 205, 0),
                (Color::Blue, 0, 0, 238),
                (Color::Magenta, 205, 0, 205),
                (Color::Cyan, 0, 205, 205),
                (Color::White, 229, 229, 229),
            ];
            candidates
                .into_iter()
                .min_by_key(|(_, r, g, b)| distance(red, green, blue, *r as u8, *g as u8, *b as u8))
                .map(|(color, ..)| color)
                .unwrap_or(Color::White)
        }
        (Color::Rgb(_, _, _), ColorLevel::None) => Color::Reset,
        (color, _) => color,
    }
}

fn distance(red: u8, green: u8, blue: u8, other_red: u8, other_green: u8, other_blue: u8) -> u32 {
    let red = i32::from(red) - i32::from(other_red);
    let green = i32::from(green) - i32::from(other_green);
    let blue = i32::from(blue) - i32::from(other_blue);
    (red * red + green * green + blue * blue) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantization_matches_terminal_levels() {
        let rgb = Color::Rgb(187, 154, 247);
        assert_eq!(quantize_color(rgb, ColorLevel::TrueColor), rgb);
        assert_eq!(
            quantize_color(rgb, ColorLevel::Ansi256),
            Color::Indexed(183)
        );
        assert_eq!(quantize_color(rgb, ColorLevel::Basic), Color::White);
        assert_eq!(quantize_color(rgb, ColorLevel::None), Color::Reset);
    }

    #[test]
    fn non_rgb_colors_survive_quantization() {
        assert_eq!(
            quantize_color(Color::Indexed(42), ColorLevel::Basic),
            Color::Indexed(42)
        );
        assert_eq!(quantize_color(Color::Reset, ColorLevel::None), Color::Reset);
    }
}
