//! Form panel action mapping and submit building.

use crate::dialog::{ItemAction, Panel, PanelItem};
use crate::model::AppState;
use crate::Event;

use super::form::FormAction;

/// Map a single event to an action on a form panel.
pub fn form_panel_action(panel: &mut Panel, event: Event) -> FormAction {
    use Event::*;
    use FormAction as A;
    match event {
        SettingsClose | CommandFormClose | DialogBack => A::Back,
        CommandFormUp | HistoryPrev | SettingsUp | PaletteUp | ModelSelectorUp => {
            let _ = panel.select_up();
            A::KeepOpen
        }
        CommandFormDown | HistoryNext | SettingsDown | PaletteDown | ModelSelectorDown => {
            let _ = panel.select_down();
            A::KeepOpen
        }
        CommandFormInput(c) | Input(c) => handle_form_input(panel, c),
        CommandFormBackspace | Backspace => {
            form_panel_edit_char(panel, ' ', false);
            A::KeepOpen
        }
        CommandFormSubmit | Submit | SettingsSelect | PaletteSelect => handle_form_submit(panel),
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
    let values = panel.get_form_values().clone();
    build_event_for_form_command(&panel.id, &values)
}

fn build_event_for_form_command(
    cmd: &str,
    values: &std::collections::HashMap<String, String>,
) -> Option<crate::Event> {
    match cmd {
        "save" => Some(crate::Event::RunSaveCommand {
            name: values.get("name").cloned().unwrap_or_default(),
        }),
        "load" => Some(crate::Event::RunLoadCommand {
            name: values.get("name").cloned().unwrap_or_default(),
        }),
        "delete" => Some(crate::Event::RunDeleteCommand {
            name: values.get("name").cloned().unwrap_or_default(),
        }),
        "import" => Some(crate::Event::RunImportCommand {
            path: values.get("path").cloned().unwrap_or_default(),
        }),
        "export" => Some(crate::Event::RunExportCommand {
            path: values.get("path").cloned().unwrap_or_default(),
        }),
        "skill" => Some(crate::Event::RunSkillCommand {
            name: values.get("name").cloned().unwrap_or_default(),
        }),
        "providers" | "provider" => Some(crate::Event::ProvidersDialog),
        "name" => Some(crate::Event::RunNameCommand {
            name: values.get("name").cloned().unwrap_or_default(),
        }),
        "fork" => Some(crate::Event::RunForkCommand {
            message_index: values.get("index").cloned().unwrap_or_default(),
        }),
        "compact" => Some(crate::Event::RunCompactCommand {
            keep: values.get("keep").cloned().unwrap_or_default(),
            focus: values.get("focus").cloned().unwrap_or_default(),
        }),
        "prompt" => Some(crate::Event::RunPromptCommand {
            name: values.get("name").cloned().unwrap_or_default(),
        }),
        "login-key" => Some(crate::Event::LoginFlowSubmitKey {
            provider: String::new(),
            key: values.get("key").cloned().unwrap_or_default(),
        }),
        _ => None,
    }
}
