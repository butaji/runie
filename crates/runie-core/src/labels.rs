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

/// Grok `format_duration` port (turn-status parity): `<10s` → `0.5s`/`9.9s`
/// (one decimal), `10–59s` → `10s`/`32s`, `1m–59m` → `1m20s`/`10m0s`,
/// `1h+` → `1h0m`/`1h2m`.
pub fn format_turn_timer(d: std::time::Duration) -> String {
    let total = d.as_secs_f64();
    if total < 10.0 {
        format!("{:.1}s", total)
    } else if total < 60.0 {
        format!("{}s", total as u64)
    } else if total < 3600.0 {
        let m = (total as u64) / 60;
        let s = (total as u64) % 60;
        format!("{}m{}s", m, s)
    } else {
        let h = (total as u64) / 3600;
        let m = ((total as u64) % 3600) / 60;
        format!("{}h{}m", h, m)
    }
}

/// Grok `format_tokens_short` port (turn-status parity): raw `<1k`
/// (`500`), `{:.2}k` for `1k–9.99k` (`1.23k`), `{:.1}k` for `10k–99.9k`
/// (`10.1k`), whole `k` for `100k–999k` (`128k`), then `{:.2}m`/`{:.1}m`
/// for millions.
pub fn format_tokens_short(tokens: u64) -> String {
    let n = tokens as f64;
    if n < 1_000.0 {
        tokens.to_string()
    } else if n < 10_000.0 {
        format!("{:.2}k", n / 1_000.0)
    } else if n < 100_000.0 {
        format!("{:.1}k", n / 1_000.0)
    } else if n < 1_000_000.0 {
        format!("{}k", tokens / 1_000)
    } else if n < 10_000_000.0 {
        format!("{:.2}m", n / 1_000_000.0)
    } else {
        format!("{:.1}m", n / 1_000_000.0)
    }
}

#[cfg(test)]
mod turn_status_tests {
    use super::{format_tokens_short, format_turn_timer};
    use std::time::Duration;

    #[test]
    fn turn_timer_matches_grok_bands_at_boundaries() {
        let cases = [
            (Duration::from_millis(0), "0.0s"),
            (Duration::from_millis(9999), "10.0s"),
            (Duration::from_secs(10), "10s"),
            (Duration::from_secs(59), "59s"),
            (Duration::from_secs(60), "1m0s"),
            (Duration::from_secs(3599), "59m59s"),
            (Duration::from_secs(3600), "1h0m"),
            (Duration::from_secs(3720), "1h2m"),
        ];
        for (duration, expected) in cases {
            assert_eq!(format_turn_timer(duration), expected, "{duration:?}");
        }
    }

    #[test]
    fn token_count_matches_grok_bands_at_boundaries() {
        let cases = [
            (0, "0"),
            (999, "999"),
            (1000, "1.00k"),
            (9999, "10.00k"),
            (10_000, "10.0k"),
            (99_999, "100.0k"),
            (100_000, "100k"),
            (999_999, "999k"),
            (1_000_000, "1.00m"),
            (9_999_999, "10.00m"),
            (10_000_000, "10.0m"),
        ];
        for (tokens, expected) in cases {
            assert_eq!(format_tokens_short(tokens), expected, "{tokens}");
        }
    }
}

// Legacy labels (deprecated)
pub const THINKING_LOADING: &str = "Thinking...";

/// The 6-frame braille spinner symbols from throbber-widgets-tui BRAILLE_SIX.
/// Index 0 → '⠷', index 5 → '⠋' (the default initial frame).
pub const BRAILLE_SIX: &[char] = &['⠷', '⠯', '⠟', '⠻', '⠽', '⠾'];

// Grok-style 8-frame braille spinner matching grok-build's braille_spinner_frames().
pub const BRAILLE_EIGHT: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧'];

/// 10-frame braille spinner for running subagent detail title animation.
pub const BRAILLE_TEN: &[char] = &['⠷', '⠯', '⠟', '⠻', '⠽', '⠾', '⠷', '⠯', '⠟', '⠻'];

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

/// Format a token count compactly (grok parity).
/// - Under 1000: `500` (raw number)
/// - 1k-100k: `1.5k`, `12.3k` (with one decimal)
/// - 100k-999k: `100k`, `500k` (whole thousands)
/// - 1M+: `1.5M`, `10.1M` (with one decimal)
pub fn format_tokens_compact(tokens: i64) -> String {
    let sign = if tokens < 0 { "-" } else { "" };
    let abs = tokens.unsigned_abs();
    if abs >= 1_000_000 {
        let m = abs as f64 / 1_000_000.0;
        format!("{sign}{}", format!("{m:.1}M").replace(".0M", "M"))
    } else if abs >= 1_000 {
        let k = abs as f64 / 1_000.0;
        format!("{sign}{}", format!("{k:.1}k").replace(".0k", "k"))
    } else {
        tokens.to_string()
    }
}

/// Format elapsed milliseconds compactly (grok parity): `5s`, `3m`, `1h`.
pub fn format_elapsed_compact(ms: u64) -> String {
    let secs = ms / 1000;
    if secs >= 3600 {
        format!("{}h", secs / 3600)
    } else if secs >= 60 {
        format!("{}m", secs / 60)
    } else {
        format!("{}s", secs)
    }
}

pub const SPINNER_THINKING: char = '◐';
