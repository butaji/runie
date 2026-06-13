//! Panel stack navigation and item activation.

use crate::commands::DialogState;
use crate::dialog::{ItemAction, PanelItem, PanelStack};
use crate::model::AppState;
use crate::Event;

use super::form::FormAction;

/// Update a panel stack in response to an event. Returns `true` if an item was activated.
pub fn update_panel_stack(state: &mut AppState, event: Event, stack: &mut PanelStack) -> bool {
    use Event::*;

    let is_form = stack.current().is_some_and(|p| p.is_form());
    if is_form {
        return update_form_panel(state, event, stack);
    }

    match event {
        SettingsClose | PaletteClose | ModelSelectorClose | DialogBack => {
            if stack.len() > 1 {
                stack.pop();
            } else {
                return pop_dialog_or_close(state);
            }
        }
        HistoryPrev | SettingsUp | PaletteUp | ModelSelectorUp => stack.select_up(),
        HistoryNext | SettingsDown | PaletteDown | ModelSelectorDown => stack.select_down(),
        CursorLeft | SettingsLeft => {
            stack.pop();
        }
        Submit | SettingsSelect | PaletteSelect | ModelSelectorSelect => {
            return try_activate_panel(state, stack);
        }
        PaletteFilter(c) | ModelSelectorFilter(c) | Input(c) => stack.push_filter(c),
        PaletteBackspace | ModelSelectorBackspace | Backspace => stack.pop_filter(),
        _ => {}
    }
    state.mark_dirty();
    false
}

fn pop_dialog_or_close(state: &mut AppState) -> bool {
    if let Some(previous) = state.dialog_back_stack.pop() {
        state.open_dialog = Some(previous);
        state.mark_dirty();
        false
    } else {
        state.open_dialog = None;
        state.mark_dirty();
        true
    }
}

/// Update a form panel. Returns `true` if closed.
fn update_form_panel(state: &mut AppState, event: Event, stack: &mut PanelStack) -> bool {
    let action = {
        let panel = stack.current_mut().expect("form panel");
        super::dialog_form::form_panel_action(panel, event)
    };

    if matches!(&action, FormAction::Back) {
        return handle_back_action(state, stack);
    }

    let keep_open = matches!(&action, FormAction::KeepOpen);
    if keep_open {
        state.open_dialog = Some(DialogState::PanelStack(stack.clone()));
    }
    super::dialog_form::apply_form_action(state, action);
    !keep_open
}

fn handle_back_action(state: &mut AppState, stack: &mut PanelStack) -> bool {
    if stack.len() > 1 {
        stack.pop();
        state.open_dialog = Some(DialogState::PanelStack(stack.clone()));
        false
    } else {
        pop_dialog_or_close(state)
    }
}

fn try_activate_panel(state: &mut AppState, stack: &mut PanelStack) -> bool {
    if let Some(action) = stack.activate() {
        if handle_panel_action(state, action, stack) {
            return true;
        }
    }
    false
}

/// Handle a panel item action. Returns `true` if the dialog was closed.
fn handle_panel_action(
    state: &mut AppState,
    action: ItemAction,
    stack: &mut PanelStack,
) -> bool {
    match action {
        ItemAction::Push(_) | ItemAction::Pop => {
            stack.pop();
            false
        }
        ItemAction::Close => {
            state.open_dialog = None;
            state.mark_dirty();
            true
        }
        ItemAction::Emit(evt) => {
            let keep_open = stack.current().map(|p| p.keep_open_on_activate).unwrap_or(false);
            if !keep_open {
                state.open_dialog = None;
            }
            state.mark_dirty();
            state.update(evt);
            !keep_open
        }
        ItemAction::Toggle(key) => {
            panel_toggle_item(state, stack, &key);
            false
        }
        ItemAction::Cycle(key) => {
            panel_cycle_item(state, stack, &key);
            false
        }
    }
}

fn panel_toggle_item(state: &mut AppState, stack: &mut PanelStack, key: &str) {
    if let Some(PanelItem::Toggle { value, .. }) =
        stack.current_mut().and_then(|p| p.selected_item_mut())
    {
        *value = !*value;
    }
    apply_panel_setting(state, stack, key);
}

fn panel_cycle_item(state: &mut AppState, stack: &mut PanelStack, key: &str) {
    if let Some(PanelItem::Select { current, options, .. }) =
        stack.current_mut().and_then(|p| p.selected_item_mut())
    {
        if let Some(idx) = options.iter().position(|o| o == current) {
            let next = (idx + 1) % options.len();
            *current = options[next].clone();
        }
    }
    apply_panel_setting(state, stack, key);
}

fn apply_panel_setting(state: &mut AppState, stack: &mut PanelStack, key: &str) {
    match key {
        "read_only" => state.toggle_read_only(),
        "steering_mode" => state.config.steering_mode = cycle_delivery_mode(state.config.steering_mode),
        "follow_up_mode" => state.config.follow_up_mode = cycle_delivery_mode(state.config.follow_up_mode),
        "provider" => {
            if let Some(value) = selected_select_value(stack) {
                state.set_provider(&value);
                state.view.cached_settings_valid = false;
            }
        }
        "model" => {
            if let Some(value) = selected_select_value(stack) {
                state.set_model(&value);
                state.view.cached_settings_valid = false;
            }
        }
        "theme" => {
            if let Some(value) = selected_select_value(stack) {
                state.switch_theme(value);
            }
        }
        "thinking_level" => {
            if let Some(value) = selected_select_value(stack) {
                if let Ok(level) = value.parse::<crate::model::ThinkingLevel>() {
                    state.set_thinking_level(level);
                }
            }
        }
        _ => {}
    }
}

fn cycle_delivery_mode(mode: crate::model::DeliveryMode) -> crate::model::DeliveryMode {
    match mode {
        crate::model::DeliveryMode::OneAtATime => crate::model::DeliveryMode::All,
        crate::model::DeliveryMode::All => crate::model::DeliveryMode::OneAtATime,
    }
}

fn selected_select_value(stack: &mut PanelStack) -> Option<String> {
    stack
        .current_mut()
        .and_then(|p| p.selected_item_mut())
        .and_then(|item| match item {
            crate::dialog::PanelItem::Select { current, .. } => Some(current.clone()),
            _ => None,
        })
}
