//! Design System — Single source of truth for all visual design tokens.
//!
//! Rule: no color literals, glyphs, or style presets outside this file.
//! Consumers import from here only; never hardcode values.

use ratatui::style::{Color, Style};

// ═════════════════════════════════════════════════════════════════════════════
// COLOR PALETTE
// ═════════════════════════════════════════════════════════════════════════════

/// WCAG-compliant color palette.
pub struct Colors {
    pub bg: Color,
    pub fg: Color,
    pub fg_mid: Color,
    pub fg_bright: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub dim: Color,
    pub code: Color,
    pub code_bg: Color,
}

impl Colors {
    pub const fn new() -> Self {
        Self {
            bg: Color::Rgb(12, 12, 12),
            fg: Color::Rgb(138, 138, 138),
            fg_mid: Color::Rgb(168, 168, 168),
            fg_bright: Color::Rgb(208, 208, 208),
            accent: Color::Rgb(139, 124, 244),
            success: Color::Rgb(62, 189, 106),
            warning: Color::Rgb(234, 184, 74),
            dim: Color::Rgb(74, 74, 74),
            code: Color::Rgb(180, 180, 200),
            code_bg: Color::Rgb(30, 30, 40),
        }
    }
}

impl Default for Colors {
    fn default() -> Self {
        Self::new()
    }
}

pub const C: Colors = Colors::new();

// ═════════════════════════════════════════════════════════════════════════════
// GLYPHS
// ═════════════════════════════════════════════════════════════════════════════

pub const GLYPH_USER: &str = "$ ";
pub const GLYPH_AGENT: &str = "→ ";
pub const GLYPH_TOOL: &str = "✓ ";
pub const GLYPH_INDENT: &str = "  ";
pub const GLYPH_SELECTED: &str = "▸ ";
pub const GLYPH_UNSELECTED: &str = "  ";
pub const GLYPH_THINKING: char = '◐';
pub const GLYPH_SPINNER: char = '⠋';

// Scrollbar
pub const SCROLLBAR_TRACK: &str = "│";
pub const SCROLLBAR_THUMB: &str = "█";

// Collapse indicator
pub const INDICATOR_COLLAPSED: &str = " [+]";

// Panel titles
pub const PANEL_CHAT: &str = " Chat ";
pub const PANEL_INPUT: &str = " Input ";

// ═════════════════════════════════════════════════════════════════════════════
// SEMANTIC STYLES — single source of truth for every visual element
// ═════════════════════════════════════════════════════════════════════════════

/// User message (bright, high-contrast)
pub fn style_user() -> Style {
    Style::default().fg(C.fg_bright)
}

/// Agent message — low-contrast, secondary to user content
pub fn style_agent() -> Style {
    Style::default().fg(C.fg)
}

/// Thought / reasoning — low-contrast, same whether expanded or collapsed
pub fn style_thought() -> Style {
    Style::default().fg(C.dim)
}

/// Thinking indicator — low-contrast
pub fn style_thinking() -> Style {
    Style::default().fg(C.dim)
}

/// Thought summary (collapsed) — same as expanded
pub fn style_thought_summary() -> Style {
    style_thought()
}

/// Tool running — low-contrast
pub fn style_tool_running() -> Style {
    Style::default().fg(C.dim)
}

/// Tool done header — low-contrast, same whether expanded or collapsed
pub fn style_tool_header() -> Style {
    Style::default().fg(C.dim)
}

/// Tool done output — readable but not prominent
pub fn style_tool_output() -> Style {
    Style::default().fg(C.fg)
}

/// Tool summary (collapsed) — same as expanded header
pub fn style_tool_summary() -> Style {
    style_tool_header()
}

/// Turn complete boundary marker — low-contrast
pub fn style_turn_complete() -> Style {
    Style::default().fg(C.dim)
}

/// Empty state hint — low-contrast
pub fn style_empty_state() -> Style {
    Style::default().fg(C.dim)
}

/// Timestamp suffix — low-contrast
pub fn style_timestamp() -> Style {
    Style::default().fg(C.dim)
}

/// Status bar when idle — low-contrast
pub fn style_status_idle() -> Style {
    Style::default().fg(C.dim)
}

/// Status bar when active — success green (important: work is happening)
pub fn style_status_active() -> Style {
    Style::default().fg(C.success)
}

/// Panel border — low-contrast chrome
pub fn style_border() -> Style {
    Style::default().fg(C.dim)
}

/// Panel border when flashing validation error — warning (important feedback)
pub fn style_border_flash() -> Style {
    Style::default().fg(C.warning)
}

/// Code block content — functional, kept distinct
pub fn style_code_block() -> Style {
    Style::default().fg(C.code).bg(C.code_bg)
}

/// Code block header label — low-contrast
pub fn style_code_header() -> Style {
    Style::default().fg(C.dim)
}

/// Input cursor block (inverted)
pub fn style_input_cursor() -> Style {
    Style::default().bg(C.fg_bright).fg(C.bg)
}

/// Input placeholder text — low-contrast
pub fn style_placeholder() -> Style {
    Style::default().fg(C.dim)
}

/// Bottom hints bar — low-contrast
pub fn style_hint() -> Style {
    Style::default().fg(C.dim)
}

/// @-ref popup selected item
pub fn style_popup_selected() -> Style {
    Style::default().fg(C.dim).bg(C.fg_mid)
}

/// @-ref popup unselected item
pub fn style_popup_unselected() -> Style {
    Style::default().fg(C.fg_mid)
}

/// @-ref popup border — accent for visibility
pub fn style_popup_border() -> Style {
    Style::default().fg(C.accent)
}

// ═════════════════════════════════════════════════════════════════════════════
// HELPERS
// ═════════════════════════════════════════════════════════════════════════════

/// Format a code block header label: "→ [code:rust]" or "→ [code]"
pub fn code_header_label(prefix: &str, lang: &str) -> String {
    if lang.is_empty() {
        format!("{}[code]", prefix)
    } else {
        format!("{}[code:{}]", prefix, lang)
    }
}

/// Format a thinking indicator line: "→ ◐ 1.2s"
pub fn thinking_line(elapsed_secs: f64) -> String {
    format!("{} {} {:.1}s", GLYPH_AGENT, GLYPH_THINKING, elapsed_secs)
}

/// Format a tool running line: "✓ Running name... 1.2s"
pub fn tool_running_line(name: &str, elapsed_secs: f64) -> String {
    format!("{}Running {}... {:.1}s", GLYPH_TOOL, name, elapsed_secs)
}

/// Format a tool done header: "✓ name 1.2s"
pub fn tool_done_header(name: &str, duration_secs: f64) -> String {
    format!("{}{} {:.1}s", GLYPH_TOOL, name, duration_secs)
}

/// Format a tool summary when collapsed
pub fn tool_summary_line(name: &str, duration_secs: f64) -> String {
    format!("{}{} {:.1}s{}", GLYPH_TOOL, name, duration_secs, INDICATOR_COLLAPSED)
}

/// Format a turn complete line
pub fn turn_complete_line(duration_secs: f64) -> String {
    format!("Turn completed in {:.1}s", duration_secs)
}

/// Format a thought summary when collapsed
pub fn thought_summary_line(first_line: &str) -> String {
    format!("{}{}", first_line, INDICATOR_COLLAPSED)
}
