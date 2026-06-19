// Re-export from core so the layout helper and renderer always agree on
// prefix widths.
pub use runie_core::layout::{GLYPH_AGENT, GLYPH_INDENT, GLYPH_USER};

pub const GLYPH_TOOL: &str = "✓ ";
pub const GLYPH_SELECTED: &str = "▸ ";
pub const GLYPH_UNSELECTED: &str = "  ";
pub const GLYPH_THINKING: char = '◐';
pub const GLYPH_SPINNER: char = '⠋';
pub const SCROLLBAR_TRACK: &str = " "; // invisible track
pub const SCROLLBAR_THUMB: &str = "▐"; // right half-block — visible but not heavy
pub const INDICATOR_COLLAPSED: &str = " [+]";
pub const PANEL_CHAT: &str = " Chat ";
pub const PANEL_INPUT: &str = " Input ";

pub fn code_header_label(prefix: &str, lang: &str) -> String {
    if lang.is_empty() {
        format!("{}[code]", prefix)
    } else {
        format!("{}[code:{}]", prefix, lang)
    }
}

pub fn thinking_line(elapsed_secs: f64) -> String {
    format!("{} {} {:.1}s", GLYPH_AGENT, GLYPH_THINKING, elapsed_secs)
}

pub fn tool_running_line(name: &str, elapsed_secs: f64) -> String {
    format!("{}Running {}... {:.1}s", GLYPH_TOOL, name, elapsed_secs)
}

pub fn tool_done_header(name: &str, duration_secs: f64) -> String {
    format!("{}{} {:.1}s", GLYPH_TOOL, name, duration_secs)
}

pub fn tool_summary_line(name: &str, duration_secs: f64) -> String {
    format!(
        "{}{} {:.1}s{}",
        GLYPH_TOOL, name, duration_secs, INDICATOR_COLLAPSED
    )
}

pub fn turn_complete_line(duration_secs: f64) -> String {
    format!("Turn completed in {:.1}s", duration_secs)
}

pub fn thought_summary_line(first_line: &str) -> String {
    format!("{}{}", first_line, INDICATOR_COLLAPSED)
}
