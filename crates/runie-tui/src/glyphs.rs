//! Unified style glyphs and visual constants.
//!
//! All UI glyphs (chevrons, spinners, bullets, etc.) defined here.
//! No hardcoded glyphs elsewhere in the codebase.

/// User message prompt chevron (matches input box)
pub const CHEVRON: char = '\u{276F}'; // ❯

/// Chevron with trailing space (for prompts)
pub const CHEVRON_WITH_SPACE: &str = "\u{276F} ";

/// Assistant idle/dot indicator
pub const DOT: char = '·';

/// Thought duration diamond
pub const THOUGHT_MARKER: char = '◆';

/// Braille spinner frames (10 frames, clockwise)
pub const SPINNER_FRAMES: [char; 10] = [
    '\u{280B}', '\u{2819}', '\u{2839}', '\u{2838}', '\u{283C}',
    '\u{2834}', '\u{2826}', '\u{2827}', '\u{2807}', '\u{280F}',
];

/// Reverse braille spinner (counter-clockwise)
pub const SPINNER_FRAMES_REVERSE: [char; 10] = [
    '\u{280F}', '\u{2807}', '\u{2827}', '\u{2826}', '\u{2834}',
    '\u{283C}', '\u{2838}', '\u{2839}', '\u{2819}', '\u{280B}',
];

/// Tool call bullet
pub const TOOL_BULLET: char = '●';

/// Separator line character
pub const SEPARATOR: char = '─';

/// Error indicator
pub const ERROR_MARKER: char = '!';

/// Streaming cursor block
pub const CURSOR_BLOCK: char = '▊';

/// Gauge empty
pub const GAUGE_EMPTY: char = '○';

/// Gauge full
pub const GAUGE_FULL: char = '■';

/// Checkmark (complete)
pub const CHECK_MARKER: char = '✓';

/// Plan step pending arrow
pub const PLAN_PENDING: char = '▸';

/// Plan step active connector
pub const PLAN_ACTIVE: char = '│';

/// Rewind/reset indicator
pub const REWIND: char = '↺';

/// Interrupt/stop indicator
pub const INTERRUPT: char = '✗';

/// Pulse fill character
pub const PULSE_FILL: char = '▐';

/// Get current spinner frame from animation tick
pub fn spinner_frame(tick: usize) -> char {
    SPINNER_FRAMES[tick % SPINNER_FRAMES.len()]
}

/// Get reverse spinner frame from animation tick
pub fn spinner_frame_reverse(tick: usize) -> char {
    SPINNER_FRAMES_REVERSE[tick % SPINNER_FRAMES_REVERSE.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinner_cycles() {
        assert_eq!(spinner_frame(0), SPINNER_FRAMES[0]);
        assert_eq!(spinner_frame(10), SPINNER_FRAMES[0]);
        assert_eq!(spinner_frame(5), SPINNER_FRAMES[5]);
    }

    #[test]
    fn test_spinner_reverse_cycles() {
        assert_eq!(spinner_frame_reverse(0), SPINNER_FRAMES_REVERSE[0]);
        assert_eq!(spinner_frame_reverse(10), SPINNER_FRAMES_REVERSE[0]);
    }
}
