use super::*;
use ratatui::{backend::TestBackend, Terminal};

fn buffer_text(buf: &ratatui::buffer::Buffer) -> String {
    let area = buf.area();
    (0..area.height)
        .flat_map(|y| (0..area.width).map(move |x| buf[(x, y)].symbol().to_string()))
        .collect()
}

#[test]
fn renders_permission_modal() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    let snap = Snapshot {
        permission_request: Some(PermissionRequestState {
            request_id: "perm-1".into(),
            tool: "bash".into(),
            input: serde_json::json!("echo hello"),
        }),
        ..Default::default()
    };

    terminal
        .draw(|f| crate::ui::draw_snapshot(f, &snap))
        .unwrap();

    let text = buffer_text(terminal.backend().buffer());
    assert!(text.contains("Permission Required"), "modal title missing");
    assert!(text.contains("bash"), "tool name missing");
    assert!(text.contains("[y] Allow"), "allow hint missing");
}
