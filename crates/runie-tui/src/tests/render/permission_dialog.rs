use super::*;
use ratatui::{backend::TestBackend, Terminal};
use runie_core::Event;

fn buffer_text(buf: &ratatui::buffer::Buffer) -> String {
    let area = buf.area();
    (0..area.height)
        .flat_map(|y| (0..area.width).map(move |x| buf[(x, y)].symbol().to_string()))
        .collect()
}

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

/// Permission dialog must hide underlying message content.
/// Bug: dialog was transparent, showing feed content through it.
#[test]
fn permission_dialog_hides_underlying_messages() {
    let _lock = crate::theme::test_lock();

    // Add a test message to AppState (simulating real state with messages)
    let mut state = AppState::default();
    state.session.messages.push(ChatMessage {
        role: Role::User,
        parts: vec![Part::Text {
            content: "XYZZY_PLUGH".into(),
        }],
        timestamp: 0.0,
        id: "u0".into(),
        ..Default::default()
    });
    state.messages_changed();
    state.update(Event::PermissionRequest {
        request_id: "perm-1".into(),
        tool: "bash".into(),
        input: serde_json::json!("echo hello"),
    });

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
        "Permission dialog should hide underlying 'XYZZY_PLUGH' — transparent background bug"
    );
}

#[test]
fn renders_hosted_permission_panel() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();

    let req = runie_core::model::PermissionRequestState {
        request_id: "perm-1".into(),
        tool: "bash".into(),
        input: serde_json::json!("echo hello"),
    };
    let dialog = runie_core::update::permission_dialog::open_permission_dialog(&req);

    let snap = Snapshot {
        dialog: Some(dialog),
        permission_request: Some(req),
        ..Default::default()
    };

    let mut throbber = throbber_widgets_tui::ThrobberState::default();
    terminal
        .draw(|f| crate::ui::draw_snapshot(f, &snap, &mut throbber))
        .unwrap();

    let text = buffer_text(terminal.backend().buffer());
    assert!(text.contains("Permission Required"), "panel title missing");
    assert!(text.contains("bash"), "tool name missing");
    // 4-option dialog: Always / This session / Once / Deny.
    assert!(text.contains("Always"), "always action missing");
    assert!(text.contains("This session"), "session action missing");
    assert!(text.contains("Once"), "once action missing");
    assert!(text.contains("Deny"), "deny action missing");
}
