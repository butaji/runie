use super::{default_bindings, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use runie_core::event::{ControlEvent, DialogEvent, InputEvent};

#[test]
fn ctrl_o_converts_to_toggle_expand() {
    let key = KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Control(ControlEvent::ToggleExpand))),
        "Ctrl+O should map to ToggleExpand, got {:?}",
        result
    );
}

#[test]
fn ctrl_o_toggles_expand_state() {
    use runie_core::{AppState, ChatMessage, Role};

    let mut state = AppState::default();
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        content: "deep reasoning\nsecond line".to_string(),
        timestamp: 1.0,
        id: "t1".to_string(),
        ..Default::default()
    });
    state.messages_changed();
    state.ensure_fresh();

    let before = state.view.all_collapsed;

    let key = KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL);
    let event = crossterm::event::Event::Key(key);
    let core_event = crate::keymap::convert_event(&event, &default_bindings());
    assert!(matches!(core_event, Some(runie_core::Event::Control(ControlEvent::ToggleExpand))));

    state.update(core_event.unwrap());
    assert_ne!(
        state.view.all_collapsed, before,
        "Ctrl+O should toggle the global collapsed state"
    );
}

#[test]
fn ctrl_e_converts_to_cursor_end() {
    let key = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Input(InputEvent::CursorEnd))),
        "Ctrl+E should map to CursorEnd, got {:?}",
        result
    );
}

#[test]
fn ctrl_c_converts_to_quit() {
    let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Control(ControlEvent::Quit))),
        "Ctrl+C should map to Quit"
    );
}

#[test]
fn plain_e_not_converted() {
    let key = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::empty());
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Input(InputEvent::Input('e')))),
        "Plain e should map to Input"
    );
}

#[test]
fn ctrl_e_does_not_conflict_with_quit() {
    let key = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        !matches!(result, Some(runie_core::Event::Control(ControlEvent::Quit))),
        "Ctrl+E should NOT map to Quit"
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
        matches!(result, Some(runie_core::Event::Input(InputEvent::CursorEnd))),
        "Ctrl+E with Repeat kind should still map to CursorEnd, got {:?}",
        result
    );
}

#[test]
fn ctrl_q_converts_to_quit() {
    let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Control(ControlEvent::Quit))),
        "Ctrl+Q should map to Quit, got {:?}",
        result
    );
}

#[test]
fn ctrl_z_converts_to_suspend() {
    let key = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Control(ControlEvent::Suspend))),
        "Ctrl+Z should map to Suspend"
    );
}

#[test]
fn ctrl_y_converts_to_redo() {
    let key = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Input(InputEvent::Redo))),
        "Ctrl+Y should map to Redo"
    );
}

#[test]
fn alt_b_converts_to_word_left() {
    let key = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::ALT);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Input(InputEvent::CursorWordLeft))),
        "Alt+B should map to CursorWordLeft"
    );
}

#[test]
fn alt_f_converts_to_word_right() {
    let key = KeyEvent::new(KeyCode::Char('f'), KeyModifiers::ALT);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Input(InputEvent::CursorWordRight))),
        "Alt+F should map to CursorWordRight"
    );
}

#[test]
fn bracketed_paste_converts_to_paste_event() {
    let event = crossterm::event::Event::Paste("hello world".to_string());
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Input(InputEvent::Paste(s))) if s == "hello world"),
        "Paste event should map to CoreEvent::Input(InputEvent::Paste)"
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
        matches!(result, Some(runie_core::Event::Control(ControlEvent::Abort))),
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
fn alt_up_emits_dequeue() {
    let key = KeyEvent::new(KeyCode::Up, KeyModifiers::ALT);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Control(ControlEvent::Dequeue))),
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
        matches!(result, Some(runie_core::Event::Control(ControlEvent::OpenExternalEditor))),
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
        matches!(result, Some(runie_core::Event::Dialog(DialogEvent::ToggleModelSelector))),
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
        matches!(result, Some(runie_core::Event::Dialog(DialogEvent::ToggleCommandPalette))),
        "Ctrl+P should map to ToggleCommandPalette, got {:?}",
        result
    );
}

#[test]
fn plain_escape_emits_dialog_back() {
    let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Dialog(DialogEvent::DialogBack))),
        "Esc should map to DialogBack so the core can decide abort vs. vim nav"
    );
}

#[test]
fn plain_space_emits_input_space() {
    let key = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::empty());
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Input(InputEvent::Input(' ')))),
        "Space should map to Input(InputEvent::Input(' ')), got {:?}",
        result
    );
}

#[test]
fn ctrl_j_converts_to_newline() {
    let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Input(InputEvent::Newline))),
        "Ctrl+J should map to Newline, got {:?}",
        result
    );
}

#[test]
fn ctrl_j_as_lf_converts_to_newline() {
    let key = KeyEvent::new(KeyCode::Char('\n'), KeyModifiers::empty());
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        matches!(result, Some(runie_core::Event::Input(InputEvent::Newline))),
        "LF char should map to Newline, got {:?}",
        result
    );
}

#[test]
fn shift_up_converts_to_history_prev() {
    let key = KeyEvent::new(KeyCode::Up, KeyModifiers::SHIFT);
    let event = crossterm::event::Event::Key(key);
    let result = crate::keymap::convert_event(&event, &default_bindings());
    assert!(
        result.is_none(),
        "Shift+Up should not map (falls through), got {:?}",
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
        matches!(result, Some(runie_core::Event::Input(InputEvent::PasteImage))),
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
        matches!(result, Some(runie_core::Event::Input(InputEvent::PasteImage))),
        "Alt+V should map to PasteImage, got {:?}",
        result
    );
}
