use super::*;
use ratatui::{backend::TestBackend, Terminal};

fn buffer_text(buf: &ratatui::buffer::Buffer) -> String {
    let area = buf.area();
    (0..area.height)
        .flat_map(|y| (0..area.width).map(move |x| buf[(x, y)].symbol().to_string()))
        .collect()
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
