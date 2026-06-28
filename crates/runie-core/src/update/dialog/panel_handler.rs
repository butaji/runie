//! Panel stack navigation and item activation (merged from dialog_panel.rs).

use crate::commands::{DialogKind, DialogState};
use crate::dialog::{ItemAction, Panel, PanelItem, PanelStack};
use crate::model::AppState;
use crate::Event;

use super::form::FormAction;
use super::toggles;

/// Result of handling a single event in a panel stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PanelUpdateResult {
    /// Event was consumed and the dialog should remain open.
    Consumed,
    /// Event closed the dialog.
    Closed,
    /// Event was ignored by the panel stack.
    Ignored,
}

/// Whether the root panel of the active dialog allows dismissal.
pub(crate) fn root_closable(state: &AppState) -> bool {
    state
        .open_dialog
        .as_ref()
        .and_then(|d| d.panel_stack())
        .and_then(|s| s.root())
        .map(|p| p.closable)
        .unwrap_or(true)
}

/// Update a panel stack in response to an event.
pub fn update_panel_stack(
    state: &mut AppState,
    event: Event,
    stack: &mut PanelStack,
) -> PanelUpdateResult {
    let is_form = stack.current().is_some_and(|p| p.is_form());
    if is_form {
        return update_form_panel(state, event, stack);
    }

    if handle_panel_close(state, &event, stack) {
        return PanelUpdateResult::Closed;
    }
    if handle_panel_navigation(state, &event, stack) {
        return PanelUpdateResult::Consumed;
    }
    if let Some(result) = handle_panel_activation(state, &event, stack) {
        return result;
    }
    handle_panel_filter(state, &event, stack);
    state.view_mut().dirty = true;
    PanelUpdateResult::Ignored
}

fn handle_panel_close(state: &mut AppState, event: &Event, stack: &mut PanelStack) -> bool {
    match event {
        crate::Event::SettingsClose
        | crate::Event::PaletteClose
        | crate::Event::ModelSelectorClose
        | crate::Event::DialogBack => {
            if stack.len() > 1 {
                stack.pop();
            } else {
                let root_closable = stack.root().map(|p| p.closable).unwrap_or(true);
                return pop_dialog_or_close(state, root_closable);
            }
        }
        // intentionally ignored: PanelPop events are handled above in the specific arm
        _ => {}
    }
    false
}

fn handle_panel_navigation(_state: &mut AppState, event: &Event, stack: &mut PanelStack) -> bool {
    match event {
        crate::Event::HistoryPrev
        | crate::Event::SettingsUp
        | crate::Event::PaletteUp
        | crate::Event::ModelSelectorUp => {
            stack.select_up();
            return true;
        }
        crate::Event::HistoryNext
        | crate::Event::SettingsDown
        | crate::Event::PaletteDown
        | crate::Event::ModelSelectorDown => {
            stack.select_down();
            return true;
        }
        crate::Event::CursorLeft | crate::Event::SettingsLeft => {
            stack.pop();
            return true;
        }
        crate::Event::Input('\t') => {
            stack.select_down();
            return true;
        }
        // intentionally ignored: other input events fall through
        _ => {}
    }
    false
}

fn handle_panel_activation(
    state: &mut AppState,
    event: &Event,
    stack: &mut PanelStack,
) -> Option<PanelUpdateResult> {
    match event {
        crate::Event::Submit
        | crate::Event::SettingsSelect
        | crate::Event::PaletteSelect
        | crate::Event::ModelSelectorSelect => {
            return Some(try_activate_panel(state, stack));
        }
        crate::Event::Input(' ') => {
            if let Some(panel) = stack.current_mut() {
                if toggle_selected_checkbox(state, panel) {
                    return Some(PanelUpdateResult::Consumed);
                }
            }
        }
        // intentionally ignored: other events fall through
        _ => {}
    }
    None
}

fn handle_panel_filter(state: &mut AppState, event: &Event, stack: &mut PanelStack) {
    match event {
        crate::Event::PaletteFilter(c) => stack.push_filter(*c),
        crate::Event::ModelSelectorFilter(c) => stack.push_filter(*c),
        crate::Event::Input(c) => {
            let is_file_picker = stack.current().is_some_and(|p| p.id == "at-files");
            stack.push_filter(*c);
            // If this is the file picker, re-query FFF with the new filter.
            // Read `is_file_picker` BEFORE calling rebuild_file_picker (which replaces open_dialog).
            if is_file_picker {
                super::rebuild_file_picker(state);
            }
        }
        crate::Event::PaletteBackspace
        | crate::Event::ModelSelectorBackspace
        | crate::Event::Backspace => {
            let is_file_picker = stack.current().is_some_and(|p| p.id == "at-files");
            stack.pop_filter();
            if is_file_picker {
                super::rebuild_file_picker(state);
            }
        }
        // intentionally ignored: other filter events are no-ops
        _ => {}
    }
}

fn pop_dialog_or_close(state: &mut AppState, root_closable: bool) -> bool {
    if !root_closable {
        // The root panel has asked to stay open.
        state.view_mut().dirty = true;
        return false;
    }
    if let Some(previous) = state.dialog_back_stack_mut().pop() {
        *state.open_dialog_mut() = Some(previous);
        state.view_mut().dirty = true;
        false
    } else {
        *state.open_dialog_mut() = None;
        // NOTE: Do NOT reset input_receiver here. handle_vim_dialog_back()
        // checks input_receiver == Dialog to know a dialog was closed and
        // should NOT trigger vim-nav. It will reset input_receiver itself.
        state.view_mut().dirty = true;
        true
    }
}

/// Update a form panel.
fn update_form_panel(
    state: &mut AppState,
    event: Event,
    stack: &mut PanelStack,
) -> PanelUpdateResult {
    let action = {
        let panel = stack.current_mut().expect("form panel");
        super::form::form_panel_action(state, panel, event)
    };

    if matches!(&action, FormAction::Back) {
        return if handle_back_action(state, stack) {
            PanelUpdateResult::Closed
        } else {
            PanelUpdateResult::Consumed
        };
    }

    let keep_open = matches!(&action, FormAction::KeepOpen);
    if keep_open && state.open_dialog().is_none() {
        *state.open_dialog_mut() = Some(DialogState::Active { kind: DialogKind::Generic, panels: stack.clone() });
    }
    super::form::apply_form_action(state, action);
    if keep_open {
        PanelUpdateResult::Consumed
    } else {
        PanelUpdateResult::Closed
    }
}

fn handle_back_action(state: &mut AppState, stack: &mut PanelStack) -> bool {
    if stack.len() > 1 {
        stack.pop();
        *state.open_dialog_mut() = Some(DialogState::Active { kind: DialogKind::Generic, panels: stack.clone() });
        false
    } else {
        let root_closable = stack.root().map(|p| p.closable).unwrap_or(true);
        pop_dialog_or_close(state, root_closable)
    }
}

fn try_activate_panel(state: &mut AppState, stack: &mut PanelStack) -> PanelUpdateResult {
    if let Some(action) = stack.activate() {
        if handle_panel_action(state, action, stack) {
            return PanelUpdateResult::Closed;
        }
    }
    PanelUpdateResult::Consumed
}

/// Handle a panel item action. Returns `true` if the dialog was closed.
fn handle_panel_action(state: &mut AppState, action: ItemAction, stack: &mut PanelStack) -> bool {
    match action {
        ItemAction::Push(_) | ItemAction::Pop => {
            stack.pop();
            false
        }
        ItemAction::Close => {
            *state.open_dialog_mut() = None;
            state.view_mut().input_receiver = crate::model::InputReceiver::ChatInput;
            state.view_mut().dirty = true;
            true
        }
        ItemAction::Emit(evt) => handle_emit_action(state, stack, evt),
        ItemAction::Toggle(key) => {
            panel_toggle_item(state, stack, &key);
            close_panel_on_activate(state, stack)
        }
        ItemAction::Cycle(key) => {
            panel_cycle_item(state, stack, &key);
            close_panel_on_activate(state, stack)
        }
    }
}

/// Extract args from panel filter for RunPaletteCommand.
fn extract_palette_args(name: &str, filter: &str) -> String {
    let filter = filter.trim();
    if filter == name {
        String::new()
    } else if let Some(rest) = filter.strip_prefix(name) {
        rest.trim().to_owned()
    } else {
        filter.to_owned()
    }
}

fn handle_emit_action(state: &mut AppState, stack: &mut PanelStack, evt: crate::Event) -> bool {
    let keep_open = stack
        .current()
        .map(|p| p.keep_open_on_activate)
        .unwrap_or(false);
    if !keep_open {
        *state.open_dialog_mut() = None;
        state.view_mut().input_receiver = crate::model::InputReceiver::ChatInput;
    }
    state.view_mut().dirty = true;
    // For RunPaletteCommand, pass the panel filter as args
    let evt = if let crate::Event::RunPaletteCommand { name, args } = &evt {
        if args.is_empty() {
            if let Some(panel) = stack.current() {
                let filter_args = extract_palette_args(name, &panel.filter);
                crate::Event::RunPaletteCommand {
                    name: name.clone(),
                    args: filter_args,
                }
            } else {
                evt.clone()
            }
        } else {
            evt.clone()
        }
    } else {
        evt
    };
    state.update(evt);
    !keep_open
}

fn close_panel_on_activate(state: &mut AppState, stack: &mut PanelStack) -> bool {
    let keep_open = stack
        .current()
        .map(|p| p.keep_open_on_activate)
        .unwrap_or(false);
    if !keep_open {
        *state.open_dialog_mut() = None;
        state.view_mut().input_receiver = crate::model::InputReceiver::ChatInput;
        state.view_mut().dirty = true;
    }
    !keep_open
}

fn panel_toggle_item(state: &mut AppState, stack: &mut PanelStack, _key: &str) {
    if let Some(panel) = stack.current_mut() {
        let _ = toggle_selected_checkbox(state, panel);
    }
}

/// Toggle the currently selected checkbox (if any) and apply its side effect.
/// Returns `true` if a toggle item was selected.
pub(super) fn toggle_selected_checkbox(state: &mut AppState, panel: &mut Panel) -> bool {
    let Some(item) = panel.selected_item_mut() else {
        return false;
    };
    toggle_checkbox_item(state, item)
}

fn toggle_checkbox_item(state: &mut AppState, item: &mut PanelItem) -> bool {
    if let PanelItem::Toggle { value, action, .. } = item {
        *value = !*value;
        match action {
            ItemAction::Toggle(key) => {
                apply_checkbox_setting(state, key);
            }
            ItemAction::Emit(evt) => {
                state.update(evt.clone());
            }
            // intentionally ignored: other item actions are handled elsewhere
            _ => {}
        }
        state.view_mut().dirty = true;
        true
    } else {
        false
    }
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
    if apply_checkbox_setting(state, key) {
        return;
    }
    match key {
        "steering_mode" => {
            state.config_mut().steering_mode = cycle_delivery_mode(state.config_mut().steering_mode)
        }
        "follow_up_mode" => {
            state.config_mut().follow_up_mode =
                cycle_delivery_mode(state.config_mut().follow_up_mode)
        }
        "provider" | "model" | "theme" | "thinking_level" => {
            apply_select_setting(state, stack, key);
        }
        "truncation_max_lines" | "truncation_max_bytes" => {
            apply_truncation_setting(state, stack, key);
        }
        // intentionally ignored: other settings are handled by their specific handlers
        _ => {}
    }
}

fn apply_checkbox_setting(state: &mut AppState, key: &str) -> bool {
    if let Some((provider, model)) = toggles::parse_provider_model_toggle(key) {
        toggles::toggle_provider_model(state, provider, model);
        return true;
    }
    match key {
        "read_only" => state.toggle_read_only(),
        "vim_mode" => toggle_vim_mode(state),
        "telemetry_enabled" => toggle_telemetry(state),
        _ => return false,
    }
    true
}

fn apply_select_setting(state: &mut AppState, stack: &mut PanelStack, key: &str) {
    let Some(value) = selected_select_value(stack) else {
        return;
    };
    match key {
        "provider" => {
            state.set_provider(&value);
            state.view_mut().cached_settings_valid = false;
        }
        "model" => {
            state.set_model(&value);
            state.view_mut().cached_settings_valid = false;
        }
        "theme" => state.switch_theme(value),
        "thinking_level" => {
            if let Ok(level) = value.parse::<crate::model::ThinkingLevel>() {
                state.set_thinking_level(level);
            }
        }
        // intentionally ignored: other settings have specific handlers above
        _ => {}
    }
}

fn toggle_vim_mode(state: &mut AppState) {
    let new_value = !state.config().vim_mode;
    state.config_mut().vim_mode = new_value;
    let handles = state.actor_handles().cloned();
    if let Some(h) = handles {
        if tokio::runtime::Handle::try_current().is_ok() {
            let h = h;
            tokio::spawn(async move {
                h.send_set_vim_mode(new_value).await;
            });
        }
    }
    state.view_mut().cached_settings_valid = false;
}

fn toggle_telemetry(state: &mut AppState) {
    let new_enabled = !state.config().telemetry.is_enabled();
    state.config_mut().telemetry = crate::telemetry::Telemetry::new(new_enabled);
    let handles = state.actor_handles().cloned();
    if let Some(h) = handles {
        if tokio::runtime::Handle::try_current().is_ok() {
            let h = h;
            tokio::spawn(async move {
                h.send_set_telemetry(new_enabled).await;
            });
        }
    }
    state.view_mut().cached_settings_valid = false;
}

fn apply_truncation_setting(state: &mut AppState, stack: &mut PanelStack, key: &str) {
    let Some(value) = selected_select_value(stack) else {
        return;
    };
    let Ok(n) = value.parse::<usize>() else {
        return;
    };
    let mut truncation = state.config().truncation.clone();
    match key {
        "truncation_max_lines" => truncation.max_lines = n,
        "truncation_max_bytes" => truncation.max_bytes = n,
        _ => return,
    }
    state.config_mut().truncation = truncation.clone();
    let handles = state.actor_handles().cloned();
    if let Some(h) = handles {
        if tokio::runtime::Handle::try_current().is_ok() {
            let h = h;
            tokio::spawn(async move {
                h.send_set_truncation(truncation).await;
            });
        }
    }
    state.view_mut().cached_settings_valid = false;
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

#[cfg(test)]
mod tests;
