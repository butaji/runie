//! Functions that open specific dialogs.

use crate::commands::{DialogKind, DialogState};
use crate::dialog::builders::{command_palette, model_selector, scoped_models, session_tree};
use crate::model::{AppState, InputReceiver};

use super::build_file_picker_panel;
use crate::update::settings_dialog;

pub fn open_command_palette(state: &mut AppState) {
    open_command_palette_with_filter(state, "");
}

/// Opens the command palette with an optional initial filter.
/// When typing `/` in the input, any existing text is passed as the initial filter.
pub fn open_command_palette_with_filter(state: &mut AppState, initial_filter: &str) {
    let mut rows: Vec<crate::commands::CommandRow> = Vec::new();
    let ranked = state.rank_commands("", 100);
    for (cmd, _score) in ranked {
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
    for skill in state.skills() {
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
    let mut stack = command_palette(rows);
    // Set initial filter if provided (e.g. when `/` is typed with existing text)
    if !initial_filter.is_empty() {
        if let Some(panel) = stack.current_mut() {
            panel.set_filter(initial_filter);
        }
    }
    let v = state.view_mut();
    v.input_receiver = InputReceiver::Dialog;
    v.dirty = true;
    *state.open_dialog_mut() = Some(DialogState::Active { kind: DialogKind::CommandPalette, panels: stack });
}

pub fn open_model_selector(state: &mut AppState) {
    let current = if state.has_models() {
        format!(
            "{}/{}",
            state.config().current_provider,
            state.config().current_model
        )
    } else {
        String::new()
    };
    let configured = state.configured_providers();
    let models = crate::model_catalog::configured_models_catalog(&configured);
    let items = crate::model_catalog::build_model_selector_items(
        &models,
        state.config().recent_models.as_slice(),
        "",
        &state.config().current_provider,
        &state.config().current_model,
    );
    let (recent, groups) = super::toggles::partition_model_items(items);
    let v = state.view_mut();
    v.input_receiver = InputReceiver::Dialog;
    v.dirty = true;
    *state.open_dialog_mut() = Some(DialogState::Active { kind: DialogKind::ModelSelector, panels: model_selector(
        recent, groups, &current,
    ) });
}

pub fn open_settings_dialog(state: &mut AppState) {
    use crate::dialog::builders::settings;

    let categories: Vec<(String, Vec<_>)> = settings_dialog::build_setting_categories(state)
        .into_iter()
        .map(|(cat, items)| (cat.as_str().to_owned(), items))
        .collect();
    let v = state.view_mut();
    v.input_receiver = InputReceiver::Dialog;
    v.dirty = true;
    *state.open_dialog_mut() = Some(DialogState::Active { kind: DialogKind::Settings, panels: settings(categories) });
}

pub fn open_scoped_models_dialog(state: &mut AppState) {
    sync_scoped_models_with_config(state);
    let models: Vec<(String, String, bool)> = state
        .config()
        .scoped_models
        .iter()
        .map(|m| (m.provider.clone(), m.name.clone(), m.enabled))
        .collect();
    let v = state.view_mut();
    v.input_receiver = InputReceiver::Dialog;
    v.dirty = true;
    *state.open_dialog_mut() = Some(DialogState::Active { kind: DialogKind::ScopedModels, panels: scoped_models(models) });
}

pub fn open_theme_selector(state: &mut AppState) {
    use crate::dialog::{ItemAction, Panel, PanelStack};

    let mut panel = Panel::new("theme", "Choose Theme")
        .header("available themes")
        .keep_open();
    for theme in crate::theme_tokens::BUILTIN_THEMES {
        panel = panel.item(
            *theme,
            ItemAction::Emit(crate::Event::SwitchTheme {
                name: theme.to_string(),
            }),
        );
    }
    let v = state.view_mut();
    v.input_receiver = InputReceiver::Dialog;
    v.dirty = true;
    *state.open_dialog_mut() = Some(DialogState::Active { kind: DialogKind::Generic, panels: PanelStack::new(panel) });
}

fn sync_scoped_models_with_config(state: &mut AppState) {
    let configured = state.configured_providers();
    if configured.is_empty() {
        return;
    }
    // Add any configured model that is missing, preserving the existing
    // enabled state when it is already present.
    for (provider, _, models) in configured {
        for name in models {
            let already_present = state
                .config()
                .scoped_models
                .iter()
                .any(|m| m.provider == provider && m.name == name);
            if !already_present {
                state
                    .config_mut()
                    .scoped_models
                    .push(crate::model::ScopedModel {
                        provider: provider.clone(),
                        name,
                        enabled: true,
                    });
            }
        }
    }
}

pub fn open_session_tree_dialog(state: &mut AppState) {
    let items: Vec<(usize, String, crate::Event)> = match state.session().session_tree.as_ref() {
        Some(tree) => tree
            .filtered_walk(crate::session::tree::SessionTreeFilter::All)
            .into_iter()
            .map(|(depth, node)| {
                let preview = format!(
                    "[{}] {}",
                    node.message.role.as_str(),
                    node.message.content().chars().take(60).collect::<String>()
                );
                let evt = crate::Event::SessionTreeSelect {
                    id: node.message.id.clone(),
                };
                (depth, preview, evt)
            })
            .collect(),
        None => Vec::new(),
    };
    let v = state.view_mut();
    v.input_receiver = InputReceiver::Dialog;
    v.dirty = true;
    *state.open_dialog_mut() = Some(DialogState::Active { kind: DialogKind::SessionTree, panels: session_tree(items) });
}

fn parse_filter(filter: Option<&str>) -> (Option<String>, Option<String>) {
    match filter {
        Some(f) => {
            if let Some(parsed) = crate::file_refs::parse_file_ref(f) {
                let suffix = parsed.range.map(|r| format!(":{}-{}", r.start(), r.end()));
                (Some(parsed.path), suffix)
            } else {
                (Some(f.to_owned()), None)
            }
        }
        None => (None, None),
    }
}

/// Opens the file picker with an optional filter.
/// If the filter contains a `:start-end` range suffix (e.g. `@src/main.rs:10-50`),
/// the base path is used for filtering and the range suffix is appended on insertion.
pub fn open_at_file_picker(state: &mut AppState, filter: Option<&str>) {
    use crate::dialog::{Panel, PanelStack};

    let (base_filter, range_suffix) = parse_filter(filter);
    state.input_mut().file_picker_range_suffix = range_suffix;

    let query = base_filter.as_deref().unwrap_or("");
    refresh_file_picker_search(state, query);

    let entries = state.fff_file_results();
    let mut panel = Panel::new("at-files", " Files ").with_filter();
    if let Some(ref f) = base_filter {
        panel.filter = f.clone();
    }
    panel = if entries.is_empty() {
        panel.header("No files found")
    } else {
        build_file_picker_panel(panel, entries, base_filter.as_deref())
    };
    let v = state.view_mut();
    v.input_receiver = InputReceiver::Dialog;
    v.dirty = true;
    *state.open_dialog_mut() = Some(DialogState::Active { kind: DialogKind::Generic, panels: PanelStack::new(panel) });
}

/// Send a file search request to `FffIndexerActor`.
/// Results arrive asynchronously via `Event::FffSearchResult`.
pub(crate) fn refresh_file_picker_search(state: &mut AppState, query: &str) {
    let Some(handles) = state.actor_handles() else { return };
    let Some(ref fff) = handles.fff_indexer else { return };

    let request_id = state.fff_debounce().wrapping_add(1);
    let request = crate::actors::FffSearchRequest {
        request_id,
        query: query.to_owned(),
        limit: Some(50),
        project_path: std::env::current_dir().unwrap_or_default(),
    };
    fff.try_search(request);
    *state.fff_debounce_mut() = request_id;
}

/// Opens the file picker without any filter (shows all files).
pub fn open_at_file_picker_all(state: &mut AppState) {
    open_at_file_picker(state, None);
}
