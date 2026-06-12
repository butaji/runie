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

fn find_popup_title(buf: &ratatui::buffer::Buffer, title: &str) -> Option<(u16, u16)> {
    let search_len = title.len();
    for y in 0..buf.area().height {
        for x in 0..buf.area().width.saturating_sub(search_len as u16) {
            let s: String = (x..x + search_len as u16).map(|cx| buf[(cx, y)].symbol()).collect();
            if s == title {
                return Some((x + 1, y + 1)); // inner: +1 for border
            }
        }
    }
    None
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

    let (inner_x, inner_y) = find_popup_title(buf, " Commands")
        .expect("Should find 'Commands' title");

    let mut found_panel_bg = false;
    for y in inner_y..inner_y + 5 {
        for x in inner_x..inner_x + 40 {
            let cell_bg = buf[(x, y)].style().bg;
            if cell_bg == Some(panel_bg) {
                found_panel_bg = true;
            }
            assert_ne!(cell_bg, Some(app_bg),
                "Popup inner at ({},{}) should not have app bg", x, y);
        }
    }
    assert!(found_panel_bg, "Should find panel background color in popup");
}

/// Panel dialog (e.g. /theme selector) must hide underlying content.
#[test]
fn panel_dialog_hides_underlying_messages() {
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

    // Open theme panel dialog via /theme with no args
    state.input.input = "/theme".into();
    state.input.cursor_pos = 6;
    state.update(Event::Submit);

    assert!(
        matches!(state.open_dialog, Some(runie_core::commands::DialogState::PanelStack(_))),
        "PanelStack dialog should be open"
    );

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    let popup_rect = ratatui::layout::Rect {
        x: 10, y: 3, width: 60, height: 18,
    };
    assert!(
        !rect_contains_text(buf, popup_rect, "XYZZY"),
        "Panel dialog should hide underlying 'XYZZY_PLUGH'"
    );
}
