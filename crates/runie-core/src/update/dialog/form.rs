//! Form Action Types and panel form handling (merged from dialog_form.rs).

use crate::dialog::{ItemAction, Panel, PanelItem};
use crate::event::{DialogEvent, InputEvent, ModelConfigEvent};
use crate::model::AppState;
use crate::Event;

/// What a form panel should do in response to an event.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
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
pub fn form_panel_action(panel: &mut Panel, event: Event) -> FormAction {
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
        DialogEvent::CommandFormInput(c) => {
            handle_form_input(panel, *c)
        }
        InputEvent::Input(c) => {
            handle_form_input(panel, *c)
        }
        DialogEvent::CommandFormBackspace | InputEvent::Backspace => {
            form_panel_edit_char(panel, ' ', false);
            A::KeepOpen
        }
        DialogEvent::CommandFormSubmit
        | InputEvent::Submit
        | ModelConfigEvent::SettingsSelect
        | DialogEvent::PaletteSelect => handle_form_submit(panel),
        _ => A::KeepOpen,
    }
}

fn handle_form_input(panel: &mut Panel, c: char) -> FormAction {
    use FormAction as A;
    if panel.selected_form_field().is_some() {
        form_panel_edit_char(panel, c, true);
        return A::KeepOpen;
    }
    if let Some(ItemAction::Emit(evt)) = panel.find_button_by_accel(c) {
        return A::Submit(Some(evt.clone()));
    }
    A::KeepOpen
}

fn handle_form_submit(panel: &mut Panel) -> FormAction {
    use FormAction as A;
    if let Some(item) = panel.selected_item() {
        match item {
            PanelItem::Action {
                action: ItemAction::Emit(evt),
                ..
            } => {
                return A::Submit(Some(evt.clone()));
            }
            PanelItem::Action { .. } | PanelItem::FormSubmit => {
                return A::Submit(None);
            }
            _ => {}
        }
    }
    A::Submit(form_build_submit(panel))
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
