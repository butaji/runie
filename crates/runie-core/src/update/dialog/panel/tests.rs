use super::*;
use crate::commands::DialogState;
use crate::dialog::Panel;
use crate::Event;

#[test]
fn space_toggles_checkbox_item_value() {
    let mut state = AppState::default();
    state.config.read_only = false;
    let mut panel = Panel::new("test", "Test").toggle(
        "Read-Only",
        false,
        ItemAction::Toggle("read_only".into()),
    );

    assert!(toggle_selected_checkbox(&mut state, &mut panel));
    assert!(
        matches!(
            panel.selected_item(),
            Some(PanelItem::Toggle { value: true, .. })
        ),
        "checkbox value should flip to true"
    );
    assert!(
        state.config.read_only,
        "read_only setting should be applied"
    );
}

#[test]
fn space_on_non_toggle_does_nothing() {
    let mut state = AppState::default();
    let mut panel = Panel::new("test", "Test").item("Do", ItemAction::Close);
    assert!(!toggle_selected_checkbox(&mut state, &mut panel));
}

#[test]
fn space_on_emit_checkbox_updates_state() {
    let mut state = AppState::default();
    let mut flow = crate::login_flow::LoginFlowState::new()
        .with_provider("minimax".into())
        .with_key("sk".into())
        .with_validation_success(vec!["m1".into()]);
    flow.selected_models.clear();
    state.login_flow = Some(flow);

    let mut panel = Panel::new("models", "Models").toggle(
        "m1",
        false,
        ItemAction::Emit(crate::Event::from(crate::Event::ToggleModel {
            model: "m1".into(),
        })),
    );

    assert!(toggle_selected_checkbox(&mut state, &mut panel));
    let flow = state.login_flow.as_ref().expect("login flow");
    assert!(flow.selected_models.contains("m1"));
}

#[test]
fn space_in_list_panel_keeps_dialog_open() {
    let mut state = AppState::default();
    let panel = Panel::new("settings", "Settings").toggle(
        "Read-Only",
        false,
        ItemAction::Toggle("read_only".into()),
    );
    let mut stack = PanelStack::new(panel);
    state.open_dialog = Some(DialogState::PanelStack(stack.clone()));

    let result = update_panel_stack(&mut state, crate::Event::Input(' ').into(), &mut stack);
    assert_eq!(
        result,
        PanelUpdateResult::Consumed,
        "space should be consumed by the toggle"
    );
    assert!(
        matches!(state.open_dialog, Some(DialogState::PanelStack(_))),
        "dialog should stay open after toggling"
    );
    assert!(state.config.read_only);
}
