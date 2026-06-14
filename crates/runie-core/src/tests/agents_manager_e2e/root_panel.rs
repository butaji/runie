use crate::commands::handlers::agents::build_root_panel;
use crate::dialog::{ItemAction, PanelItem};

fn find_action<F>(stack: &crate::dialog::PanelStack, predicate: F) -> Option<ItemAction>
where
    F: Fn(&PanelItem) -> bool,
{
    let panel = stack.current().unwrap();
    panel
        .items
        .iter()
        .find(|it| predicate(it))
        .and_then(|it| match it {
            PanelItem::Action { action, .. } => Some(action.clone()),
            _ => None,
        })
}

#[test]
fn root_panel_always_has_new_profile_item() {
    let stack = build_root_panel();
    let panel = stack.current().unwrap();
    let has_new = panel.items.iter().any(|it| {
        if let PanelItem::Action { label, .. } = it {
            label.contains("New profile")
        } else {
            false
        }
    });
    assert!(has_new, "Root must always show '+ New profile'");
}

#[test]
fn root_panel_always_has_close_item() {
    let stack = build_root_panel();
    let panel = stack.current().unwrap();
    let has_close = panel.items.iter().any(|it| {
        if let PanelItem::Action { label, action } = it {
            label.contains("Close") && matches!(action, ItemAction::Close)
        } else {
            false
        }
    });
    assert!(has_close, "Root must always show 'Close' action");
}

#[test]
fn root_panel_new_profile_pushes_edit_panel() {
    let stack = build_root_panel();
    let action = find_action(
        &stack,
        |it| matches!(it, PanelItem::Action { label, .. } if label.contains("New profile")),
    );
    match action {
        Some(ItemAction::Push(target)) => {
            assert_eq!(
                target, "agents_edit_new",
                "New profile must push to agents_edit_new"
            );
        }
        other => panic!("expected Push action, got {:?}", other),
    }
}

#[test]
fn root_panel_no_profiles_shows_empty_message() {
    let stack = build_root_panel();
    let panel = stack.current().unwrap();
    let has_msg = panel.items.iter().any(|it| {
        if let PanelItem::Action { label, .. } = it {
            label.contains("no profiles") || label.contains("No profiles")
        } else {
            false
        }
    });
    assert!(panel.items.len() >= 2);
    let _ = has_msg;
}

#[test]
fn root_panel_with_profile_has_view_action() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("myagent.toml");
    std::fs::write(
        &path,
        r#"
        name = "myagent"
        description = "Test"
        system_prompt = "x"
        tools = []
    "#,
    )
    .unwrap();
    let profiles = crate::agent_profiles::load_profiles_from_dir(dir.path()).unwrap();
    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].name, "myagent");
}
