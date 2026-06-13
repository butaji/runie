//! Theme color quantization for terminals that don't support truecolor.
//!
//! Maps RGB colors to the nearest ANSI 256-color or ANSI 16-color.
//! This allows the same theme to look good on:
//! - Modern terminals with 24-bit color (no quantization)
//! - Terminals with 256-color palette (ANSI 256)
//! - Basic terminals with 16 colors (ANSI 16)

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
        // Default to ANSI 256 for unknown terminals
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
        // Named/Indexed colors pass through
        other => other,
    }
}

/// Map RGB to the nearest ANSI 256 color index.
pub fn quantize_to_256(r: u8, g: u8, b: u8) -> Color {
    let ansi = rgb_to_ansi256(r, g, b);
    Color::Indexed(ansi)
}

/// Compute the ANSI 256 color index for an RGB color.
fn rgb_to_ansi256(r: u8, g: u8, b: u8) -> u8 {
    // Check if it's close to grayscale - use the grayscale ramp
    if is_near_grayscale(r, g, b) {
        return grayscale_index(r);
    }

    // Use the 6x6x6 color cube
    let ri = rgb_to_cube_channel(r);
    let gi = rgb_to_cube_channel(g);
    let bi = rgb_to_cube_channel(b);

    // 6x6x6 cube starts at index 16
    16 + 36 * ri + 6 * gi + bi
}

/// Check if RGB is close to grayscale (max-min <= 10).
fn is_near_grayscale(r: u8, g: u8, b: u8) -> bool {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    max - min <= 10
}

/// Map an 8-bit color channel to its closest 6-level cube value (0-5).
fn rgb_to_cube_channel(c: u8) -> u8 {
    if c < 48 {
        0
    } else if c < 115 {
        1
    } else if c < 155 {
        2
    } else if c < 195 {
        3
    } else if c < 235 {
        4
    } else {
        5
    }
}

/// Map an 8-bit grayscale value to the ANSI 256 grayscale ramp (232-255).
fn grayscale_index(c: u8) -> u8 {
    if c < 8 {
        return 16; // black from the color cube
    }
    let level = ((c as u16 - 8) as f32 / 10.0).round() as u16;
    let level = level.min(23);
    (232 + level) as u8
}

/// Map RGB to the nearest ANSI 16 color.
pub fn quantize_to_16(r: u8, g: u8, b: u8) -> Color {
    let ansi256 = rgb_to_ansi256(r, g, b);
    Color::Indexed(ansi256_to_16(ansi256))
}

/// Map an ANSI 256 color index to one of the 16 basic ANSI colors.
fn ansi256_to_16(idx: u8) -> u8 {
    match idx {
        0..=15 => idx,
        232..=255 => {
            if idx < 244 {
                0
            } else if idx < 250 {
                8
            } else if idx < 254 {
                7
            } else {
                15
            }
        }
        _ => {
            let cube_idx = idx - 16;
            let ri = cube_idx / 36;
            let gi = (cube_idx / 6) % 6;
            let bi = cube_idx % 6;
            let avg = (ri + gi + bi) / 3;
            if avg < 1 {
                0
            } else if ri > gi && ri > bi {
                if ri > 2 {
                    9
                } else {
                    1
                }
            } else if gi > ri && gi > bi {
                if gi > 2 {
                    10
                } else {
                    2
                }
            } else if bi > ri && bi > gi {
                if bi > 2 {
                    12
                } else {
                    4
                }
            } else if ri > 0 && gi > 0 {
                if ri > 2 || gi > 2 {
                    11
                } else {
                    3
                }
            } else if ri > 0 && bi > 0 {
                if ri > 2 || bi > 2 {
                    13
                } else {
                    5
                }
            } else if gi > 0 && bi > 0 {
                if gi > 2 || bi > 2 {
                    14
                } else {
                    6
                }
            } else {
                7
            }
        }
    }
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
    fn ansi16_white_maps_to_white() {
        let q = quantize(Color::Rgb(255, 255, 255), ColorDepth::ANSI16);
        if let Color::Indexed(i) = q {
            assert!(i == 7 || i == 15, "got {}", i);
        } else {
            panic!("expected indexed color");
        }
    }

    #[test]
    fn cube_channel_boundaries() {
        assert_eq!(rgb_to_cube_channel(0), 0);
        assert_eq!(rgb_to_cube_channel(47), 0);
        assert_eq!(rgb_to_cube_channel(48), 1);
        assert_eq!(rgb_to_cube_channel(95), 1);
        assert_eq!(rgb_to_cube_channel(115), 2);
        assert_eq!(rgb_to_cube_channel(135), 2);
        assert_eq!(rgb_to_cube_channel(155), 3);
        assert_eq!(rgb_to_cube_channel(175), 3);
        assert_eq!(rgb_to_cube_channel(195), 4);
        assert_eq!(rgb_to_cube_channel(215), 4);
        assert_eq!(rgb_to_cube_channel(235), 5);
        assert_eq!(rgb_to_cube_channel(255), 5);
    }

    #[test]
    fn is_near_grayscale_detection() {
        assert!(is_near_grayscale(128, 128, 128));
        assert!(is_near_grayscale(100, 95, 105));
        assert!(!is_near_grayscale(255, 0, 0));
        assert!(!is_near_grayscale(0, 255, 0));
        assert!(!is_near_grayscale(50, 100, 200));
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
    fn all_cube_channels_covered() {
        // Test all 6 cube levels for each channel
        let levels = [0u8, 95, 135, 175, 215, 255];
        for &c in &levels {
            let result = rgb_to_cube_channel(c);
            assert!(result <= 5, "cube channel for {} should be <= 5, got {}", c, result);
        }
    }
}
