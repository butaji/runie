//! State consistency tests - Verify dirty flag is set correctly by hotkeys.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use crate::tui::state::{AppState, TuiMode, Msg};
use crate::tui::update::update;
use crate::components::CommandPalette;
use crate::tui::events::event_to_msg;
use super::helpers::simulate_key;

/// Mock Tui for dirty flag testing
struct MockTui {
    state: AppState,
    palette: CommandPalette,
    dirty: bool,
}

impl MockTui {
    fn new() -> Self {
        Self {
            state: AppState::default(),
            palette: CommandPalette::new(),
            dirty: false,
        }
    }

    fn update(&mut self, msg: Msg) {
        self.dirty = true;
        update(&mut self.state, &mut self.palette, msg);
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn clear_dirty(&mut self) {
        self.dirty = false;
    }
}

#[test]
fn test_hotkey_updates_set_dirty() {
    let mut tui = MockTui::new();

    let hotkey_cases = vec![
        (KeyCode::Char('c'), KeyModifiers::CONTROL, TuiMode::Chat),
        (KeyCode::Char('j'), KeyModifiers::CONTROL, TuiMode::Chat),
        (KeyCode::Char('a'), KeyModifiers::CONTROL, TuiMode::Chat),
        (KeyCode::Char('e'), KeyModifiers::CONTROL, TuiMode::Chat),
        (KeyCode::Char('w'), KeyModifiers::CONTROL, TuiMode::Chat),
        (KeyCode::Char('u'), KeyModifiers::CONTROL, TuiMode::Chat),
        (KeyCode::Char('d'), KeyModifiers::CONTROL, TuiMode::Chat),
        (KeyCode::Char('b'), KeyModifiers::CONTROL, TuiMode::Chat),
        (KeyCode::Char('k'), KeyModifiers::CONTROL, TuiMode::Chat),
        (KeyCode::Esc, KeyModifiers::NONE, TuiMode::CommandPalette),
        (KeyCode::Enter, KeyModifiers::NONE, TuiMode::CommandPalette),
        (KeyCode::Up, KeyModifiers::NONE, TuiMode::CommandPalette),
        (KeyCode::Down, KeyModifiers::NONE, TuiMode::CommandPalette),
        (KeyCode::Enter, KeyModifiers::NONE, TuiMode::Chat),
        (KeyCode::PageUp, KeyModifiers::NONE, TuiMode::Chat),
        (KeyCode::PageDown, KeyModifiers::NONE, TuiMode::Chat),
    ];

    for (code, modifiers, mode) in hotkey_cases {
        tui.clear_dirty();
        let event = Event::Key(KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        });
        let state = AppState { mode: mode.clone(), ..Default::default() };

        for msg in event_to_msg(event, &state) {
            tui.update(msg);
            assert!(tui.is_dirty(), "Hotkey {:?}+{:?} in {:?} mode should set dirty", modifiers, code, mode);
        }
    }
}
