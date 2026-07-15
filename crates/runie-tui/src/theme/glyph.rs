// Re-export from core so the layout helper and renderer always agree on
// prefix widths and the feed indent.
pub use runie_core::layout::{FEED_INDENT, GLYPH_AGENT, GLYPH_INDENT, GLYPH_USER};

// Checkbox glyphs
pub const GLYPH_CHECKED: &str = "[x]";
pub const GLYPH_UNCHECKED: &str = "[ ]";
pub const GLYPH_CHECK: &str = "✓";
pub const GLYPH_X: &str = "✗";

// Arrow / selection glyphs
pub const GLYPH_SELECTED: &str = "▸ ";
pub const GLYPH_UNSELECTED: &str = "  ";
pub const GLYPH_THINKING: char = '◐';
pub const GLYPH_FILTER: char = '❯'; // filter input prompt indicator

// Tool / status glyphs
pub const GLYPH_TOOL: &str = "◆ ";
pub const GLYPH_BULLET: &str = "•";
pub const GLYPH_DOWNLOAD: &str = "⇣"; // bytes transferred indicator

// Sub-agent lifecycle row glyphs (GROK.md §26).
pub const GLYPH_SUBAGENT_BAR: &str = "❙"; // running left bar
pub const GLYPH_SUBAGENT_DIAMOND: &str = "◆"; // state diamond
pub const GLYPH_SUBAGENT_QUOTE_LEFT: &str = "“";
pub const GLYPH_SUBAGENT_QUOTE_RIGHT: &str = "”";

// Spinner and indicator glyphs
// throbber BRAILLE_SIX[5] = '⠋' — first frame of the braille spinner.
pub const GLYPH_SPINNER: char = '⠋';
pub const INDICATOR_COLLAPSED: &str = " [+]";
pub const INDICATOR_ERROR: &str = " [✗]";

// Box drawing glyphs
pub const BOX_HORIZONTAL: char = '─'; // horizontal line
pub const BOX_VERTICAL: char = '│'; // vertical line
pub const BOX_TOP_LEFT: &str = "┌";
pub const BOX_TOP_RIGHT: &str = "┐";
pub const BOX_BOTTOM_LEFT: &str = "└";
pub const BOX_BOTTOM_RIGHT: &str = "┘";

// Scrollbar glyphs
pub const SCROLLBAR_TRACK: &str = " "; // invisible track
pub const SCROLLBAR_THUMB: &str = "▐"; // right half-block — visible but not heavy

// Panel headers
pub const PANEL_CHAT: &str = " Chat ";
pub const PANEL_INPUT: &str = " Input ";

pub fn code_header_label(prefix: &str, lang: &str) -> String {
    if lang.is_empty() {
        format!("{}[code]", prefix)
    } else {
        format!("{}[code:{}]", prefix, lang)
    }
}

/// Thinking/waiting indicator line (grok parity — GROK.md §24).
///
/// `◆ ⠋ Waiting for response… 0.4s` — the braille frame is derived from the
/// elapsed wall time (~120ms per frame), so the row animates at a steady
/// cadence regardless of render rate. Timer: one decimal below 10s, integer
/// at ≥10s.
pub fn thinking_line(elapsed_secs: f64) -> String {
    use runie_core::labels::{format_elapsed_secs, BRAILLE_SIX};
    const FRAME_MS: f64 = 120.0;
    let idx = ((elapsed_secs * 1000.0 / FRAME_MS) as usize) % BRAILLE_SIX.len();
    format!(
        "{}{} Waiting for response… {}",
        GLYPH_AGENT,
        BRAILLE_SIX[idx],
        format_elapsed_secs(elapsed_secs)
    )
}

/// Tool running line.
pub fn tool_running_line(name: &str, elapsed_secs: f64) -> String {
    format!("{}Running {}... {:.1}s", GLYPH_TOOL, name, elapsed_secs)
}

/// Tool done header.
pub fn tool_done_header(name: &str, duration_secs: f64) -> String {
    format!("{}{} {:.1}s", GLYPH_TOOL, name, duration_secs)
}

/// Tool summary line.
pub fn tool_summary_line(name: &str, duration_secs: f64) -> String {
    format!("{}{} {:.1}s", GLYPH_TOOL, name, duration_secs)
}

/// Turn complete line.
pub fn turn_complete_line(duration_secs: f64) -> String {
    format!("Turn completed in {:.1}s.", duration_secs)
}

/// Thought summary line.
pub fn thought_summary_line(first_line: &str) -> String {
    first_line.to_string()
}
