//! Panel stack navigation and item activation (merged from dialog_panel.rs).

use crate::commands::DialogState;
use crate::dialog::{ItemAction, PanelItem, PanelStack};
use crate::event::{DialogEvent, InputEvent, ModelConfigEvent};
use crate::model::AppState;
use crate::Event;

use super::form::FormAction;

/// Update a panel stack in response to an event. Returns `true` if an item was activated.
pub fn update_panel_stack(state: &mut AppState, event: Event, stack: &mut PanelStack) -> bool {
    let is_form = stack.current().is_some_and(|p| p.is_form());
    if is_form {
        return update_form_panel(state, event, stack);
    }

    if handle_panel_close(state, &event, stack) {
        return true;
    }
    if handle_panel_navigation(state, &event, stack) {
        return false;
    }
    if handle_panel_activation(state, &event, stack) {
        return true;
    }
    handle_panel_filter(state, &event, stack);
    state.mark_dirty();
    false
}

fn handle_panel_close(state: &mut AppState, event: &Event, stack: &mut PanelStack) -> bool {
    match event {
        ModelConfigEvent::SettingsClose
        | DialogEvent::PaletteClose
        | DialogEvent::ModelSelectorClose
        | DialogEvent::DialogBack => {
            if stack.len() > 1 {
                stack.pop();
            } else {
                return pop_dialog_or_close(state);
            }
        }
        _ => {}
    }
    false
}

fn handle_panel_navigation(_state: &mut AppState, event: &Event, stack: &mut PanelStack) -> bool {
    match event {
        InputEvent::HistoryPrev
        | ModelConfigEvent::SettingsUp
        | DialogEvent::PaletteUp
        | DialogEvent::ModelSelectorUp => {
            stack.select_up();
            return true;
        }
        InputEvent::HistoryNext
        | ModelConfigEvent::SettingsDown
        | DialogEvent::PaletteDown
        | DialogEvent::ModelSelectorDown => {
            stack.select_down();
            return true;
        }
        InputEvent::CursorLeft | ModelConfigEvent::SettingsLeft => {
            stack.pop();
            return true;
        }
        InputEvent::Input('\t') => {
            stack.select_down();
            return true;
        }
        _ => {}
    }
    false
}

fn handle_panel_activation(state: &mut AppState, event: &Event, stack: &mut PanelStack) -> bool {
    match event {
        InputEvent::Submit
        | ModelConfigEvent::SettingsSelect
        | DialogEvent::PaletteSelect
        | DialogEvent::ModelSelectorSelect => {
            return try_activate_panel(state, stack);
        }
        _ => {}
    }
    false
}

fn handle_panel_filter(state: &mut AppState, event: &Event, stack: &mut PanelStack) {
    match event {
        DialogEvent::PaletteFilter(c) => stack.push_filter(*c),
        DialogEvent::ModelSelectorFilter(c) => stack.push_filter(*c),
        InputEvent::Input(c) => {
            let is_file_picker = stack.current().is_some_and(|p| p.id == "at-files");
            stack.push_filter(*c);
            // If this is the file picker, re-query FFF with the new filter.
            // Read `is_file_picker` BEFORE calling rebuild_file_picker (which replaces open_dialog).
            if is_file_picker {
                super::rebuild_file_picker(state);
            }
        }
        DialogEvent::PaletteBackspace
        | DialogEvent::ModelSelectorBackspace
        | InputEvent::Backspace => {
            let is_file_picker = stack.current().is_some_and(|p| p.id == "at-files");
            stack.pop_filter();
            if is_file_picker {
                super::rebuild_file_picker(state);
            }
        }
        _ => {}
    }
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
        super::form::form_panel_action(panel, event)
    };

    if matches!(&action, FormAction::Back) {
        return handle_back_action(state, stack);
    }

    let keep_open = matches!(&action, FormAction::KeepOpen);
    if keep_open {
        state.open_dialog = Some(DialogState::PanelStack(stack.clone()));
    }
    super::form::apply_form_action(state, action);
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
fn handle_panel_action(state: &mut AppState, action: ItemAction, stack: &mut PanelStack) -> bool {
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
            let keep_open = stack
                .current()
                .map(|p| p.keep_open_on_activate)
                .unwrap_or(false);
            if !keep_open {
                state.open_dialog = None;
            }
            state.mark_dirty();
            state.update(evt);
            !keep_open
        }
        ItemAction::Toggle(key) => {
            panel_toggle_item(state, stack, &key);
            let keep_open = stack
                .current()
                .map(|p| p.keep_open_on_activate)
                .unwrap_or(false);
            if !keep_open {
                state.open_dialog = None;
                state.mark_dirty();
            }
            !keep_open
        }
        ItemAction::Cycle(key) => {
            panel_cycle_item(state, stack, &key);
            let keep_open = stack
                .current()
                .map(|p| p.keep_open_on_activate)
                .unwrap_or(false);
            if !keep_open {
                state.open_dialog = None;
                state.mark_dirty();
            }
            !keep_open
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
    if let Some(PanelItem::Select {
        current, options, ..
    }) = stack.current_mut().and_then(|p| p.selected_item_mut())
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
        "steering_mode" => {
            state.config.steering_mode = cycle_delivery_mode(state.config.steering_mode)
        }
        "follow_up_mode" => {
            state.config.follow_up_mode = cycle_delivery_mode(state.config.follow_up_mode)
        }
        "provider" | "model" | "theme" | "thinking_level" => {
            apply_select_setting(state, stack, key);
        }
        "vim_mode" => toggle_vim_mode(state),
        "telemetry_enabled" => toggle_telemetry(state),
        "truncation_max_lines" | "truncation_max_bytes" => {
            apply_truncation_setting(state, stack, key);
        }
        _ => {}
    }
}

fn apply_select_setting(state: &mut AppState, stack: &mut PanelStack, key: &str) {
    let Some(value) = selected_select_value(stack) else {
        return;
    };
    match key {
        "provider" => {
            state.set_provider(&value);
            state.view.cached_settings_valid = false;
        }
        "model" => {
            state.set_model(&value);
            state.view.cached_settings_valid = false;
        }
        "theme" => state.switch_theme(value),
        "thinking_level" => {
            if let Ok(level) = value.parse::<crate::model::ThinkingLevel>() {
                state.set_thinking_level(level);
            }
        }
        _ => {}
    }
}

fn toggle_vim_mode(state: &mut AppState) {
    state.config.vim_mode = !state.config.vim_mode;
    state.view.cached_settings_valid = false;
}

fn toggle_telemetry(state: &mut AppState) {
    let new_enabled = !state.config.telemetry.is_enabled();
    state.config.telemetry = crate::telemetry::Telemetry::new(new_enabled);
    state.view.cached_settings_valid = false;
}

fn apply_truncation_setting(state: &mut AppState, stack: &mut PanelStack, key: &str) {
    let Some(value) = selected_select_value(stack) else {
        return;
    };
    let Ok(n) = value.parse::<usize>() else {
        return;
    };
    match key {
        "truncation_max_lines" => state.config.truncation.max_lines = n,
        "truncation_max_bytes" => state.config.truncation.max_bytes = n,
        _ => return,
    }
    state.view.cached_settings_valid = false;
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
