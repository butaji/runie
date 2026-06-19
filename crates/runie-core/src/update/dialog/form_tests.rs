//! Tests for form panel editing (cursor-aware input).

use super::*;
use crate::LoginFlowEvent;

fn key_panel() -> Panel {
    Panel::new("login-key", "Login")
        .form_field_value("API Key", "sk-...", "key", String::new())
        .form_submit()
}

fn set_field(panel: &mut Panel, value: &str, cursor_pos: usize) {
    panel.form_values.insert("key".into(), value.into());
    if let Some(idx) = panel.selected_form_field() {
        if let crate::dialog::PanelItem::FormField {
            value: v,
            cursor_pos: cp,
            ..
        } = &mut panel.items[idx]
        {
            *v = value.into();
            *cp = cursor_pos;
        }
    }
}

#[test]
fn typing_inserts_at_cursor_position() {
    let mut state = AppState::default();
    let mut panel = key_panel();
    set_field(&mut panel, "ab", 1);

    let action = form_panel_action(&mut state, &mut panel, Event::Input('X'));

    assert!(matches!(action, FormAction::KeepOpen));
    assert_eq!(panel.form_values.get("key"), Some(&"aXb".to_string()));
}

#[test]
fn backspace_deletes_before_cursor() {
    let mut state = AppState::default();
    let mut panel = key_panel();
    set_field(&mut panel, "abc", 2);

    let action = form_panel_action(&mut state, &mut panel, Event::Backspace);

    assert!(matches!(action, FormAction::KeepOpen));
    assert_eq!(panel.form_values.get("key"), Some(&"ac".to_string()));
}

#[test]
fn cursor_left_moves_before_previous_grapheme() {
    let mut state = AppState::default();
    let mut panel = key_panel();
    set_field(&mut panel, "ab", 2);

    let action = form_panel_action(&mut state, &mut panel, Event::CursorLeft);

    assert!(matches!(action, FormAction::KeepOpen));
    let pos = panel
        .selected_form_field()
        .and_then(|i| match &panel.items[i] {
            crate::dialog::PanelItem::FormField { cursor_pos, .. } => Some(*cursor_pos),
            _ => None,
        });
    assert_eq!(pos, Some(1));
}

#[test]
fn paste_appends_to_selected_form_field() {
    let mut state = AppState::default();
    let mut panel = key_panel();
    let action = form_panel_action(&mut state, &mut panel, Event::Paste("sk-pasted".into()));
    assert!(matches!(action, FormAction::KeepOpen));
    assert_eq!(panel.form_values.get("key"), Some(&"sk-pasted".to_string()));
}

#[test]
fn paste_ignores_paste_when_no_field_selected() {
    let mut state = AppState::default();
    let mut panel = key_panel();
    // Move selection down to the Submit button.
    panel.select_down();
    let action = form_panel_action(&mut state, &mut panel, Event::Paste("sk-pasted".into()));
    assert!(matches!(action, FormAction::KeepOpen));
    assert!(!panel.form_values.contains_key("key"));
}

#[test]
fn submit_on_toggle_checkbox_keeps_form_open() {
    let mut state = AppState::default();
    state.config.read_only = false;
    let mut panel = Panel::new("settings", "Settings").toggle(
        "Read-Only",
        false,
        ItemAction::Toggle("read_only".into()),
    );

    let action = form_panel_action(&mut state, &mut panel, Event::Submit);

    assert!(matches!(action, FormAction::KeepOpen));
    assert!(state.config.read_only, "toggle setting should be applied");
    assert!(
        matches!(
            panel.selected_item(),
            Some(PanelItem::Toggle { value: true, .. })
        ),
        "checkbox value should flip"
    );
}

#[test]
fn submit_on_emit_toggle_checkbox_updates_state_and_keeps_open() {
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
        ItemAction::Emit(LoginFlowEvent::ToggleModel { model: "m1".into() }),
    );

    let action = form_panel_action(&mut state, &mut panel, Event::Submit);

    assert!(matches!(action, FormAction::KeepOpen));
    let flow = state.login_flow.as_ref().expect("login flow");
    assert!(flow.selected_models.contains("m1"));
}

#[test]
fn submit_on_emit_action_dispatches_event() {
    let mut state = AppState::default();
    let mut panel =
        Panel::new("models", "Models").item("_Save", ItemAction::Emit(LoginFlowEvent::Save));

    let action = form_panel_action(&mut state, &mut panel, Event::Submit);

    assert!(
        matches!(action, FormAction::Submit(Some(crate::Event::Save))),
        "Enter on an emit action should submit its event"
    );
}
