use crate::commands::CommandResult;
use crate::dialog::PanelItem;
use crate::model::AppState;

#[test]
fn form_command_sets_open_dialog() {
    let mut state = AppState::default();
    let result = state
        .handle_slash("/load")
        .expect("load command should return a result");

    if let CommandResult::OpenPanelStack(stack) = result {
        assert!(!stack.panels.is_empty());
        let panel = stack.current().unwrap();
        assert!(!panel.title.is_empty());
    } else {
        panic!("expected panel stack");
    }
}

#[test]
fn form_panels_have_input_field() {
    let mut state = AppState::default();
    let result = state
        .handle_slash("/load")
        .expect("load command should return a result");

    if let CommandResult::OpenPanelStack(stack) = result {
        let panel = stack.current().unwrap();
        let has_field = panel
            .items
            .iter()
            .any(|it| matches!(it, PanelItem::FormField { .. }));
        assert!(has_field, "load form should have at least one form field");
    } else {
        panic!("expected panel stack");
    }
}

#[test]
fn skill_form_panel_carries_command_name_and_keys() {
    let mut state = AppState::default();
    let result = state
        .handle_slash("/skill grill-me")
        .expect("skill command should return a result");

    if let CommandResult::OpenPanelStack(stack) = result {
        let panel = stack.current().unwrap();
        assert_eq!(panel.cmd_name.as_deref(), Some("skill"));
        assert_eq!(panel.field_keys, vec!["name".to_string()]);
        let has_name_field = panel.items.iter().any(|it| matches!(it,
            PanelItem::FormField { placeholder, .. } if placeholder == "skill-name"
        ));
        assert!(has_name_field, "skill form should show the skill-name placeholder");
    } else {
        panic!("expected panel stack");
    }
}
