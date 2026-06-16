use crate::commands::dsl::handlers::subagent::handle_spawn;
use crate::commands::CommandResult;
use crate::dialog::PanelItem;
use crate::model::AppState;

#[test]
fn form_command_sets_open_dialog() {
    let mut state = AppState::default();
    let result = handle_spawn(&mut state, "");

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
    let result = handle_spawn(&mut state, "");

    if let CommandResult::OpenPanelStack(stack) = result {
        let panel = stack.current().unwrap();
        let has_field = panel
            .items
            .iter()
            .any(|it| matches!(it, PanelItem::FormField { .. }));
        assert!(has_field, "spawn form should have at least one form field");
    } else {
        panic!("expected panel stack");
    }
}
