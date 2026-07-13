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

    terminal
        .draw(|f| crate::ui::draw_snapshot(f, &snap))
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

/// Every button label must render contiguously on a single line at common
/// terminal widths. Bug (live-found): at 120 columns the button row
/// overflowed the dialog and ratatui wrapped it mid-label, splitting
/// "4." and "Deny" across two lines.
#[test]
fn permission_dialog_buttons_never_wrap_mid_label() {
    let _lock = crate::theme::test_lock();
    for (w, h) in [(60u16, 20u16), (80, 24), (120, 35)] {
        let backend = TestBackend::new(w, h);
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

        terminal
            .draw(|f| crate::ui::draw_snapshot(f, &snap))
            .unwrap();

        let buf = terminal.backend().buffer();
        let area = buf.area();
        let lines: Vec<String> = (0..area.height)
            .map(|y| (0..area.width).map(|x| buf[(x, y)].symbol()).collect())
            .collect();
        for label in ["1. Always", "2. This session", "3. Once", "4. Deny"] {
            assert!(
                lines.iter().any(|l| l.contains(label)),
                "button {label:?} wrapped mid-label at {w}x{h}:\n{}",
                lines.join("\n")
            );
        }
    }
}

/// A fieldless form (permission dialog) must not show the field-editing
/// hint "Fill in the form and press Enter to submit" — there is nothing
/// to fill in; the user selects an option.
#[test]
fn fieldless_form_does_not_show_fill_in_hint() {
    let _lock = crate::theme::test_lock();
    let backend = TestBackend::new(80, 24);
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

    terminal
        .draw(|f| crate::ui::draw_snapshot(f, &snap))
        .unwrap();

    let text = buffer_text(terminal.backend().buffer());
    assert!(
        !text.contains("Fill in the form"),
        "fieldless permission dialog shows field-editing hint"
    );
}
