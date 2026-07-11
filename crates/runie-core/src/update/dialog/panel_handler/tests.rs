#![allow(clippy::useless_conversion)]
use super::*;
use crate::commands::DialogState;
use crate::dialog::{ItemAction, Panel, PanelItem};

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
    state.open_dialog = Some(DialogState::Active {
        kind: DialogKind::Generic,
        panels: stack.clone(),
    });

    let result = update_panel_stack(&mut state, crate::Event::Input(' ').into(), &mut stack);
    assert_eq!(
        result,
        PanelUpdateResult::Consumed,
        "space should be consumed by the toggle"
    );
    assert!(
        matches!(
            state.open_dialog,
            Some(DialogState::Active {
                kind: DialogKind::Generic,
                panels: _
            })
        ),
        "dialog should stay open after toggling"
    );
    assert!(state.config.read_only);
}

/// Regression (live-test #6): pressing Enter on a settings/model toggle row must
/// toggle the checkbox and KEEP the dialog open (same as Space). It previously
/// toggled and then closed the dialog, so Enter looked like it just dismissed it.
#[test]
fn enter_on_toggle_row_toggles_and_keeps_dialog_open() {
    let mut state = AppState::default();
    state.config.read_only = false;
    let panel = Panel::new("settings", "Settings").toggle(
        "Read-Only",
        false,
        ItemAction::Toggle("read_only".into()),
    );
    let mut stack = PanelStack::new(panel);
    state.open_dialog = Some(DialogState::Active {
        kind: DialogKind::Generic,
        panels: stack.clone(),
    });

    let result = update_panel_stack(&mut state, crate::Event::Submit.into(), &mut stack);

    assert_eq!(
        result,
        PanelUpdateResult::Consumed,
        "Enter on a toggle row should toggle (consumed), not close the dialog"
    );
    assert!(
        matches!(
            state.open_dialog,
            Some(DialogState::Active {
                kind: DialogKind::Generic,
                ..
            })
        ),
        "dialog should stay open after Enter toggles a row"
    );
    assert!(
        state.config.read_only,
        "the setting should have been toggled by Enter"
    );
}

/// Regression (live-test ISSUE B): pressing Enter on a settings *cycle/select* row
/// (e.g. Thinking Level, Theme, Steering/Follow-Up Mode) must advance the option,
/// apply the setting, and KEEP the dialog open — same as a toggle row. It
/// previously cycled and then closed the dialog.
#[test]
fn enter_on_cycle_row_cycles_and_keeps_dialog_open() {
    use crate::model::ThinkingLevel;

    let mut state = AppState::default();
    let mut panel = Panel::new("settings", "Settings");
    panel.items.push(PanelItem::Select {
        label: "Thinking Level".into(),
        current: "Off".into(),
        options: vec!["Off".into(), "Low".into(), "Medium".into(), "High".into()],
        key: "thinking_level".into(),
    });
    let mut stack = PanelStack::new(panel);
    state.open_dialog = Some(DialogState::Active {
        kind: DialogKind::Generic,
        panels: stack.clone(),
    });

    let result = update_panel_stack(&mut state, crate::Event::Submit.into(), &mut stack);

    assert_eq!(
        result,
        PanelUpdateResult::Consumed,
        "Enter on a cycle/select row should cycle (consumed), not close the dialog"
    );
    assert!(
        matches!(
            state.open_dialog,
            Some(DialogState::Active {
                kind: DialogKind::Generic,
                ..
            })
        ),
        "dialog should stay open after Enter cycles a settings row"
    );
    assert_eq!(
        state.config.thinking_level,
        ThinkingLevel::Low,
        "the setting should have advanced Off -> Low and been applied"
    );
}
