use crate::ui::view;
use ratatui::{backend::TestBackend, Terminal};
use runie_core::{AppState, ChatMessage, Event, Role};

/// Helper: check if the given text string appears anywhere in the rect.
fn rect_contains_text(
    buf: &ratatui::buffer::Buffer,
    rect: ratatui::layout::Rect,
    text: &str,
) -> bool {
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
        x: 10,
        y: 3,
        width: 60,
        height: 18,
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
        x: 10,
        y: 3,
        width: 60,
        height: 18,
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
        x: 10,
        y: 3,
        width: 60,
        height: 18,
    };
    assert!(
        !rect_contains_text(buf, popup_rect, "XYZZY"),
        "Model selector should hide underlying content"
    );
}

/// Default theme uses the terminal background for both the app and popup
/// areas. The popup must still render its own content so underlying
/// messages are not visible.
#[test]
fn command_palette_uses_terminal_background() {
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
    state.update(Event::ToggleCommandPalette);
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    // Both app and popup backgrounds should be the terminal default (Reset).
    assert_eq!(
        crate::theme::color_bg(),
        ratatui::style::Color::Reset,
        "App background should use terminal default"
    );
    assert_eq!(
        crate::theme::color_bg_panel(),
        ratatui::style::Color::Reset,
        "Popup background should use terminal default"
    );

    // Underlying message content must still be hidden by the popup widgets.
    let popup_rect = ratatui::layout::Rect {
        x: 10,
        y: 3,
        width: 60,
        height: 18,
    };
    assert!(
        !rect_contains_text(buf, popup_rect, "XYZZY"),
        "Popup should hide underlying 'XYZZY_PLUGH' even with transparent bg"
    );
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
    state.input.input = "/theme".into();
    state.input.cursor_pos = 6;
    state.update(Event::Submit);
    assert!(
        matches!(
            state.open_dialog,
            Some(runie_core::commands::DialogState::PanelStack(_))
        ),
        "PanelStack dialog should be open"
    );
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let popup_rect = ratatui::layout::Rect {
        x: 10,
        y: 3,
        width: 60,
        height: 18,
    };
    assert!(
        !rect_contains_text(buf, popup_rect, "XYZZY"),
        "Panel dialog should hide underlying 'XYZZY_PLUGH'"
    );
}
