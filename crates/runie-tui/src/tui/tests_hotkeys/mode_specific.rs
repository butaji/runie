//! Mode-specific behavior tests.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use crate::tui::state::{AppState, TuiMode, Msg};
use crate::tui::update::update;
use crate::tui::events::event_to_msg;
use super::helpers::simulate_key;

#[test]
fn test_ctrl_keys_always_quit_in_permission_mode() {
    // Ctrl+C and Ctrl+Q are GLOBAL hotkeys that always produce Quit
    // regardless of the current mode (including Permission)
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
    assert_eq!(msg, Some(Msg::Quit), "Ctrl+C in Permission mode should always produce Quit");
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
    // Ctrl+Q must produce Quit in EVERY mode
    let modes = vec![
        TuiMode::Chat,
        TuiMode::Permission,
        TuiMode::CommandPalette,
        TuiMode::DiffViewer,
        TuiMode::SessionTree,
        TuiMode::Onboarding,
        TuiMode::Overlay,
        TuiMode::Select,
    ];

    for mode in modes {
        let msg = simulate_key(KeyCode::Char('q'), KeyModifiers::CONTROL, mode.clone());
        assert_eq!(
            msg,
            Some(Msg::Quit),
            "Ctrl+Q in {:?} mode should always produce Msg::Quit",
            mode
        );
    }
}

#[test]
fn test_ctrl_c_always_quits_all_modes() {
    // Ctrl+C must also produce Quit in EVERY mode (same as Ctrl+Q)
    let modes = vec![
        TuiMode::Chat,
        TuiMode::Permission,
        TuiMode::CommandPalette,
        TuiMode::Onboarding,
    ];

    for mode in modes {
        let msg = simulate_key(KeyCode::Char('c'), KeyModifiers::CONTROL, mode.clone());
        assert_eq!(
            msg,
            Some(Msg::Quit),
            "Ctrl+C in {:?} mode should always produce Msg::Quit",
            mode
        );
    }
}
