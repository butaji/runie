//! Dialog Stack Management
//!
//! Handles dialog update, panel stack navigation, and form interactions.

use crate::commands::{DialogState, DialogType};
use crate::dialog::builders::{command_palette, model_selector, scoped_models, session_tree};
use crate::dialog::{ItemAction, Panel, PanelItem, PanelStack};
use crate::model_catalog::model_catalog;
use crate::model::AppState;
use crate::Event;

use super::form::FormAction;
use super::model_selector::partition_model_items;
use super::settings_dialog;

// ============================================================================
// Update Dialog
// ============================================================================

/// Handle command form dialog events. Entry point for legacy `CommandForm*` events.
pub fn handle_form_dialog(state: &mut AppState, event: Event) {
    let action = {
        let Some(d) = &mut state.open_dialog else {
            return;
        };
        let crate::commands::DialogState::PanelStack(stack) = d else {
            return;
        };
        let Some(panel) = stack.current_mut() else {
            return;
        };
        if !panel.is_form() {
            return;
        };
        form_panel_action(panel, event)
    };
    apply_form_action(state, action);
}

/// Insert filepath into input and close any dialog.
pub fn insert_at_ref(state: &mut AppState, path: &str) {
    state.input.input = path.to_string();
    state.input.cursor_pos = state.input.input.len();
    state.open_dialog = None;
    state.mark_dirty();
}

/// Handles dialog-specific events. Returns whether the dialog was closed.
pub fn update_dialog(state: &mut AppState, event: Event) {
    if matches!(event, Event::Abort) {
        state.open_dialog = None;
        state.mark_dirty();
        return;
    }

    // Global events pass through to model/config
    if matches!(
        event,
        Event::SwitchTheme { .. }
            | Event::SwitchModel { .. }
            | Event::CycleModelNext
            | Event::CycleModelPrev
            | Event::CycleThinkingLevel
            | Event::SetThinkingLevel(_)
            | Event::ToggleReadOnly
            | Event::TrustProject
            | Event::UntrustProject
    ) {
        super::model_config::model_config_event(state, event);
        return;
    }

    if matches!(event, Event::Quit) {
        state.should_quit = true;
        return;
    }

    let Some(mut dialog) = state.open_dialog.take() else {
        return;
    };

    // For activation on command palette, push to back stack before handler runs
    let is_palette_activation = matches!(event, Event::Submit | Event::PaletteSelect)
        && matches!(dialog, DialogState::CommandPalette(_));
    if is_palette_activation {
        state.push_dialog_to_back_stack(dialog.clone());
    }

    let stack = dialog.panel_stack_mut();
    let activated = update_panel_stack(state, event, stack);

    if !activated && state.open_dialog.is_none() {
        if is_palette_activation {
            state.dialog_back_stack.pop();
        } else {
            state.open_dialog = Some(dialog);
        }
    }
    state.mark_dirty();
}

/// Update a panel stack in response to an event. Returns `true` if an item was activated.
fn update_panel_stack(state: &mut AppState, event: Event, stack: &mut PanelStack) -> bool {
    use Event::*;

    let is_form = stack.current().is_some_and(|p| p.is_form());
    if is_form {
        return update_form_panel(state, event, stack);
    }

    match event {
        // ESC / close-key: stack nav
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

/// Pop the current dialog or close if at root.
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
        form_panel_action(panel, event)
    };

    if matches!(&action, FormAction::Back) {
        return handle_back_action(state, stack);
    }

    let keep_open = matches!(&action, FormAction::KeepOpen);
    if keep_open {
        state.open_dialog = Some(DialogState::PanelStack(stack.clone()));
    }
    apply_form_action(state, action);
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

// ============================================================================
// Form Panel Actions
// ============================================================================

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
        CommandFormInput(c) | Input(c) => {
            if panel.selected_form_field().is_some() {
                form_panel_edit_char(panel, c, true);
                A::KeepOpen
            } else if let Some(ItemAction::Emit(evt)) = panel.find_button_by_accel(c) {
                A::Submit(Some(evt.clone()))
            } else {
                A::KeepOpen
            }
        }
        CommandFormBackspace | Backspace => {
            form_panel_edit_char(panel, ' ', false);
            A::KeepOpen
        }
        CommandFormSubmit | Submit | SettingsSelect | PaletteSelect => {
            if let Some(item) = panel.selected_item() {
                match item {
                    PanelItem::Action { action: ItemAction::Emit(evt), .. } => {
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
        _ => A::KeepOpen,
    }
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
fn apply_form_action(state: &mut AppState, action: FormAction) {
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
        FormAction::Back => {} // Handled in update_form_panel
    }
}

// ============================================================================
// Panel Activation
// ============================================================================

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
    apply_panel_setting(state, key);
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
    apply_panel_setting(state, key);
}

fn apply_panel_setting(state: &mut AppState, key: &str) {
    match key {
        "read_only" => {
            state.config.read_only = !state.config.read_only;
            let status = if state.config.read_only { "enabled" } else { "disabled" };
            state.notify(format!("Read-only mode {}", status), crate::event::TransientLevel::Warning);
        }
        "steering_mode" => {
            state.config.steering_mode = match state.config.steering_mode {
                crate::model::DeliveryMode::OneAtATime => crate::model::DeliveryMode::All,
                crate::model::DeliveryMode::All => crate::model::DeliveryMode::OneAtATime,
            };
        }
        "follow_up_mode" => {
            state.config.follow_up_mode = match state.config.follow_up_mode {
                crate::model::DeliveryMode::OneAtATime => crate::model::DeliveryMode::All,
                crate::model::DeliveryMode::All => crate::model::DeliveryMode::OneAtATime,
            };
        }
        _ => {}
    }
}

// ============================================================================
// Command Result Processing
// ============================================================================

pub fn process_command_result(state: &mut AppState, result: crate::commands::CommandResult) {
    use crate::commands::CommandResult as CR;
    match result {
        CR::Message(msg) => state.add_system_msg(msg),
        CR::Warning(msg) => state.notify(msg, crate::event::TransientLevel::Warning),
        CR::Event(evt) => state.update(evt),
        CR::OpenDialog(d) => {
            if let Some(current) = state.open_dialog.take() {
                state.push_dialog_to_back_stack(current);
            }
            match d {
                DialogType::CommandPalette => open_command_palette(state),
                DialogType::ModelSelector => open_model_selector(state),
                DialogType::Settings => open_settings_dialog(state),
                DialogType::ScopedModels => open_scoped_models_dialog(state),
            }
        }
        CR::OpenPanelStack(stack) => {
            if let Some(current) = state.open_dialog.take() {
                state.push_dialog_to_back_stack(current);
            }
            state.open_dialog = Some(DialogState::PanelStack(stack));
            state.mark_dirty();
        }
        CR::None => {}
    }
}

// ============================================================================
// Open Dialog Functions (free functions for dialog_toggle + process_command_result)
// ============================================================================

/// Toggle a dialog open/closed. Used by dialog_toggle module.
pub fn toggle_dialog(state: &mut AppState, is_same: bool, open: fn(&mut AppState)) {
    if is_same {
        state.open_dialog = None;
        state.mark_dirty();
    } else {
        open(state);
    }
}

/// Open the command palette dialog.
pub fn open_command_palette(state: &mut AppState) {
    let mut items: Vec<(String, String, crate::Event)> = Vec::new();
    for cmd in state.registry.list() {
        let evt = crate::Event::RunPaletteCommand {
            name: cmd.name.clone(),
            args: String::new(),
        };
        items.push((cmd.category.as_str().to_string(), format!("{} {}", cmd.name, cmd.desc), evt));
    }
    for skill in &state.skills {
        if skill.user_invocable {
            let evt = crate::Event::RunSkillCommand {
                name: skill.name.clone(),
            };
            items.push((
                "Skill".to_string(),
                format!("{} {}", skill.name, skill.description),
                evt,
            ));
        }
    }
    state.open_dialog = Some(DialogState::CommandPalette(command_palette(items)));
    state.mark_dirty();
}

/// Open the model selector dialog.
pub fn open_model_selector(state: &mut AppState) {
    let current = format!(
        "{}/{}",
        state.config.current_provider, state.config.current_model
    );
    let items = crate::model_catalog::build_model_selector_items(
        &model_catalog(),
        &state.config.recent_models,
        "",
        &state.config.current_provider,
        &state.config.current_model,
    );
    let (recent, groups) = partition_model_items(items);
    state.open_dialog =
        Some(DialogState::ModelSelector(model_selector(recent, groups, &current)));
    state.mark_dirty();
}

/// Open the settings dialog.
pub fn open_settings_dialog(state: &mut AppState) {
    use crate::dialog::builders::{SettingsRow, SettingsRowKind};
    use crate::settings::SettingValue;
    let items = settings_dialog::build_setting_items(state);
    let mut categories: Vec<(String, Vec<SettingsRow>)> = Vec::new();
    for item in items {
        let cat_name = item.category.as_str().to_string();
        let row = match item.value {
            SettingValue::Bool(v) => SettingsRow {
                label: item.label,
                key: item.key,
                kind: SettingsRowKind::Bool(v),
            },
            SettingValue::Enum { current, options } => SettingsRow {
                label: item.label,
                key: item.key,
                kind: SettingsRowKind::Cycle { current, options },
            },
        };
        if let Some(last) = categories.last_mut() {
            if last.0 == cat_name {
                last.1.push(row);
                continue;
            }
        }
        categories.push((cat_name, vec![row]));
    }
    state.open_dialog = Some(DialogState::Settings(crate::dialog::builders::settings(
        categories,
    )));
    state.mark_dirty();
}

/// Open the scoped models dialog.
pub fn open_scoped_models_dialog(state: &mut AppState) {
    let models: Vec<(String, String, bool)> = state
        .config
        .scoped_models
        .iter()
        .map(|m| (m.provider.clone(), m.name.clone(), m.enabled))
        .collect();
    state.open_dialog = Some(DialogState::ScopedModels(scoped_models(models)));
    state.mark_dirty();
}

/// Open the session tree dialog.
pub fn open_session_tree_dialog(state: &mut AppState) {
    let items: Vec<(usize, String, crate::Event)> =
        match state.session.session_tree.as_ref() {
            Some(tree) => tree
                .filtered_walk(crate::session_tree::SessionTreeFilter::All)
                .into_iter()
                .map(|(depth, node)| {
                    let preview = format!(
                        "[{}] {}",
                        node.message.role.as_str(),
                        node.message.content.chars().take(60).collect::<String>()
                    );
                    let evt = crate::Event::SessionTreeSelect {
                        id: node.message.id.clone(),
                    };
                    (depth, preview, evt)
                })
                .collect(),
            None => Vec::new(),
        };
    state.open_dialog = Some(DialogState::SessionTree(session_tree(items)));
    state.mark_dirty();
}

/// Open a filterable @-file picker as a PanelStack dialog.
pub fn open_at_file_picker(state: &mut AppState) {
    let entries = crate::file_refs::find_file_entries(".", 50);
    let mut panel = Panel::new("at-files", " Files ").with_filter();
    if entries.is_empty() {
        panel = panel.header("No files found");
    } else {
        panel = panel.header(format!("{} files", entries.len()));
        for entry in entries {
            let label = if entry.is_dir {
                format!("{}/", entry.name)
            } else {
                entry.name.clone()
            };
            let insert_name = if entry.is_dir {
                format!("{}/", entry.name)
            } else {
                entry.name.clone()
            };
            panel = panel.item(&label, ItemAction::Emit(crate::Event::InsertAtRef(insert_name)));
        }
    }
    state.open_dialog = Some(DialogState::PanelStack(PanelStack::new(panel)));
    state.mark_dirty();
}

// ============================================================================
// Dialog Back Stack
// ============================================================================

impl AppState {
    /// Push a dialog onto the global back stack.
    pub(crate) fn push_dialog_to_back_stack(&mut self, dialog: DialogState) {
        self.dialog_back_stack.push(dialog);
    }

    /// Insert filepath into input and close any dialog.
    pub(crate) fn insert_at_ref(&mut self, path: &str) {
        self.input.input = path.to_string();
        self.input.cursor_pos = self.input.input.len();
        self.open_dialog = None;
        self.mark_dirty();
    }
}

// ============================================================================
// Form Submit Builder
// ============================================================================

/// Build the submit event for a form panel by reading form values.
pub fn form_build_submit(panel: &mut Panel) -> Option<crate::Event> {
    let values = panel.get_form_values().clone();
    let cmd = panel.id.clone();
    match cmd.as_str() {
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
