use super::{default_bindings, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[test]
fn ctrl_shift_e_is_unbound() {
    let key = KeyEvent::new(
        KeyCode::Char('E'),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    );
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert_eq!(
        result, None,
        "Ctrl+Shift+E should be unbound, got {:?}",
        result
    );
}

#[test]
fn ctrl_shift_e_lowercase_is_unbound_for_tmux() {
    let key = KeyEvent::new(
        KeyCode::Char('e'),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    );
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert_eq!(
        result, None,
        "Ctrl+Shift+E (lowercase) should be unbound, got {:?}",
        result
    );
}

#[test]
fn ctrl_shift_o_converts_to_copy_last_response() {
    let key = KeyEvent::new(
        KeyCode::Char('O'),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    );
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::CopyLastResponse)),
        "Ctrl+Shift+O should map to CopyLastResponse, got {:?}",
        result
    );
}

#[test]
fn ctrl_shift_e_on_repeat_kind_is_unbound() {
    let key = KeyEvent::new_with_kind(
        KeyCode::Char('E'),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        KeyEventKind::Repeat,
    );
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert_eq!(
        result, None,
        "Ctrl+Shift+E with Repeat kind should be unbound, got {:?}",
        result
    );
}

#[test]
fn shift_ctrl_p_emits_toggle_command_palette() {
    let key = KeyEvent::new(
        KeyCode::Char('P'),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    );
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::ToggleCommandPalette)),
        "Shift+Ctrl+P should map to ToggleCommandPalette, got {:?}",
        result
    );
}

#[test]
fn ctrl_m_emits_cycle_model_next() {
    let key = KeyEvent::new(KeyCode::Char('m'), KeyModifiers::CONTROL);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::CycleModelNext)),
        "Ctrl+M should map to CycleModelNext, got {:?}",
        result
    );
}

#[test]
fn shift_ctrl_m_emits_cycle_model_prev() {
    let key = KeyEvent::new(
        KeyCode::Char('M'),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    );
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::CycleModelPrev)),
        "Shift+Ctrl+M should map to CycleModelPrev, got {:?}",
        result
    );
}

#[test]
fn shift_exclamation_converts_to_input_exclamation() {
    let key = KeyEvent::new(KeyCode::Char('!'), KeyModifiers::SHIFT);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Input('!'))),
        "Shift+! should map to Input('!'), got {:?}",
        result
    );
}

#[test]
fn shift_question_converts_to_input_question() {
    let key = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::SHIFT);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Input('?'))),
        "Shift+? should map to Input('?'), got {:?}",
        result
    );
}

#[test]
fn shift_parenthesis_open_converts_to_input() {
    let key = KeyEvent::new(KeyCode::Char('('), KeyModifiers::SHIFT);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Input('('))),
        "Shift+( should map to Input('('), got {:?}",
        result
    );
}

#[test]
fn shift_at_converts_to_input_at() {
    let key = KeyEvent::new(KeyCode::Char('@'), KeyModifiers::SHIFT);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Input('@'))),
        "Shift+@ should map to Input('@'), got {:?}",
        result
    );
}

#[test]
fn shift_enter_converts_to_newline() {
    let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Newline)),
        "Shift+Enter should map to Newline, got {:?}",
        result
    );
}

#[test]
fn shift_f3_converts_to_newline_for_tmux_shift_enter() {
    let key = KeyEvent::new(KeyCode::F(3), KeyModifiers::SHIFT);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Newline)),
        "Shift+F3 should map to Newline for tmux Shift+Enter, got {:?}",
        result
    );
}

#[test]
fn f3_without_shift_converts_to_newline_for_tmux_compat() {
    let key = KeyEvent::new(KeyCode::F(3), KeyModifiers::empty());
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Newline)),
        "F3 without shift should map to Newline for tmux compat, got {:?}",
        result
    );
}
