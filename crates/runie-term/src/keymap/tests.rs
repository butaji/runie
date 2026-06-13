use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

fn default_bindings() -> std::collections::HashMap<String, String> {
    runie_core::keybindings::default_keybindings()
}

#[test]
fn ctrl_shift_e_converts_to_toggle_expand() {
    let key = KeyEvent::new(
        KeyCode::Char('E'),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    );
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::ToggleExpand)),
        "Ctrl+Shift+E should map to ToggleExpand, got {:?}",
        result
    );
}

#[test]
fn ctrl_e_converts_to_toggle_expand_for_terminals_without_shift() {
    let key = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::ToggleExpand)),
        "Ctrl+E should map to ToggleExpand, got {:?}",
        result
    );
}

#[test]
fn ctrl_c_converts_to_quit() {
    let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Quit)),
        "Ctrl+C should map to Quit"
    );
}

#[test]
fn plain_e_not_converted() {
    let key = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::empty());
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Input('e'))),
        "Plain e should map to Input"
    );
}

#[test]
fn ctrl_e_does_not_conflict_with_quit() {
    let key = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        !matches!(result, Some(runie_core::Event::Quit)),
        "Ctrl+E should NOT map to Quit"
    );
}

#[test]
fn ctrl_shift_e_on_repeat_kind_still_works() {
    let key = KeyEvent::new_with_kind(
        KeyCode::Char('E'),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        KeyEventKind::Repeat,
    );
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::ToggleExpand)),
        "Ctrl+Shift+E with Repeat kind should still map to ToggleExpand, got {:?}",
        result
    );
}

#[test]
fn ctrl_e_on_repeat_kind_still_works() {
    let key = KeyEvent::new_with_kind(
        KeyCode::Char('e'),
        KeyModifiers::CONTROL,
        KeyEventKind::Repeat,
    );
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::ToggleExpand)),
        "Ctrl+E with Repeat kind should still map to ToggleExpand, got {:?}",
        result
    );
}

#[test]
fn ctrl_z_converts_to_suspend() {
    let key = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Suspend)),
        "Ctrl+Z should map to Suspend"
    );
}

#[test]
fn ctrl_y_converts_to_redo() {
    let key = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Redo)),
        "Ctrl+Y should map to Redo"
    );
}

#[test]
fn alt_b_converts_to_word_left() {
    let key = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::ALT);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::CursorWordLeft)),
        "Alt+B should map to CursorWordLeft"
    );
}

#[test]
fn alt_f_converts_to_word_right() {
    let key = KeyEvent::new(KeyCode::Char('f'), KeyModifiers::ALT);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::CursorWordRight)),
        "Alt+F should map to CursorWordRight"
    );
}

#[test]
fn bracketed_paste_converts_to_paste_event() {
    let event = crossterm::event::Event::Paste("hello world".to_string());
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Paste(s)) if s == "hello world"),
        "Paste event should map to CoreEvent::Paste"
    );
}

#[test]
fn custom_keybinding_overrides_default() {
    let mut bindings = default_bindings();
    bindings.insert("ctrl+c".to_string(), "Abort".to_string());
    let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &bindings);
    assert!(
        matches!(result, Some(runie_core::Event::Abort)),
        "Custom keybinding should override default"
    );
}

#[test]
fn unknown_keybinding_falls_back_to_default() {
    let bindings = default_bindings();
    let key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &bindings);
    assert_eq!(result, None, "Unmapped key should fall through to None");
}

#[test]
fn key_event_to_combo_ctrl_c() {
    let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(crate::keymap::key_event_to_combo(&key), "ctrl+c");
}

#[test]
fn key_event_to_combo_alt_enter() {
    let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT);
    assert_eq!(crate::keymap::key_event_to_combo(&key), "alt+enter");
}

#[test]
fn alt_up_emits_dequeue() {
    let key = KeyEvent::new(KeyCode::Up, KeyModifiers::ALT);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Dequeue)),
        "Alt+Up should map to Dequeue, got {:?}",
        result
    );
}

#[test]
fn ctrl_g_emits_open_external_editor() {
    let key = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::OpenExternalEditor)),
        "Ctrl+G should map to OpenExternalEditor, got {:?}",
        result
    );
}

#[test]
fn ctrl_l_emits_toggle_model_selector() {
    let key = KeyEvent::new(KeyCode::Char('l'), KeyModifiers::CONTROL);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::ToggleModelSelector)),
        "Ctrl+L should map to ToggleModelSelector, got {:?}",
        result
    );
}

#[test]
fn ctrl_p_emits_toggle_command_palette() {
    let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::ToggleCommandPalette)),
        "Ctrl+P should map to ToggleCommandPalette, got {:?}",
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
fn key_event_to_combo_shift_enter() {
    let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT);
    assert_eq!(crate::keymap::key_event_to_combo(&key), "shift+enter");
}

#[test]
fn key_event_to_combo_plain_escape() {
    let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
    assert_eq!(crate::keymap::key_event_to_combo(&key), "escape");
}

#[test]
fn plain_escape_emits_dialog_back() {
    let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::DialogBack)),
        "Esc should map to DialogBack so the core can decide abort vs. vim nav"
    );
}

#[test]
fn plain_space_emits_input_space() {
    let key = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::empty());
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Input(' '))),
        "Space should map to Input(' '), got {:?}",
        result
    );
}

#[test]
#[cfg(not(target_os = "windows"))]
fn ctrl_v_emits_paste_image() {
    let key = KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::PasteImage)),
        "Ctrl+V should map to PasteImage, got {:?}",
        result
    );
}

#[test]
#[cfg(target_os = "windows")]
fn alt_v_emits_paste_image() {
    let key = KeyEvent::new(KeyCode::Char('v'), KeyModifiers::ALT);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::PasteImage)),
        "Alt+V should map to PasteImage, got {:?}",
        result
    );
}

// ─── Shift+symbol input ─────────────────────────────────────────────────────

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
    // tmux sends \e[13;2~ for Shift+Enter, which crossterm interprets as F(3)+SHIFT
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
    // Some crossterm versions report tmux Shift+Enter (\e[13;2~) as F(3)
    // without the SHIFT modifier bit set. F(3) should still map to Newline.
    let key = KeyEvent::new(KeyCode::F(3), KeyModifiers::empty());
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Newline)),
        "F3 without shift should map to Newline for tmux compat, got {:?}",
        result
    );
}

#[test]
fn ctrl_j_converts_to_newline() {
    let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Newline)),
        "Ctrl+J should map to Newline, got {:?}",
        result
    );
}

#[test]
fn ctrl_j_as_lf_converts_to_newline() {
    // Some terminals send Ctrl+J as just LF (\n) without CONTROL modifier
    let key = KeyEvent::new(KeyCode::Char('\n'), KeyModifiers::empty());
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Newline)),
        "LF char should map to Newline, got {:?}",
        result
    );
}

#[test]
fn shift_up_converts_to_history_prev() {
    let key = KeyEvent::new(KeyCode::Up, KeyModifiers::SHIFT);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    // Shift+arrow is not a binding, falls through to None
    assert!(
        result.is_none(),
        "Shift+Up should not map (falls through), got {:?}",
        result
    );
}
