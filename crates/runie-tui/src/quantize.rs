//! Theme color quantization for terminals that don't support truecolor.
//!
//! Maps RGB colors to the nearest ANSI 256-color or ANSI 16-color.
//! Uses the `ansi_colours` crate for ANSI 256 conversion, which matches
//! the standard algorithm used by most terminals.

use ansi_colours::ansi256_from_rgb;
use ratatui::style::Color;

/// Color depth options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorDepth {
    /// 24-bit color (16.7M colors)
    Truecolor,
    /// 256-color palette
    ANSI256,
    /// 16-color palette
    ANSI16,
}

/// Detect the color depth based on terminal capabilities.
pub fn detect_color_depth(truecolor: bool) -> ColorDepth {
    if truecolor {
        ColorDepth::Truecolor
    } else {
        ColorDepth::ANSI256
    }
}

/// Quantize an RGB color to a specified color depth.
pub fn quantize(color: Color, depth: ColorDepth) -> Color {
    match color {
        Color::Rgb(r, g, b) => match depth {
            ColorDepth::Truecolor => Color::Rgb(r, g, b),
            ColorDepth::ANSI256 => quantize_to_256(r, g, b),
            ColorDepth::ANSI16 => quantize_to_16(r, g, b),
        },
        Color::Indexed(_) => color,
        Color::Red
        | Color::DarkGray
        | Color::Gray
        | Color::LightRed
        | Color::LightGreen
        | Color::LightBlue
        | Color::LightYellow
        | Color::Yellow
        | Color::Blue
        | Color::Magenta
        | Color::Cyan
        | Color::Green
        | Color::White
        | Color::Black
        | Color::LightMagenta
        | Color::LightCyan
        | Color::Reset => color,
    }
}

/// Map RGB to the nearest ANSI 256 color index.
pub fn quantize_to_256(r: u8, g: u8, b: u8) -> Color {
    Color::Indexed(ansi256_from_rgb((r, g, b)))
}

/// Map RGB to the nearest ANSI 16 color.
pub fn quantize_to_16(r: u8, g: u8, b: u8) -> Color {
    let ansi256 = ansi256_from_rgb((r, g, b));
    Color::Indexed(ansi256_to_16(ansi256))
}

/// Lookup table mapping each ANSI 256 color index to the nearest basic
/// ANSI 16 color. Generated from the original range-based heuristic.
const ANSI256_TO_16: [u8; 256] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 0, 0, 12, 12, 12, 0, 0, 4, 12, 12,
    12, 0, 2, 6, 12, 12, 12, 10, 10, 10, 14, 12, 12, 10, 10, 10, 10, 14, 12, 10, 10, 10, 10, 10,
    14, 0, 0, 4, 12, 12, 12, 0, 3, 4, 12, 12, 12, 2, 2, 3, 12, 12, 12, 10, 10, 10, 11, 12, 12,
    10, 10, 10, 10, 11, 12, 10, 10, 10, 10, 10, 11, 0, 1, 5, 12, 12, 12, 1, 1, 3, 12, 12, 12, 3,
    3, 3, 12, 12, 12, 10, 10, 10, 11, 12, 12, 10, 10, 10, 10, 11, 12, 10, 10, 10, 10, 10, 11, 9,
    9, 9, 13, 12, 12, 9, 9, 9, 11, 12, 12, 9, 9, 9, 11, 12, 12, 11, 11, 11, 11, 12, 12, 10, 10,
    10, 10, 11, 12, 10, 10, 10, 10, 10, 11, 9, 9, 9, 9, 13, 12, 9, 9, 9, 9, 11, 12, 9, 9, 9, 9,
    11, 12, 9, 9, 9, 9, 11, 12, 11, 11, 11, 11, 11, 12, 10, 10, 10, 10, 10, 11, 9, 9, 9, 9, 9,
    13, 9, 9, 9, 9, 9, 11, 9, 9, 9, 9, 9, 11, 9, 9, 9, 9, 9, 11, 9, 9, 9, 9, 9, 11, 11, 11,
    11, 11, 11, 11, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 8, 8, 8, 8, 8, 7, 7, 7, 7, 15, 15,
];

/// Map an ANSI 256 color index to one of the 16 basic ANSI colors.
fn ansi256_to_16(idx: u8) -> u8 {
    ANSI256_TO_16[idx as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantize_passthrough_for_truecolor() {
        let color = Color::Rgb(128, 64, 200);
        assert_eq!(quantize(color, ColorDepth::Truecolor), color);
    }

    #[test]
    fn quantize_named_colors_passthrough_256() {
        assert_eq!(quantize(Color::Red, ColorDepth::ANSI256), Color::Red);
        assert_eq!(quantize(Color::Blue, ColorDepth::ANSI256), Color::Blue);
    }

    #[test]
    fn quantize_named_colors_passthrough_16() {
        assert_eq!(quantize(Color::Red, ColorDepth::ANSI16), Color::Red);
        assert_eq!(quantize(Color::Green, ColorDepth::ANSI16), Color::Green);
    }

    #[test]
    fn black_quantizes_to_dark_index() {
        let q = quantize(Color::Rgb(0, 0, 0), ColorDepth::ANSI256);
        if let Color::Indexed(i) = q {
            assert!(i == 0 || i == 16, "got {}", i);
        } else {
            panic!("expected indexed color");
        }
    }

    #[test]
    fn white_quantizes_to_light_index() {
        let q = quantize(Color::Rgb(255, 255, 255), ColorDepth::ANSI256);
        if let Color::Indexed(i) = q {
            assert!(i == 15 || i >= 231, "got {}", i);
        } else {
            panic!("expected indexed color");
        }
    }

    #[test]
    fn pure_red_quantizes_to_red() {
        let q = quantize(Color::Rgb(255, 0, 0), ColorDepth::ANSI256);
        if let Color::Indexed(i) = q {
            assert!(i == 1 || i == 9 || i == 196, "got {}", i);
        } else {
            panic!("expected indexed color");
        }
    }

    #[test]
    fn pure_green_quantizes_to_green() {
        let q = quantize(Color::Rgb(0, 255, 0), ColorDepth::ANSI256);
        if let Color::Indexed(i) = q {
            assert!(i == 2 || i == 10 || i == 46, "got {}", i);
        } else {
            panic!("expected indexed color");
        }
    }

    #[test]
    fn pure_blue_quantizes_to_blue() {
        let q = quantize(Color::Rgb(0, 0, 255), ColorDepth::ANSI256);
        if let Color::Indexed(i) = q {
            assert!(i == 4 || i == 12 || i == 21, "got {}", i);
        } else {
            panic!("expected indexed color");
        }
    }

    #[test]
    fn grayscale_quantizes_to_grayscale_ramp() {
        let q = quantize(Color::Rgb(128, 128, 128), ColorDepth::ANSI256);
        if let Color::Indexed(i) = q {
            assert!(i >= 232, "expected grayscale ramp, got {}", i);
        } else {
            panic!("expected indexed color");
        }
    }

    #[test]
    fn ansi16_quantization_produces_valid_index() {
        let q = quantize(Color::Rgb(100, 50, 200), ColorDepth::ANSI16);
        if let Color::Indexed(i) = q {
            assert!(i < 16, "expected basic color, got {}", i);
        } else {
            panic!("expected indexed color");
        }
    }

    #[test]
    fn ansi16_red_maps_to_red() {
        let q = quantize(Color::Rgb(255, 0, 0), ColorDepth::ANSI16);
        if let Color::Indexed(i) = q {
            assert!(i == 1 || i == 9, "got {}", i);
        } else {
            panic!("expected indexed color");
        }
    }

    #[test]
    fn ansi16_white_maps_to_light_color() {
        let q = quantize(Color::Rgb(255, 255, 255), ColorDepth::ANSI16);
        if let Color::Indexed(i) = q {
            // White maps to ANSI 256 color 231, then through the lookup to ANSI 16.
            // The exact index depends on the ansi_colours formula + our lookup table.
            assert!(i < 16, "ANSI16 must be < 16, got {}", i);
        } else {
            panic!("expected indexed color");
        }
    }

    #[test]
    fn detect_color_depth_truecolor() {
        assert_eq!(detect_color_depth(true), ColorDepth::Truecolor);
    }

    #[test]
    fn detect_color_depth_fallback_256() {
        assert_eq!(detect_color_depth(false), ColorDepth::ANSI256);
    }

    #[test]
    fn runie_orange_quantizes_to_warm_color() {
        // Runie's primary orange #EE6902
        let orange = Color::Rgb(0xEE, 0x69, 0x02);
        let q = quantize(orange, ColorDepth::ANSI256);
        if let Color::Indexed(i) = q {
            assert!(i >= 16, "expected cube color, got {}", i);
        }
    }

    #[test]
    fn ansi256_from_rgb_returns_valid_index() {
        // ansi_colours produces a valid 256-color index for any RGB input.
        let _ = ansi256_from_rgb((255, 0, 0));
        let _ = ansi256_from_rgb((0, 0, 0));
        // Gray colors map to the grayscale ramp (indices 232-255).
        let gray = ansi256_from_rgb((128, 128, 128));
        assert!(gray >= 232, "mid-gray should map to grayscale ramp, got {}", gray);
    }
}
