//! All static labels and text constants.
//!
//! Design system (colors, glyphs, borders) lives in runie-tui::theme.

/// Format timestamp from f64 (unix seconds) to HH:MM (UTC).
pub fn format_timestamp(unix_secs: f64) -> String {
    let secs = unix_secs as i64;
    let hours = (secs / 3600) % 24;
    let mins = (secs / 60) % 60;
    format!("{:02}:{:02}", hours, mins)
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
    format!("◆ Thought {:.1}s", seconds)
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
