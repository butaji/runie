use super::*;

#[test]
fn snapshot_permission_modal() {
    let modal = PermissionModal::new(
        "bash",
        r#"{"command": "ls -la"}"#,
        "Lists files in the current directory",
    );
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 0, 60, 18);
    let mut buf = Buffer::empty(area);
    modal.render_ref(area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_permission_modal", buffer_to_string(&buf));
}

#[test]
fn snapshot_permission_dangerous_command() {
    let modal = PermissionModal::new(
        "bash",
        r#"rm -rf /"#,
        "⚠ This command will DELETE ALL FILES on your system!",
    );
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 0, 60, 18);
    let mut buf = Buffer::empty(area);
    modal.render_ref(area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_permission_dangerous_command", buffer_to_string(&buf));
}

#[test]
fn snapshot_permission_modal_timeout() {
    let mut modal = PermissionModal::new("bash", r#"{"command": "npm install"}"#, "Installing npm packages");
    modal.timeout_secs = Some(45);
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 0, 60, 18);
    let mut buf = Buffer::empty(area);
    modal.render_ref(area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_permission_modal_timeout", buffer_to_string(&buf));
}
