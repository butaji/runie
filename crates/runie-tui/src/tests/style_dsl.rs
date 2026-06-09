use ratatui::style::Style;
use crate::theme::{set_current_theme, style_user, style_code_block, style_input_cursor};

fn setup() {
    set_current_theme("silkcircuit-neon");
}

#[test]
fn style_user_has_fg() {
    setup();
    let s: Style = style_user();
    assert!(s.fg.is_some(), "style_user should have a foreground color");
}

#[test]
fn style_code_block_has_fg_and_bg() {
    setup();
    let s: Style = style_code_block();
    assert!(s.fg.is_some(), "style_code_block should have a foreground color");
    assert!(s.bg.is_some(), "style_code_block should have a background color");
}

#[test]
fn style_input_cursor_is_reversible() {
    setup();
    let s: Style = style_input_cursor();
    assert!(s.fg.is_some(), "style_input_cursor should have a foreground color");
    assert!(s.bg.is_some(), "style_input_cursor should have a background color");
}

#[test]
fn all_style_functions_exist() {
    setup();
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
