//! Dialog opening and top-level routing.

use git2;
use crate::commands::{DialogState, DialogType};
use crate::{FffSearchState, FffFileEntry};
use crate::dialog::builders::{command_palette, model_selector, scoped_models, session_tree};
use crate::event::{ControlEvent, DialogEvent};
use crate::model::AppState;
use crate::model_catalog::model_catalog;

use crate::Event;

use super::settings_dialog;

/// Query the global FFF index for fuzzy file search results.
/// Returns up to `limit` entries ranked by frecency + fuzzy score.
/// Query FFF indexer for files. Falls back to file_refs if FFF is unavailable or
/// returns no results (e.g. not indexed yet in tests).
fn query_fff_files(query: &str, limit: usize) -> Vec<FffFileEntry> {
    // Primary: use file_refs (deterministic, sorted by name).
    let fallback: Vec<FffFileEntry> = crate::file_refs::find_file_entries(".", limit)
        .into_iter()
        .map(|e| FffFileEntry {
            name: e.name.clone(),
            path: e.name,
            is_dir: e.is_dir,
            score: 0.0,
            git_status: None,
        })
        .collect();

    let Some(state) = FffSearchState::get() else {
        return fallback;
    };

    let picker_guard = match state.picker.read() {
        Ok(g) => g,
        Err(_) => return fallback,
    };
    let qt_guard = match state.query_tracker.read() {
        Ok(g) => g,
        Err(_) => return fallback,
    };

    let picker = match picker_guard.as_ref() {
        Some(p) => p,
        None => return fallback,
    };

    let parsed = fff_search::QueryParser::default().parse(query);
    let results = picker.fuzzy_search(
        &parsed,
        qt_guard.as_ref(),
        fff_search::FuzzySearchOptions {
            max_threads: 0,
            current_file: None,
            project_path: None,
            pagination: fff_search::PaginationArgs {
                offset: 0,
                limit,
            },
            combo_boost_score_multiplier: 100,
            min_combo_count: 2,
        },
    );

    // If FFF returned no results (not indexed yet), use file_refs fallback.
    if results.items.is_empty() {
        return fallback;
    }

    results
        .items
        .iter()
        .zip(results.scores.iter())
        .map(|(item, score)| {
            let path = item.relative_path(picker);
            let name = std::path::Path::new(&path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.clone());
            let is_dir = path.ends_with('/') || item.relative_path(picker).is_empty();
            let git_status = item.git_status.map(format_fff_git_status);
            FffFileEntry {
                name,
                path,
                is_dir,
                score: score.total as f64,
                git_status,
            }
        })
        .collect()
}

fn format_fff_git_status(status: git2::Status) -> String {
    use git2::Status as G;
    if status.contains(G::WT_NEW) || status.contains(G::INDEX_NEW) {
        "untracked".to_string()
    } else if status.contains(G::WT_MODIFIED) || status.contains(G::INDEX_MODIFIED) {
        "modified".to_string()
    } else if status.contains(G::WT_DELETED) || status.contains(G::INDEX_DELETED) {
        "deleted".to_string()
    } else if status.contains(G::WT_RENAMED) || status.contains(G::INDEX_RENAMED) {
        "renamed".to_string()
    } else {
        String::new()
    }
}

mod form;
mod model_selector;
mod panel;
mod tab_complete;
pub mod toggle;

pub use toggle::dialog_toggle_event;

#[cfg(test)]
pub use form::{form_panel_action, FormAction};

// ── Form panel free functions (merged from dialog_form) ────────────────────────

/// Handle command form dialog events. Entry point for `CommandForm*` events.
pub fn handle_form_dialog(state: &mut AppState, event: DialogEvent) {
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
        form::form_panel_action(panel, event)
    };
    form::apply_form_action(state, action);
}

/// Insert filepath into input and close any dialog.
pub fn insert_at_ref(state: &mut AppState, path: &str) {
    // Record frecency for the selected file.
    if let Some(fff_state) = crate::actors::FffSearchState::get() {
        fff_state.record_file_access(std::path::Path::new(path));
    }
    state.input.input = build_insert_text(state, path);
    state.input.cursor_pos = state.input.input.len();
    state.open_dialog = None;
    state.mark_dirty();
}

fn build_insert_text(state: &mut AppState, path: &str) -> String {
    let Some((original_input, insert_pos, cursor, _)) = state.input.file_picker_backup.take() else {
        return path.to_string();
    };
    let before = compute_before_text(&original_input, insert_pos, cursor);
    let after = extract_after(&original_input, cursor);
    // Append the range suffix if one was present (e.g. `:10-50`).
    let suffix = state.input.file_picker_range_suffix.take().unwrap_or_default();
    format!("{}{}{}{}", before, path, suffix, after)
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

// ── Panel stack routing (merged from dialog_panel) ────────────────────────────

/// Handles dialog-specific events. Returns whether the dialog was closed.
pub fn update_dialog(state: &mut AppState, event: Event) {
    if route_global_dialog_event(state, &event) {
        return;
    }
    let Some(mut dialog) = state.open_dialog.take() else {
        return;
    };
    // Welcome dialog has no panel stack — only close on specific events
    if matches!(dialog, crate::commands::DialogState::Welcome) {
        state.open_dialog = Some(dialog);
        return;
    }
    let is_dialog_back = matches!(&event, crate::event::DialogEvent::DialogBack);
    let is_palette_activation = is_palette_activation(&dialog, &event);
    if is_palette_activation {
        state.push_dialog_to_back_stack(dialog.clone());
    }
    let stack = dialog.panel_stack_mut().expect("non-welcome dialog has panel stack");
    let activated = panel::update_panel_stack(state, event, stack);
    restore_or_pop_dialog(state, dialog, activated, is_palette_activation);

    if is_dialog_back && state.open_dialog.is_none() {
        state.handle_vim_dialog_back();
    }

    state.mark_dirty();
}

fn route_global_dialog_event(state: &mut AppState, event: &Event) -> bool {
    if matches!(event, ControlEvent::Abort) {
        if let Some((input, _, _, _)) = state.input.file_picker_backup.take() {
            state.input.input = input;
            state.input.cursor_pos = state.input.input.len();
        }
        state.input.file_picker_range_suffix = None;
        state.open_dialog = None;
        state.mark_dirty();
        return true;
    }
    if matches!(event, ControlEvent::Quit) {
        state.should_quit = true;
        return true;
    }
    false
}

fn is_palette_activation(dialog: &DialogState, event: &Event) -> bool {
    use crate::event::{InputEvent, DialogEvent};
    matches!(event, InputEvent::Submit | DialogEvent::PaletteSelect)
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

/// Toggle a dialog open/closed.
#[allow(dead_code)]
pub fn toggle_dialog(state: &mut AppState, is_same: bool, open: fn(&mut AppState)) {
    if is_same {
        state.open_dialog = None;
        state.mark_dirty();
    } else {
        open(state);
    }
}

// ── Command Result Processing ─────────────────────────────────────────────────

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

// ── Open Dialog Functions ───────────────────────────────────────────────────────

pub fn open_command_palette(state: &mut AppState) {
    let mut rows: Vec<crate::commands::CommandRow> = Vec::new();
    let ranked = state.rank_commands("", 100);
    for (cmd, _score) in ranked {
        rows.push(crate::commands::CommandRow::new(
            cmd.category.as_str(),
            &cmd.name,
            &cmd.desc,
            crate::event::CommandEvent::RunPaletteCommand {
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
                crate::event::CommandEvent::RunSkillCommand {
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
    let (recent, groups) = model_selector::partition_model_items(items);
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
                let evt = crate::event::SessionEvent::SessionTreeSelect {
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
/// If the filter contains a `:start-end` range suffix (e.g. `@src/main.rs:10-50`),
/// the base path is used for filtering and the range suffix is appended on insertion.
pub fn open_at_file_picker(state: &mut AppState, filter: Option<&str>) {
    use crate::dialog::{Panel, PanelStack};

    // Strip the range suffix from the filter so the picker matches on the base path.
    let (base_filter, range_suffix) = match filter {
        Some(f) => {
            if let Some(parsed) = crate::file_refs::parse_file_ref(f) {
                let suffix = parsed.range.map(|r| format!(":{}-{}", r.start(), r.end()));
                (Some(parsed.path), suffix)
            } else {
                (Some(f.to_string()), None)
            }
        }
        None => (None, None),
    };

    // Store the range suffix so `insert_at_ref` can append it after the selected file.
    state.input.file_picker_range_suffix = range_suffix;

    // Query FFF for file results.
    let query = base_filter.as_deref().unwrap_or("");
    let entries = query_fff_files(query, 50);
    state.fff_file_results = entries.clone();
    state.fff_debounce = state.fff_debounce.wrapping_add(1);

    let mut panel = Panel::new("at-files", " Files ").with_filter();

    if let Some(ref f) = base_filter {
        panel.filter = f.clone();
    }

    panel = if entries.is_empty() {
        panel.header("No files found")
    } else {
        build_file_picker_panel(panel, &entries, base_filter.as_deref())
    };
    state.open_dialog = Some(DialogState::PanelStack(PanelStack::new(panel)));
    state.mark_dirty();
}

fn build_file_picker_panel(
    mut panel: crate::dialog::Panel,
    entries: &[FffFileEntry],
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
            ItemAction::Emit(crate::event::DialogEvent::InsertAtRef(insert_name)),
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

fn file_picker_label(entry: &FffFileEntry) -> String {
    if entry.is_dir {
        format!("{}/", entry.name)
    } else {
        entry.name.clone()
    }
}

fn file_picker_insert_name(entry: &FffFileEntry) -> String {
    // Use full relative path so frecency can record access by path.
    if entry.is_dir {
        format!("{}/", entry.path)
    } else {
        entry.path.clone()
    }
}

/// Rebuild the file picker panel with the current FFF results and panel filter.
/// Called when the user types in the file picker to update fuzzy results.
fn rebuild_file_picker(state: &mut AppState) {
    use crate::dialog::{Panel, PanelStack, ItemAction};

    let Some(DialogState::PanelStack(ref stack)) = state.open_dialog else {
        return;
    };
    let Some(panel) = stack.current() else {
        return;
    };
    let filter = panel.filter.clone();

    // Re-query FFF with the new filter.
    let query = if filter.is_empty() { "" } else { &filter };
    let entries = query_fff_files(query, 50);
    state.fff_file_results = entries.clone();
    state.fff_debounce = state.fff_debounce.wrapping_add(1);

    let mut new_panel = Panel::new("at-files", " Files ").with_filter();
    new_panel.filter = filter.clone();

    let count = entries.len();
    new_panel = if entries.is_empty() {
        new_panel.header("No files found")
    } else {
        let header = file_picker_header(count, Some(&filter));
        new_panel = new_panel.header(&header);
        for entry in entries {
            let label = file_picker_label(&entry);
            let insert_name = file_picker_insert_name(&entry);
            new_panel = new_panel.item(
                &label,
                ItemAction::Emit(crate::event::DialogEvent::InsertAtRef(insert_name)),
            );
        }
        new_panel
    };

    state.open_dialog = Some(DialogState::PanelStack(PanelStack::new(new_panel)));
    state.mark_dirty();
}

/// Opens the file picker without any filter (shows all files).
pub fn open_at_file_picker_all(state: &mut AppState) {
    open_at_file_picker(state, None);
}
