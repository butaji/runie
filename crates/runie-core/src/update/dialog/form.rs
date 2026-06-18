//! Form Action Types and panel form handling (merged from dialog_form.rs).

use crate::dialog::{ItemAction, Panel, PanelItem};
use crate::event::{DialogEvent, InputEvent, ModelConfigEvent, TransientLevel};
use crate::model::AppState;
use crate::Event;

use super::panel::toggle_selected_checkbox;

/// What a form panel should do in response to an event.
#[derive(Debug, Clone)]
pub enum FormAction {
    /// Keep the form open, persist the panel state.
    KeepOpen,
    /// Close the form (no submit).
    #[allow(dead_code)]
    Close,
    /// Close the form and dispatch the submit event.
    Submit(Option<crate::Event>),
    /// Go back one step: if the stack is deeper than the root, pop the
    /// current panel and keep the dialog open; if at the root, close
    /// the dialog. This is the semantic of ESC / back.
    Back,
}

/// Map a single event to an action on a form panel.
pub fn form_panel_action(state: &mut AppState, panel: &mut Panel, event: Event) -> FormAction {
    use FormAction as A;
    match &event {
        ModelConfigEvent::SettingsClose
        | DialogEvent::CommandFormClose
        | DialogEvent::DialogBack => A::Back,
        DialogEvent::CommandFormUp
        | InputEvent::HistoryPrev
        | ModelConfigEvent::SettingsUp
        | DialogEvent::PaletteUp
        | DialogEvent::ModelSelectorUp => {
            let _ = panel.select_up();
            A::KeepOpen
        }
        DialogEvent::CommandFormDown
        | InputEvent::HistoryNext
        | ModelConfigEvent::SettingsDown
        | DialogEvent::PaletteDown
        | DialogEvent::ModelSelectorDown => {
            let _ = panel.select_down();
            A::KeepOpen
        }
        DialogEvent::CommandFormInput(c) => handle_form_input(state, panel, *c),
        InputEvent::Input(' ') => handle_form_space(state, panel),
        InputEvent::Input(c) => handle_form_input(state, panel, *c),
        InputEvent::Paste(text) => handle_form_paste(panel, text),
        DialogEvent::CommandFormBackspace | InputEvent::Backspace => {
            form_panel_edit_char(panel, ' ', false);
            A::KeepOpen
        }
        DialogEvent::CommandFormSubmit
        | InputEvent::Submit
        | ModelConfigEvent::SettingsSelect
        | DialogEvent::PaletteSelect => handle_form_submit(state, panel),
        _ => A::KeepOpen,
    }
}

fn handle_form_input(state: &mut AppState, panel: &mut Panel, c: char) -> FormAction {
    use FormAction as A;
    if panel.selected_form_field().is_some() {
        form_panel_edit_char(panel, c, true);
        return A::KeepOpen;
    }
    if let Some(ItemAction::Emit(evt)) = panel.find_button_by_accel(c) {
        if panel.id == "login-key" && is_empty_submit_key(evt, panel) {
            state.set_transient("API key is required.".into(), TransientLevel::Warning);
            return A::KeepOpen;
        }
        return A::Submit(Some(evt.clone()));
    }
    A::KeepOpen
}

fn handle_form_space(state: &mut AppState, panel: &mut Panel) -> FormAction {
    use FormAction as A;
    if toggle_selected_checkbox(state, panel) {
        return A::KeepOpen;
    }
    handle_form_input(state, panel, ' ')
}

fn handle_form_paste(panel: &mut Panel, text: &str) -> FormAction {
    use FormAction as A;
    if panel.selected_form_field().is_some() {
        form_panel_paste(panel, text);
    }
    A::KeepOpen
}

fn form_panel_paste(panel: &mut Panel, text: &str) {
    let Some(idx) = panel.selected_form_field() else {
        return;
    };
    let PanelItem::FormField { value, key, .. } = &mut panel.items[idx] else {
        return;
    };
    value.push_str(text);
    panel.form_values.insert(key.clone(), value.clone());
}

fn is_empty_submit_key(evt: &crate::Event, panel: &Panel) -> bool {
    matches!(
        evt,
        crate::Event::SubmitKey { key, .. } if key.trim().is_empty() && key_field_empty(panel)
    )
}

fn handle_form_submit(state: &mut AppState, panel: &mut Panel) -> FormAction {
    use FormAction as A;
    if panel.id == "login-key" && key_field_empty(panel) {
        state.set_transient("API key is required.".into(), TransientLevel::Warning);
        return A::KeepOpen;
    }
    match panel.selected_item().cloned() {
        Some(PanelItem::Action {
            action: ItemAction::Emit(evt),
            ..
        }) => {
            return A::Submit(Some(evt));
        }
        Some(PanelItem::Action { .. }) => {
            return A::Submit(None);
        }
        Some(PanelItem::Toggle {
            action: ItemAction::Emit(crate::Event::ToggleModel { model }),
            ..
        }) if panel.id == "login-models" => {
            // In the login model selector, Enter confirms the selection and
            // saves. Make sure the focused model is selected before saving.
            if let Some(flow) = state.login_flow.as_mut() {
                flow.selected_models.insert(model.clone());
            }
            return A::Submit(Some(crate::Event::Save));
        }
        Some(PanelItem::Toggle { .. }) => {
            toggle_selected_checkbox(state, panel);
            return A::KeepOpen;
        }
        _ => {}
    }
    A::Submit(form_build_submit(panel))
}

fn key_field_empty(panel: &Panel) -> bool {
    panel
        .form_values
        .get("key")
        .map(|v| v.trim().is_empty())
        .unwrap_or(true)
}

fn form_panel_edit_char(panel: &mut Panel, c: char, push: bool) {
    let Some(idx) = panel.selected_form_field() else {
        return;
    };
    let PanelItem::FormField { value, key, .. } = &mut panel.items[idx] else {
        return;
    };
    if push {
        value.push(c);
    } else {
        value.pop();
    }
    panel.form_values.insert(key.clone(), value.clone());
}

/// Apply a `FormAction` to the current dialog.
pub fn apply_form_action(state: &mut AppState, action: FormAction) {
    match action {
        FormAction::Close => {
            state.open_dialog = None;
            state.mark_dirty();
        }
        FormAction::Submit(evt) => {
            state.open_dialog = None;
            state.mark_dirty();
            if let Some(e) = evt {
                state.update(e);
            }
        }
        FormAction::KeepOpen => {
            state.mark_dirty();
        }
        FormAction::Back => {}
    }
}

/// Build the submit event for a form panel by reading form values.
pub fn form_build_submit(panel: &mut Panel) -> Option<crate::Event> {
    let factory = panel.submit_factory?;
    let values = panel.get_form_values().clone();
    Some(factory(&values))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LoginFlowEvent;

    fn key_panel() -> Panel {
        Panel::new("login-key", "Login")
            .form_field_value("API Key", "sk-...", "key", String::new())
            .form_submit()
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
}
