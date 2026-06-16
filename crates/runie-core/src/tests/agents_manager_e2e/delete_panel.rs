use crate::commands::handlers::agents::build_delete_panel;
use crate::dialog::{ItemAction, PanelItem};
use crate::event::{DialogEvent, Event};

#[test]
fn delete_panel_has_yes_and_no() {
    let stack = build_delete_panel("anyname");
    let panel = stack.current().unwrap();
    let has_yes = panel.items.iter().any(|it| {
        if let PanelItem::Action { label, .. } = it {
            label.contains("Yes")
        } else {
            false
        }
    });
    let has_no = panel.items.iter().any(|it| {
        if let PanelItem::Action { label, .. } = it {
            label.contains("No")
        } else {
            false
        }
    });
    assert!(has_yes, "Delete panel must have Yes");
    assert!(has_no, "Delete panel must have No");
}

#[test]
fn delete_panel_yes_emits_delete_event() {
    let stack = build_delete_panel("killme");
    let panel = stack.current().unwrap();
    let yes_item = panel.items.iter().find(|it| {
        if let PanelItem::Action { label, .. } = it {
            label.contains("Yes")
        } else {
            false
        }
    });
    let action = match yes_item.unwrap() {
        PanelItem::Action { action, .. } => action,
        _ => panic!("expected Action"),
    };
    match action {
        ItemAction::Emit(Event::Dialog(DialogEvent::AgentsManagerDelete { name })) => {
            assert_eq!(*name, "killme");
        }
        other => panic!("expected AgentsManagerDelete event, got {:?}", other),
    }
}

#[test]
fn delete_panel_no_pops_back() {
    let stack = build_delete_panel("anyname");
    let panel = stack.current().unwrap();
    let no_item = panel.items.iter().find(|it| {
        if let PanelItem::Action { label, .. } = it {
            label.contains("No")
        } else {
            false
        }
    });
    let action = match no_item.unwrap() {
        PanelItem::Action { action, .. } => action,
        _ => panic!("expected Action"),
    };
    assert!(matches!(action, ItemAction::Pop));
}
