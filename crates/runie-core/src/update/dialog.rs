//! Dialog opening and top-level routing.

use crate::commands::{DialogState, DialogType};
use crate::dialog::builders::{command_palette, model_selector, scoped_models, session_tree};
use crate::model::AppState;
use crate::model_catalog::model_catalog;
use crate::Event;

use super::model_selector::partition_model_items;
use super::settings_dialog;

/// Handle command form dialog events. Entry point for legacy `CommandForm*` events.
pub fn handle_form_dialog(state: &mut AppState, event: Event) {
    let action = {
        let Some(d) = &mut state.open_dialog else {
            return;
        };
        let DialogState::PanelStack(stack) = d else {
            return;
        };
        let Some(panel) = stack.current_mut() else {
            return;
        };
        if !panel.is_form() {
            return;
        }
        super::dialog_form::form_panel_action(panel, event)
    };
    super::dialog_form::apply_form_action(state, action);
}

/// Insert filepath into input and close any dialog.
pub fn insert_at_ref(state: &mut AppState, path: &str) {
    state.input.input = build_insert_text(state, path);
    state.input.cursor_pos = state.input.input.len();
    state.open_dialog = None;
    state.mark_dirty();
}

fn build_insert_text(state: &mut AppState, path: &str) -> String {
    let Some((original_input, insert_pos, cursor, _)) = state.file_picker_backup.take() else {
        return path.to_string();
    };
    let before = compute_before_text(&original_input, insert_pos, cursor);
    let after = extract_after(&original_input, cursor);
    format!("{}{}{}", before, path, after)
}

fn compute_before_text(original_input: &str, insert_pos: usize, cursor: usize) -> String {
    let before = extract_before(original_input, insert_pos);
    let trimmed_before = before.trim_end();
    let prefix = extract_prefix(original_input, insert_pos, cursor);
    let has_word_boundary_space =
        insert_pos > 0 && original_input.chars().nth(insert_pos - 1) == Some(' ');

    if has_word_boundary_space {
        before
    } else if cursor >= original_input.len() && trimmed_before != prefix {
        trimmed_before.to_string()
    } else {
        before
    }
}

fn extract_before(original_input: &str, insert_pos: usize) -> String {
    if insert_pos > 0 {
        original_input[..insert_pos].to_string()
    } else {
        String::new()
    }
}

fn extract_prefix(original_input: &str, insert_pos: usize, cursor: usize) -> &str {
    if cursor > insert_pos && insert_pos < original_input.len() {
        original_input[insert_pos..cursor].trim()
    } else {
        ""
    }
}

fn extract_after(original_input: &str, cursor: usize) -> String {
    if cursor < original_input.len() {
        original_input[cursor..].to_string()
    } else {
        String::new()
    }
}

/// Handles dialog-specific events. Returns whether the dialog was closed.
pub fn update_dialog(state: &mut AppState, event: Event) {
    if route_global_dialog_event(state, &event) {
        return;
    }
    let Some(mut dialog) = state.open_dialog.take() else {
        return;
    };
    let is_palette_activation = is_palette_activation(&dialog, &event);
    if is_palette_activation {
        state.push_dialog_to_back_stack(dialog.clone());
    }
    let stack = dialog.panel_stack_mut();
    let activated = super::dialog_panel::update_panel_stack(state, event, stack);
    restore_or_pop_dialog(state, dialog, activated, is_palette_activation);
    state.mark_dirty();
}

fn route_global_dialog_event(state: &mut AppState, event: &Event) -> bool {
    if matches!(event, Event::Abort) {
        // Restore input backup if exists (from file picker)
        if let Some((input, _, _, _)) = state.file_picker_backup.take() {
            state.input.input = input;
            state.input.cursor_pos = state.input.input.len();
        }
        state.open_dialog = None;
        state.mark_dirty();
        return true;
    }
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
        super::model_config::model_config_event(state, event.clone());
        return true;
    }
    if matches!(event, Event::Quit) {
        state.should_quit = true;
        return true;
    }
    false
}

fn is_palette_activation(dialog: &DialogState, event: &Event) -> bool {
    matches!(event, Event::Submit | Event::PaletteSelect)
        && matches!(dialog, DialogState::CommandPalette(_))
}

fn restore_or_pop_dialog(
    state: &mut AppState,
    dialog: DialogState,
    activated: bool,
    is_palette_activation: bool,
) {
    if !activated && state.open_dialog.is_none() {
        if is_palette_activation {
            state.dialog_back_stack.pop();
        } else {
            state.open_dialog = Some(dialog);
        }
    }
}

/// Push a dialog onto the global back stack.
pub fn push_dialog_to_back_stack(state: &mut AppState, dialog: DialogState) {
    state.push_dialog_to_back_stack(dialog);
}

/// Toggle a dialog open/closed. Used by dialog_toggle module.
pub fn toggle_dialog(state: &mut AppState, is_same: bool, open: fn(&mut AppState)) {
    if is_same {
        state.open_dialog = None;
        state.mark_dirty();
    } else {
        open(state);
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
                push_dialog_to_back_stack(state, current);
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
                push_dialog_to_back_stack(state, current);
            }
            state.open_dialog = Some(DialogState::PanelStack(stack));
            state.mark_dirty();
        }
        CR::None => {}
    }
}

// ============================================================================
// Open Dialog Functions
// ============================================================================

pub fn open_command_palette(state: &mut AppState) {
    let mut rows: Vec<crate::commands::CommandRow> = Vec::new();
    for cmd in state.registry.list() {
        rows.push(crate::commands::CommandRow::new(
            cmd.category.as_str(),
            &cmd.name,
            &cmd.desc,
            crate::Event::RunPaletteCommand {
                name: cmd.name.clone(),
                args: String::new(),
            },
        ));
    }
    for skill in &state.skills {
        if skill.user_invocable {
            rows.push(crate::commands::CommandRow::new(
                "Skill",
                &skill.name,
                &skill.description,
                crate::Event::RunSkillCommand {
                    name: skill.name.clone(),
                },
            ));
        }
    }
    state.open_dialog = Some(DialogState::CommandPalette(command_palette(rows)));
    state.mark_dirty();
}

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
    state.open_dialog = Some(DialogState::ModelSelector(model_selector(
        recent, groups, &current,
    )));
    state.mark_dirty();
}

pub fn open_settings_dialog(state: &mut AppState) {
    use crate::dialog::builders::{settings, SettingsRow, SettingsRowKind};
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
    state.open_dialog = Some(DialogState::Settings(settings(categories)));
    state.mark_dirty();
}

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

pub fn open_session_tree_dialog(state: &mut AppState) {
    let items: Vec<(usize, String, crate::Event)> = match state.session.session_tree.as_ref() {
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

/// Opens the file picker with an optional filter.
/// The filter is pre-filled with the given text to narrow results.
pub fn open_at_file_picker(state: &mut AppState, filter: Option<&str>) {
    use crate::dialog::{Panel, PanelStack};
    let entries = crate::file_refs::find_file_entries(".", 50);
    let mut panel = Panel::new("at-files", " Files ").with_filter();

    if let Some(f) = filter {
        panel.filter = f.to_string();
    }

    panel = if entries.is_empty() {
        panel.header("No files found")
    } else {
        build_file_picker_panel(panel, &entries, filter)
    };
    state.open_dialog = Some(DialogState::PanelStack(PanelStack::new(panel)));
    state.mark_dirty();
}

fn build_file_picker_panel(
    mut panel: crate::dialog::Panel,
    entries: &[crate::file_refs::FileEntry],
    filter: Option<&str>,
) -> crate::dialog::Panel {
    use crate::dialog::ItemAction;
    let header = file_picker_header(entries.len(), filter);
    panel = panel.header(&header);
    for entry in entries {
        let label = file_picker_label(entry);
        let insert_name = file_picker_insert_name(entry);
        panel = panel.item(
            &label,
            ItemAction::Emit(crate::Event::InsertAtRef(insert_name)),
        );
    }
    panel
}

fn file_picker_header(count: usize, filter: Option<&str>) -> String {
    if filter.map(|f| !f.is_empty()).unwrap_or(false) {
        format!("{} files matching '{}'", count, filter.unwrap_or(""))
    } else {
        format!("{} files", count)
    }
}

fn file_picker_label(entry: &crate::file_refs::FileEntry) -> String {
    if entry.is_dir {
        format!("{}/", entry.name)
    } else {
        entry.name.clone()
    }
}

fn file_picker_insert_name(entry: &crate::file_refs::FileEntry) -> String {
    if entry.is_dir {
        format!("{}/", entry.name)
    } else {
        entry.name.clone()
    }
}

/// Opens the file picker without any filter (shows all files).
pub fn open_at_file_picker_all(state: &mut AppState) {
    open_at_file_picker(state, None);
}
