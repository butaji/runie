use ratatui::style::{Color, Style};

pub const C: Colors = Colors::new();

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
    fn default() -> Self { Self::new() }
}

#[macro_export]
macro_rules! style {
    ($fg:ident) => { Style::default().fg($crate::theme::C.$fg) };
    (fg: $fg:ident, bg: $bg:ident) => { Style::default().fg($crate::theme::C.$fg).bg($crate::theme::C.$bg) };
    (bg: $bg:ident, fg: $fg:ident) => { Style::default().bg($crate::theme::C.$bg).fg($crate::theme::C.$fg) };
}

pub use style;

pub const GLYPH_USER: &str = "$ ";
pub const GLYPH_AGENT: &str = "→ ";
pub const GLYPH_TOOL: &str = "✓ ";
pub const GLYPH_INDENT: &str = "  ";
pub const GLYPH_SELECTED: &str = "▸ ";
pub const GLYPH_UNSELECTED: &str = "  ";
pub const GLYPH_THINKING: char = '◐';
pub const GLYPH_SPINNER: char = '⠋';
pub const SCROLLBAR_TRACK: &str = "│";
pub const SCROLLBAR_THUMB: &str = "█";
pub const INDICATOR_COLLAPSED: &str = " [+]";
pub const PANEL_CHAT: &str = " Chat ";
pub const PANEL_INPUT: &str = " Input ";

pub fn style_user()        -> Style { style!(fg_bright) }
pub fn style_agent()       -> Style { style!(fg) }
pub fn style_thought()     -> Style { style!(dim) }
pub fn style_thinking()    -> Style { style!(dim) }
pub fn style_tool_running()-> Style { style!(dim) }
pub fn style_tool_header() -> Style { style!(dim) }
pub fn style_tool_output() -> Style { style!(fg) }
pub fn style_turn_complete()-> Style { style!(dim) }
pub fn style_empty_state() -> Style { style!(dim) }
pub fn style_timestamp()   -> Style { style!(dim) }
pub fn style_status_idle() -> Style { style!(dim) }
pub fn style_status_active()-> Style { style!(success) }
pub fn style_border()      -> Style { style!(dim) }
pub fn style_border_flash()-> Style { style!(warning) }
pub fn style_code_block()  -> Style { style!(fg: code, bg: code_bg) }
pub fn style_code_header() -> Style { style!(dim) }
pub fn style_input_cursor()-> Style { style!(bg: fg_bright, fg: bg) }
pub fn style_placeholder() -> Style { style!(dim) }
pub fn style_hint()        -> Style { style!(dim) }
pub fn style_popup_selected()   -> Style { style!(fg: dim, bg: fg_mid) }
pub fn style_popup_unselected() -> Style { style!(fg_mid) }
pub fn style_popup_border()     -> Style { style!(accent) }
pub fn style_thought_summary()  -> Style { style_thought() }
pub fn style_tool_summary()     -> Style { style_tool_header() }

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
    format!("{}{} {:.1}s{}", GLYPH_TOOL, name, duration_secs, INDICATOR_COLLAPSED)
}

pub fn turn_complete_line(duration_secs: f64) -> String {
    format!("Turn completed in {:.1}s", duration_secs)
}

pub fn thought_summary_line(first_line: &str) -> String {
    format!("{}{}", first_line, INDICATOR_COLLAPSED)
}
