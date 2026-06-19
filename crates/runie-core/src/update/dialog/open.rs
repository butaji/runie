//! Functions that open specific dialogs.

use crate::commands::DialogState;
use crate::dialog::builders::{
    command_palette, model_selector, provider_model_editor, scoped_models, session_tree,
};
use crate::model::AppState;

use super::build_file_picker_panel;
use crate::update::settings_dialog;

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
    let current = if state.has_models() {
        format!("{}/{}", state.config.current_provider, state.config.current_model)
    } else {
        String::new()
    };
    let configured = crate::login_config::list_configured_providers();
    let models = crate::model_catalog::configured_models_catalog(&configured);
    let items = crate::model_catalog::build_model_selector_items(
        &models,
        &state.config.recent_models,
        "",
        &state.config.current_provider,
        &state.config.current_model,
    );
    let (recent, groups) = super::model_selector::partition_model_items(items);
    state.open_dialog = Some(DialogState::ModelSelector(model_selector(
        recent, groups, &current,
    )));
    state.mark_dirty();
}

pub fn open_settings_dialog(state: &mut AppState) {
    use crate::dialog::builders::{settings, SettingsRow};
    let items = settings_dialog::build_setting_items(state);
    let mut categories: Vec<(String, Vec<SettingsRow>)> = Vec::new();
    for item in items {
        let cat_name = item.category.as_str().to_string();
        let row = SettingsRow {
            label: item.label,
            key: item.key,
            kind: item.value,
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

pub fn open_provider_model_editor(state: &mut AppState, provider: &str) {
    let configured = crate::login_config::list_configured_providers();
    let saved_models: Vec<String> = configured
        .iter()
        .find(|(p, _, _)| p == provider)
        .map(|(_, _, models)| models.clone())
        .unwrap_or_default();
    let saved_set: std::collections::HashSet<String> =
        saved_models.iter().cloned().collect();

    let mut available: Vec<String> = Vec::new();
    if let Some(meta) = crate::provider_registry::find_provider(provider) {
        for model in meta.models {
            available.push(model.name.to_string());
        }
    }
    for model in saved_models {
        if !available.contains(&model) {
            available.push(model);
        }
    }

    state.open_dialog = Some(DialogState::PanelStack(provider_model_editor(
        provider, &available, &saved_set,
    )));
    state.mark_dirty();
}

pub fn open_scoped_models_dialog(state: &mut AppState) {
    sync_scoped_models_with_config(state);
    let models: Vec<(String, String, bool)> = state
        .config
        .scoped_models
        .iter()
        .map(|m| (m.provider.clone(), m.name.clone(), m.enabled))
        .collect();
    state.open_dialog = Some(DialogState::ScopedModels(scoped_models(models)));
    state.mark_dirty();
}

fn sync_scoped_models_with_config(state: &mut AppState) {
    let configured = crate::login_config::list_configured_providers();
    if configured.is_empty() {
        return;
    }
    // Add any configured model that is missing, preserving the existing
    // enabled state when it is already present.
    for (provider, _, models) in configured {
        for name in models {
            let already_present = state
                .config
                .scoped_models
                .iter()
                .any(|m| m.provider == provider && m.name == name);
            if !already_present {
                state.config.scoped_models.push(crate::model::ScopedModel {
                    provider: provider.clone(),
                    name,
                    enabled: true,
                });
            }
        }
    }
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
    let entries = super::fff::query_fff_files(query, 50);
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

/// Opens the file picker without any filter (shows all files).
pub fn open_at_file_picker_all(state: &mut AppState) {
    open_at_file_picker(state, None);
}
