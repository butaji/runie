//! Dialog opening helpers.

use crate::model::AppState;
use super::{scoped_models, settings_dialog};
use crate::Event;

impl AppState {
    pub(crate) fn dialog_toggle_event(&mut self, event: Event) {
        use crate::commands::DialogState;
        match event {
            Event::ToggleCommandPalette => self.open_command_palette(),
            Event::ToggleModelSelector => self.toggle_dialog(
                matches!(self.open_dialog, Some(DialogState::ModelSelector(_))),
                Self::open_model_selector,
            ),
            Event::ToggleScopedModelsDialog => self.toggle_dialog(
                matches!(self.open_dialog, Some(DialogState::ScopedModels(_))),
                Self::open_scoped_models_dialog,
            ),
            Event::ToggleSettingsDialog => self.toggle_dialog(
                matches!(self.open_dialog, Some(DialogState::Settings(_))),
                Self::open_settings_dialog,
            ),
            Event::ToggleSessionTree => self.toggle_dialog(
                matches!(self.open_dialog, Some(DialogState::SessionTree(_))),
                Self::open_session_tree_dialog,
            ),
            Event::AtFilePicker => self.open_at_file_picker(),
            Event::ScopedModelToggle { name } => scoped_models::toggle_scoped_model(self, &name),
            Event::ScopedModelEnableAll => scoped_models::enable_all(self),
            Event::ScopedModelDisableAll => scoped_models::disable_all(self),
            Event::ScopedModelToggleProvider { provider } => {
                scoped_models::toggle_provider(self, &provider)
            }
            _ => {}
        }
    }

    fn toggle_dialog(&mut self, is_same: bool, open: fn(&mut Self)) {
        if is_same {
            self.open_dialog = None;
            self.mark_dirty();
        } else {
            open(self);
        }
    }

    pub(crate) fn open_command_palette(&mut self) {
        use crate::dialog::builders::command_palette;
        let mut items: Vec<(String, String, crate::Event)> = Vec::new();
        for cmd in self.registry.list() {
            let evt = crate::Event::RunPaletteCommand {
                name: cmd.name.clone(),
                args: String::new(),
            };
            items.push((
                cmd.category.as_str().to_string(),
                format!("{} {}", cmd.name, cmd.desc),
                evt,
            ));
        }
        for skill in &self.skills {
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
        self.open_dialog = Some(crate::commands::DialogState::CommandPalette(
            command_palette(items),
        ));
        self.mark_dirty();
    }

    pub(crate) fn open_model_selector(&mut self) {
        use crate::dialog::builders::model_selector;
        use crate::model_catalog::{build_model_selector_items, model_catalog};
        let current = format!(
            "{}/{}",
            self.config.current_provider, self.config.current_model
        );
        let items = build_model_selector_items(
            &model_catalog(),
            &self.recent_models,
            "",
            &self.config.current_provider,
            &self.config.current_model,
        );
        let (recent, groups) = partition_model_items(items);
        self.open_dialog = Some(crate::commands::DialogState::ModelSelector(model_selector(
            recent, groups, &current,
        )));
        self.mark_dirty();
    }

    pub(crate) fn open_settings_dialog(&mut self) {
        use crate::dialog::builders::{settings, SettingsRow, SettingsRowKind};
        use crate::settings::SettingValue;
        let items = settings_dialog::build_setting_items(self);
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
        self.open_dialog = Some(crate::commands::DialogState::Settings(settings(categories)));
        self.mark_dirty();
    }

    pub(crate) fn open_scoped_models_dialog(&mut self) {
        use crate::dialog::builders::scoped_models;
        let models: Vec<(String, String, bool)> = self
            .config
            .scoped_models
            .iter()
            .map(|m| (m.provider.clone(), m.name.clone(), m.enabled))
            .collect();
        self.open_dialog = Some(crate::commands::DialogState::ScopedModels(scoped_models(
            models,
        )));
        self.mark_dirty();
    }

    pub(crate) fn open_session_tree_dialog(&mut self) {
        use crate::dialog::builders::session_tree;
        let items: Vec<(usize, String, crate::Event)> = match self.session.session_tree.as_ref() {
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
        self.open_dialog = Some(crate::commands::DialogState::SessionTree(session_tree(
            items,
        )));
        self.mark_dirty();
    }

    // === Settings Event Handler ===
    // === Edit Event Handler ===

    pub(crate) fn open_at_file_picker(&mut self) {
        use crate::dialog::{ItemAction, Panel, PanelStack};
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
                panel = panel.item(
                    &label,
                    ItemAction::Emit(crate::Event::InsertAtRef(insert_name)),
                );
            }
        }
        self.open_dialog = Some(crate::commands::DialogState::PanelStack(PanelStack::new(
            panel,
        )));
        self.mark_dirty();
    }

}

#[allow(clippy::type_complexity)]
fn partition_model_items(
    items: Vec<(String, String, String, bool, bool)>,
) -> (Vec<String>, Vec<(String, Vec<(String, crate::Event)>)>) {
    let mut recent: Vec<String> = Vec::new();
    let mut groups: Vec<(String, Vec<(String, crate::Event)>)> = Vec::new();
    let mut last_header = String::new();
    let mut current_group: Vec<(String, crate::Event)> = Vec::new();
    for (header, name, _cost, _is_selected, _is_current) in items {
        if header == "Recent" {
            recent.push(name);
            continue;
        }
        if !header.is_empty() && header != last_header {
            if !current_group.is_empty() {
                groups.push((last_header.clone(), std::mem::take(&mut current_group)));
            }
            last_header = header.clone();
        }
        if let Some((provider, model)) = name.split_once('/') {
            let evt = crate::Event::SwitchModel {
                provider: provider.to_string(),
                model: model.to_string(),
            };
            current_group.push((name, evt));
        }
    }
    if !current_group.is_empty() {
        groups.push((last_header, current_group));
    }
    (recent, groups)
}
