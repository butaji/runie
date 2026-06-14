use crate::commands::handlers::agents::{
    build_delete_panel, build_edit_panel, build_root_panel, build_view_panel,
};

#[test]
fn root_panel_id_is_agents_root() {
    let stack = build_root_panel();
    let panel = stack.current().unwrap();
    assert_eq!(panel.id, "agents_root");
}

#[test]
fn view_panel_id_includes_profile_name() {
    let stack = build_view_panel("myprof");
    let panel = stack.current().unwrap();
    assert!(panel.id.contains("myprof"));
    assert!(panel.id.contains("agents_view"));
}

#[test]
fn edit_panel_id_includes_profile_name() {
    let stack = build_edit_panel("myprof");
    let panel = stack.current().unwrap();
    assert!(panel.id.contains("myprof"));
    assert!(panel.id.contains("agents_edit"));
}

#[test]
fn delete_panel_id_includes_profile_name() {
    let stack = build_delete_panel("myprof");
    let panel = stack.current().unwrap();
    assert!(panel.id.contains("myprof"));
    assert!(panel.id.contains("agents_delete"));
}
