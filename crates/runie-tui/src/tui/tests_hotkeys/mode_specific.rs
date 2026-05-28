//! Mode-specific behavior tests.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use crate::tui::state::{AppState, TuiMode, Msg};
use crate::tui::events::event_to_msg;
use super::helpers::simulate_key;

#[test]
fn test_ctrl_keys_always_quit_in_permission_mode() {
    // P0-3 FIX: Ctrl+C and Ctrl+Q in Permission mode now PRODUCE PermissionCancel,
    // not Quit. Blocking modes intercept ALL keys to prevent accidental quit.
    let state = AppState {
        mode: TuiMode::Permission,
        ..Default::default()
    };

    let event = Event::Key(KeyEvent {
        code: KeyCode::Char('c'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    });

    let msg = event_to_msg(event, &state).into_iter().next();
    assert_eq!(msg, Some(Msg::PermissionCancel), "Ctrl+C in Permission mode should cancel, not quit");
}

#[test]
fn test_enter_doesnt_submit_in_permission_mode() {
    let state = AppState {
        mode: TuiMode::Permission,
        ..Default::default()
    };

    let event = Event::Key(KeyEvent {
        code: KeyCode::Enter,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    });

    let msg = event_to_msg(event, &state).into_iter().next();
    assert_eq!(msg, Some(Msg::PermissionConfirm), "Enter in Permission should confirm, not submit");
}

#[test]
fn test_ctrl_keys_dont_work_in_palette_mode() {
    let state = AppState {
        mode: TuiMode::CommandPalette,
        command_palette: crate::tui::state::CommandPaletteState {
            open: true,
            ..Default::default()
        },
        ..Default::default()
    };

    // Ctrl+B is treated as regular 'b' char in palette mode (goes to filter)
    // This is because key_to_palette_msg doesn't check control modifiers on Char
    let event = Event::Key(KeyEvent {
        code: KeyCode::Char('b'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    });

    let msg = event_to_msg(event, &state).into_iter().next();
    assert_eq!(msg, Some(Msg::CommandPaletteFilter('b')), "Ctrl+B in CommandPalette treated as filter 'b'");
}

#[test]
fn test_arrow_keys_dont_affect_input_in_palette_mode() {
    let state = AppState {
        mode: TuiMode::CommandPalette,
        command_palette: crate::tui::state::CommandPaletteState {
            open: true,
            ..Default::default()
        },
        ..Default::default()
    };

    // Left/Right should not move cursor in palette mode
    let event = Event::Key(KeyEvent {
        code: KeyCode::Left,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    });

    let msg = event_to_msg(event, &state).into_iter().next();
    assert_eq!(msg, None, "Left in CommandPalette should be ignored");
}

#[test]
fn test_ctrl_q_always_quits_all_modes() {
    // P0-3 FIX: Ctrl+Q in Permission/Overlay mode now produces PermissionCancel/CloseModal
    // (blocking modes intercept ALL keys). In other modes, it produces Quit.
    let quit_modes = vec![
        TuiMode::Chat,
        TuiMode::CommandPalette,
        TuiMode::DiffViewer,
        TuiMode::SessionTree,
        TuiMode::Onboarding,
        TuiMode::Select,
    ];

    for mode in quit_modes {
        let msg = simulate_key(KeyCode::Char('q'), KeyModifiers::CONTROL, mode.clone());
        assert_eq!(
            msg,
            Some(Msg::Quit),
            "Ctrl+Q in {:?} mode should produce Msg::Quit",
            mode
        );
    }

    // Blocking modes intercept Ctrl+Q
    let cancel_msg = simulate_key(KeyCode::Char('q'), KeyModifiers::CONTROL, TuiMode::Permission);
    assert_eq!(cancel_msg, Some(Msg::PermissionCancel), "Ctrl+Q in Permission should cancel");
    let close_msg = simulate_key(KeyCode::Char('q'), KeyModifiers::CONTROL, TuiMode::Overlay);
    assert_eq!(close_msg, Some(Msg::CloseModal), "Ctrl+Q in Overlay should close");
}

#[test]
fn test_ctrl_c_always_quits_all_modes() {
    // P0-3 FIX: Ctrl+C in Permission mode produces PermissionCancel (not Quit).
    // In Chat/Onboarding, it still produces Quit (empty input) or ClearInput.
    let modes = vec![
        TuiMode::Chat,
        TuiMode::CommandPalette,
        TuiMode::Onboarding,
    ];

    for mode in modes {
        let msg = simulate_key(KeyCode::Char('c'), KeyModifiers::CONTROL, mode.clone());
        // In these modes, Ctrl+C produces Quit (empty textarea) or ClearInput
        assert!(
            matches!(msg, Some(Msg::Quit) | Some(Msg::ClearInput)),
            "Ctrl+C in {:?} mode should produce Msg::Quit or Msg::ClearInput",
            mode
        );
    }

    // Permission mode intercepts Ctrl+C
    let cancel_msg = simulate_key(KeyCode::Char('c'), KeyModifiers::CONTROL, TuiMode::Permission);
    assert_eq!(cancel_msg, Some(Msg::PermissionCancel), "Ctrl+C in Permission should cancel");
}
