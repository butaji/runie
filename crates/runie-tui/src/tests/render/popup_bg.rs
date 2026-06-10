use runie_core::{AppState, Event, ChatMessage, Role};
use crate::ui::view;
use ratatui::{backend::TestBackend, Terminal};

/// Helper: check if the given text string appears anywhere in the rect.
fn rect_contains_text(buf: &ratatui::buffer::Buffer, rect: ratatui::layout::Rect, text: &str) -> bool {
    for y in rect.y..rect.y + rect.height {
        let line: String = (rect.x..rect.x + rect.width)
            .map(|x| buf[(x, y)].symbol())
            .collect();
        if line.contains(text) {
            return true;
        }
    }
    false
}

/// Popup background must hide underlying message content.
/// Bug: command palette was transparent, showing messages through it.
#[test]
fn command_palette_hides_underlying_messages() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();

    // Add a visible message that would show through if popup is transparent
    state.session.messages.push(ChatMessage {
        role: Role::User,
        content: "XYZZY_PLUGH".into(),
        timestamp: 0.0,
        id: "u0".into(),
        ..Default::default()
    });
    state.messages_changed();

    // Open command palette
    state.update(Event::ToggleCommandPalette);

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    // The popup area should NOT contain the underlying message text
    let popup_rect = ratatui::layout::Rect {
        x: 10, y: 3, width: 60, height: 18,
    };
    assert!(
        !rect_contains_text(buf, popup_rect, "XYZZY"),
        "Popup should hide underlying 'XYZZY_PLUGH' — transparent background bug"
    );
}

/// Settings dialog must also hide underlying content.
#[test]
fn settings_dialog_hides_underlying_messages() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();

    state.session.messages.push(ChatMessage {
        role: Role::User,
        content: "XYZZY_PLUGH".into(),
        timestamp: 0.0,
        id: "u0".into(),
        ..Default::default()
    });
    state.messages_changed();

    state.update(Event::ToggleSettingsDialog);

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    let popup_rect = ratatui::layout::Rect {
        x: 10, y: 3, width: 60, height: 18,
    };
    assert!(
        !rect_contains_text(buf, popup_rect, "XYZZY"),
        "Settings dialog should hide underlying content"
    );
}

/// Model selector dialog must also hide underlying content.
#[test]
fn model_selector_hides_underlying_messages() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();

    state.session.messages.push(ChatMessage {
        role: Role::User,
        content: "XYZZY_PLUGH".into(),
        timestamp: 0.0,
        id: "u0".into(),
        ..Default::default()
    });
    state.messages_changed();

    state.update(Event::ToggleModelSelector);

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    let popup_rect = ratatui::layout::Rect {
        x: 10, y: 3, width: 60, height: 18,
    };
    assert!(
        !rect_contains_text(buf, popup_rect, "XYZZY"),
        "Model selector should hide underlying content"
    );
}

/// Popup area must have panel background color, not app background.
#[test]
fn command_palette_has_panel_background_color() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.session.messages.push(ChatMessage {
        role: Role::User,
        content: "hello".into(),
        timestamp: 0.0,
        id: "u0".into(),
        ..Default::default()
    });
    state.messages_changed();
    state.update(Event::ToggleCommandPalette);

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    let panel_bg = crate::theme::color_bg_panel();
    let app_bg = crate::theme::color_bg();

    // Find the popup inner area by looking for the "Commands" title
    let mut found_title = false;
    let mut popup_inner_y = 0;
    let mut popup_inner_x_start = 0;

    for y in 0..buf.area().height {
        for x in 0..buf.area().width.saturating_sub(9) {
            let s: String = (x..x+9).map(|cx| buf[(cx, y)].symbol()).collect();
            if s == " Commands" {
                found_title = true;
                popup_inner_y = y + 1; // row below title
                popup_inner_x_start = x + 1; // inside left border
                break;
            }
        }
        if found_title { break; }
    }

    assert!(found_title, "Should find 'Commands' title");

    // Inner area should have panel bg, not app bg
    let mut found_panel_bg = false;
    for y in popup_inner_y..popup_inner_y + 5 {
        for x in popup_inner_x_start..popup_inner_x_start + 40 {
            let cell_bg = buf[(x, y)].style().bg;
            if cell_bg == Some(panel_bg) {
                found_panel_bg = true;
            }
            // Should NOT have app background in popup inner area
            assert_ne!(cell_bg, Some(app_bg),
                "Popup inner area at ({},{}) should not have app bg color", x, y);
        }
    }
    assert!(found_panel_bg, "Should find panel background color in popup");
}
