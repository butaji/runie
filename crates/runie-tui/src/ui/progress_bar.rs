//! 1/8-cell progress bar spans — port of Grok's `progress_bar.rs`.
//!
//! Filled cells use the left-fractional block glyphs `▏▎▍▌▋▊▉█` so a small
//! bar (4-6 cells) can still show fine-grained percentages. On legacy
//! Windows ConHost (missing U+258F..U+2589) the CP437 shade glyphs
//! `░▒▓` are substituted with the same index domain, so call sites never
//! branch.

use ratatui::style::Color;
use ratatui::text::Span;

/// Left-fractional blocks by eighths: index = filled eighths (0..=8).
pub(crate) const BLOCKS: [&str; 9] = ["", "▏", "▎", "▍", "▌", "▋", "▊", "▉", "█"];

/// CP437 shade fallback for legacy Windows consoles (same index domain).
const SHADES: [&str; 9] = ["", "░", "░", "░", "▒", "▒", "▓", "▓", "█"];

/// True on legacy Windows ConHost; always false on macOS/Linux.
fn is_legacy_windows_console() -> bool {
    #[cfg(windows)]
    {
        // ConHost lacks the U+258F..U+2589 glyphs; Windows Terminal has them.
        std::env::var("WT_SESSION").is_err() && std::env::var("TERM_PROGRAM").is_err()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Select the glyph table for this console.
fn partial_blocks() -> &'static [&'static str; 9] {
    if is_legacy_windows_console() {
        &SHADES
    } else {
        &BLOCKS
    }
}

/// Decompose `value` (clamped 0.0..=1.0) into `(full_cells, remainder_eighths)`.
/// E.g. 3% of a 5-cell bar lights one `▏`, not a full cell.
pub(crate) fn cell_breakdown(width: u16, value: f32) -> (u16, usize) {
    let value = value.clamp(0.0, 1.0);
    let total_eighths = (value * f32::from(width) * 8.0).round() as usize;
    let full = (total_eighths / 8).min(width as usize);
    let remainder = total_eighths % 8;
    (full as u16, remainder)
}

/// Per-cell `(symbol, is_filled)` for a bar of `width` cells at `value`.
/// The boundary cell uses `blocks[remainder]` (partial glyph) only when
/// `remainder > 0`; at exact fractions it is a track space so the bar always
/// occupies exactly `width` columns (width-invariant rule).
pub(crate) fn bar_cells(width: u16, value: f32) -> Vec<(&'static str, bool)> {
    let blocks = partial_blocks();
    let (full, remainder) = cell_breakdown(width, value);
    (0..width)
        .map(|i| {
            if (i as usize) < full as usize {
                (blocks[8], true)
            } else if (i as usize) == full as usize {
                if remainder > 0 {
                    (blocks[remainder], true)
                } else {
                    (" ", false)
                }
            } else {
                (" ", false)
            }
        })
        .collect()
}

/// Styled spans for a progress bar: filled cells `fg` on `bg`, empty cells
/// `" "` with only `bg` (track). Width-invariant callers pad around this.
pub(crate) fn progress_bar_spans(width: u16, value: f32, fg: Color, bg: Color) -> Vec<Span<'static>> {
    bar_cells(width, value)
        .into_iter()
        .map(|(symbol, is_filled)| {
            if is_filled {
                Span::styled(symbol, ratatui::style::Style::new().fg(fg).bg(bg))
            } else {
                Span::styled(symbol, ratatui::style::Style::new().bg(bg))
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breakdown_zero_value_has_no_filled_cells() {
        assert_eq!(cell_breakdown(5, 0.0), (0, 0));
    }

    #[test]
    fn breakdown_full_value_fills_all_cells() {
        assert_eq!(cell_breakdown(5, 1.0), (5, 0));
    }

    #[test]
    fn breakdown_small_pct_lights_partial_cell() {
        // 3% of 5 cells = 0.15 cells = 1.2 eighths -> rounds to 1 eighth.
        assert_eq!(cell_breakdown(5, 0.03), (0, 1));
    }

    #[test]
    fn breakdown_50pct_fills_half() {
        // 50% of 6 cells = 3.0 cells = 24 eighths -> 3 full cells.
        assert_eq!(cell_breakdown(6, 0.5), (3, 0));
    }

    #[test]
    fn breakdown_clamps_above_one() {
        assert_eq!(cell_breakdown(4, 1.5), (4, 0));
        assert_eq!(cell_breakdown(4, -0.5), (0, 0));
    }

    #[test]
    fn breakdown_rounds_to_nearest_eighth() {
        // 42% of 5 cells = 2.1 cells = 16.8 eighths -> rounds to 17 = 2 full + 1.
        assert_eq!(cell_breakdown(5, 0.42), (2, 1));
    }

    #[test]
    fn spans_have_expected_symbols() {
        let spans = progress_bar_spans(4, 1.0, Color::Red, Color::Blue);
        assert_eq!(spans.len(), 4);
        assert_eq!(spans[0].content.as_ref(), "█");
    }

    #[test]
    fn partial_blocks_selected() {
        #[cfg(not(windows))]
        {
            assert_eq!(partial_blocks()[8], "█");
            assert_eq!(partial_blocks()[1], "▏");
        }
    }
}
