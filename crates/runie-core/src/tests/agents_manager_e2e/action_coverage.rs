use crate::commands::handlers::agents::{build_delete_panel, build_root_panel, build_view_panel};
use crate::dialog::{ItemAction, PanelItem};

#[test]
fn root_panel_only_has_push_and_emit_and_close() {
    let stack = build_root_panel();
    let panel = stack.current().unwrap();
    for item in &panel.items {
        if let PanelItem::Action { action, .. } = item {
            match action {
                ItemAction::Push(_) | ItemAction::Emit(_) | ItemAction::Close => {}
                other => panic!("unexpected action in root: {:?}", other),
            }
        }
    }
}

#[test]
fn view_panel_actions_are_valid() {
    let stack = build_view_panel("x");
    let panel = stack.current().unwrap();
    for item in &panel.items {
        if let PanelItem::Action { action, .. } = item {
            match action {
                ItemAction::Push(_) | ItemAction::Pop | ItemAction::Close | ItemAction::Emit(_) => {
                }
                other => panic!("unexpected action: {:?}", other),
            }
        }
    }
}

#[test]
fn delete_panel_actions_are_valid() {
    let stack = build_delete_panel("x");
    let panel = stack.current().unwrap();
    for item in &panel.items {
        if let PanelItem::Action { action, .. } = item {
            match action {
                ItemAction::Pop | ItemAction::Close | ItemAction::Emit(_) => {}
                other => panic!("unexpected action: {:?}", other),
            }
        }
    }
}
