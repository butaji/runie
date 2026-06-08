use ratatui::style::{Color, Style};
use crate::theme::{style, C};

#[test]
fn style_macro_fg_only() {
    let s: Style = style!(fg_bright);
    assert_eq!(s.fg, Some(C.fg_bright));
    assert_eq!(s.bg, None);
}

#[test]
fn style_macro_fg_and_bg() {
    let s: Style = style!(fg: code, bg: code_bg);
    assert_eq!(s.fg, Some(C.code));
    assert_eq!(s.bg, Some(C.code_bg));
}

#[test]
fn style_macro_reversible() {
    let s: Style = style!(bg: fg_bright, fg: bg);
    assert_eq!(s.fg, Some(C.bg));
    assert_eq!(s.bg, Some(C.fg_bright));
}

#[test]
fn all_style_functions_exist() {
    use crate::theme::*;
    let _ = style_user();
    let _ = style_agent();
    let _ = style_thought();
    let _ = style_thinking();
    let _ = style_tool_header();
    let _ = style_tool_output();
    let _ = style_tool_running();
    let _ = style_tool_summary();
    let _ = style_turn_complete();
    let _ = style_empty_state();
    let _ = style_timestamp();
    let _ = style_status_idle();
    let _ = style_status_active();
    let _ = style_border();
    let _ = style_border_flash();
    let _ = style_code_block();
    let _ = style_code_header();
    let _ = style_input_cursor();
    let _ = style_placeholder();
    let _ = style_hint();
    let _ = style_popup_selected();
    let _ = style_popup_unselected();
    let _ = style_popup_border();
}
