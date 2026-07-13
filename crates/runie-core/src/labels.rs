//! All static labels and text constants.
//!
//! Design system (colors, glyphs, borders) lives in runie-tui::theme.

/// Format timestamp from f64 (unix seconds) to H:MM AM/PM (local time).
pub fn format_timestamp(unix_secs: f64) -> String {
    let datetime = chrono::DateTime::from_timestamp(unix_secs as i64, 0)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());
    datetime.format("%-I:%M %p").to_string()
}

/// Format an elapsed duration the way grok does (GROK.md §24): one decimal
/// below 10 seconds (`0.4s`, `9.9s`), integer seconds at ≥10s (`24s`).
pub fn format_elapsed_secs(secs: f64) -> String {
    if secs < 10.0 {
        format!("{:.1}s", secs)
    } else {
        format!("{:.0}s", secs)
    }
}

// Legacy labels (deprecated)
pub const THINKING_LOADING: &str = "Thinking...";

/// The 6-frame braille spinner symbols from throbber-widgets-tui BRAILLE_SIX.
/// Index 0 → '⠷', index 5 → '⠋' (the default initial frame).
pub const BRAILLE_SIX: &[char] = &['⠷', '⠯', '⠟', '⠻', '⠽', '⠾'];

// throbber BRAILLE_SIX[5] = '⠋' — used as the default/initial spinner frame.
pub const SPINNER: char = BRAILLE_SIX[5];

/// Unified action text: spinner + tag + timer.
/// Tags ending with "ing" (ongoing actions) automatically get "...".
pub fn action_text(spinner: char, tag: &str, elapsed: f64) -> String {
    if tag.ends_with("ing") {
        format!("{} {}... {:.1}s", spinner, tag, elapsed)
    } else {
        format!("{} {} {:.1}s", spinner, tag, elapsed)
    }
}

/// tui1-style thinking indicator
pub fn thinking_with_time(seconds: f64) -> String {
    format!("◐ Thinking... {:.1}s", seconds)
}

/// tui1-style thought indicator
pub fn thought_with_time(seconds: f64) -> String {
    format!("◆ Thought for {:.1}s", seconds)
}

/// tui1-style tool running
pub fn tool_running(name: &str) -> String {
    format!("⠋ Running {}...", name)
}

/// tui1-style tool done
pub fn tool_done(name: &str, seconds: f64) -> String {
    format!("✓ {} {:.1}s", name, seconds)
}

pub const SPINNER_THINKING: char = '◐';
