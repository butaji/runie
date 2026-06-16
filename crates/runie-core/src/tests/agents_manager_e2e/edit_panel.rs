use crate::commands::dsl::handlers::agents::build_edit_panel;
use crate::dialog::{ItemAction, PanelItem};
use crate::event::{DialogEvent, Event};

#[test]
fn edit_panel_has_at_least_eight_field_items() {
    let stack = build_edit_panel("anyname");
    let panel = stack.current().unwrap();
    assert!(
        panel.items.len() >= 8,
        "expected at least 8 items, got {}",
        panel.items.len()
    );
}

#[test]
fn edit_panel_has_save_action() {
    let stack = build_edit_panel("anyname");
    let panel = stack.current().unwrap();
    let has_save = panel.items.iter().any(|it| {
        if let PanelItem::Action { label, action } = it {
            label.contains("Save") && matches!(action, ItemAction::Emit(_))
        } else {
            false
        }
    });
    assert!(has_save, "Edit panel must have a Save action");
}

#[test]
fn edit_panel_save_emits_agents_manager_save_event() {
    let stack = build_edit_panel("testname");
    let panel = stack.current().unwrap();
    let save_item = panel.items.iter().find(|it| {
        if let PanelItem::Action { label, .. } = it {
            label.contains("Save")
        } else {
            false
        }
    });
    let action = match save_item.unwrap() {
        PanelItem::Action { action, .. } => action,
        _ => panic!("expected Action"),
    };
    match action {
        ItemAction::Emit(Event::Dialog(DialogEvent::AgentsManagerSave { name })) => {
            assert_eq!(*name, "testname");
        }
        other => panic!("expected AgentsManagerSave event, got {:?}", other),
    }
}

#[test]
fn edit_panel_has_back_action() {
    let stack = build_edit_panel("anyname");
    let panel = stack.current().unwrap();
    let has_back = panel.items.iter().any(|it| {
        if let PanelItem::Action { action, .. } = it {
            matches!(action, ItemAction::Pop)
        } else {
            false
        }
    });
    assert!(has_back, "Edit panel must have a Back action");
}

#[test]
fn edit_panel_field_items_emit_set_field_event() {
    let stack = build_edit_panel("myprofile");
    let panel = stack.current().unwrap();
    let set_field_count = panel
        .items
        .iter()
        .filter(|it| {
            if let PanelItem::Action { action, .. } = it {
                matches!(
                    action,
                    ItemAction::Emit(Event::Dialog(DialogEvent::AgentsManagerSetField { .. }))
                )
            } else {
                false
            }
        })
        .count();
    assert_eq!(
        set_field_count, 7,
        "expected 7 field items, got {}",
        set_field_count
    );
}
