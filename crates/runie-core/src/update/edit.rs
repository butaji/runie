//! Form-submit and edit-event handling.

use crate::model::AppState;
use crate::Event;

pub(crate) fn update(state: &mut AppState, event: Event) {
    if let Some(_cmd) = state.edit_misc_event(&event) {
        return;
    }
    state.edit_run_event(event);
}

impl AppState {
    fn edit_misc_event(&mut self, event: &Event) -> Option<()> {
        match event {
            Event::PendingEdit {
                path,
                original,
                proposed,
                diff,
            } => {
                self.pending_edits
                    .push(crate::edit_preview::EditPreview::new(
                        std::path::PathBuf::from(path.clone()),
                        original.clone(),
                        proposed.clone(),
                        diff.clone(),
                    ));
                self.mark_dirty();
            }
            Event::ApproveEdit => self.approve_edits(),
            Event::RejectEdit => self.reject_edits(),
            Event::ReloadAll => self.reload_all(),
            Event::ShowDiagnostics => self.show_diagnostics(),
            Event::TogglePathCompletion => self.toggle_path_completion(),
            Event::PathCompletionUp => self.path_completion_up(),
            Event::PathCompletionDown => self.path_completion_down(),
            Event::PathCompletionSelect => self.path_completion_select(),
            Event::PathCompletionClose => self.path_completion_close(),
            _ => return None,
        }
        Some(())
    }

    fn edit_run_event(&mut self, event: Event) {
        match event {
            Event::RunSaveCommand { name } => self.run_save_command(&name),
            Event::RunLoadCommand { name } => self.run_load_command(&name),
            Event::RunDeleteCommand { name } => self.run_delete_command(&name),
            Event::RunExportCommand { path } => self.run_export_command(&path),
            Event::RunImportCommand { path } => self.run_import_command(&path),
            Event::RunSkillCommand { name } => self.run_skill_command(&name),
            Event::RunLoginCommand { provider, token } => self.run_login_command(&provider, &token),
            Event::RunLogoutCommand { provider } => self.run_logout_command(&provider),
            Event::RunNameCommand { name } => self.run_name_command(&name),
            Event::RunForkCommand { message_index } => self.run_fork_command(&message_index),
            Event::RunCompactCommand { keep, focus } => self.run_compact_command(&keep, &focus),
            Event::RunPromptCommand { name } => self.run_prompt_command(&name),
            Event::RunThinkingCommand { level } => self.run_thinking_command(level),
            Event::RunPaletteCommand { name, args } => self.run_palette_command(&name, &args),
            _ => {}
        }
    }

    fn run_palette_command(&mut self, name: &str, args: &str) {
        use crate::commands::CommandResult;
        let result = if let Some(cmd) = self.registry.get(name) {
            let cmd_name = cmd.name.clone();
            cmd.flow.clone().exec(self, &cmd_name, args)
        } else {
            CommandResult::Message(format!("Unknown command: /{}", name))
        };
        self.process_command_result(result);
    }

    fn run_save_command(&mut self, name: &str) {
        use crate::session::Session;
        let now = crate::update::now();
        let session = Session {
            name: name.to_string(),
            display_name: self.session.session_display_name.clone(),
            created_at: self.session.session_created_at,
            updated_at: now,
            messages: self.session.messages.clone(),
            provider: self.config.current_provider.clone(),
            model: self.config.current_model.clone(),
            theme_name: self.config.theme_name.clone(),
            thinking_level: self.config.thinking_level,
            read_only: self.config.read_only,
            session_tree: self.session.session_tree.clone(),
        };
        match crate::session::save(name, &session) {
            Ok(()) => {
                self.session.session_updated_at = now;
                self.add_system_msg(format!("Session '{}' saved.", name));
            }
            Err(e) => self.add_system_msg(format!("Could not save '{}': {}", name, e)),
        }
    }

    fn run_load_command(&mut self, name: &str) {
        match crate::session::load(name) {
            Ok(session) => {
                self.session.messages = session.messages;
                self.config.current_provider = session.provider;
                self.config.current_model = session.model;
                self.config.theme_name = session.theme_name;
                self.config.thinking_level = session.thinking_level;
                self.config.read_only = session.read_only;
                self.session.session_display_name = session.display_name.or(Some(session.name));
                self.session.session_created_at = session.created_at;
                self.session.session_updated_at = session.updated_at;
                self.session.session_tree = session.session_tree;
                self.messages_changed();
                self.add_system_msg(format!("Session '{}' loaded.", name));
            }
            Err(_) => self.add_system_msg(format!(
                "Session '{}' not found. Use /sessions to list saved sessions.",
                name
            )),
        }
    }

    fn run_delete_command(&mut self, name: &str) {
        match crate::session::delete(name) {
            Ok(()) => self.add_system_msg(format!("Session '{}' deleted.", name)),
            Err(_) => self.add_system_msg(format!(
                "Session '{}' not found. Use /sessions to list saved sessions.",
                name
            )),
        }
    }

    fn run_export_command(&mut self, path: &str) {
        use crate::session::Session;
        let session = Session {
            name: self
                .session
                .session_display_name
                .clone()
                .unwrap_or_else(|| "exported".into()),
            display_name: self.session.session_display_name.clone(),
            created_at: self.session.session_created_at,
            updated_at: crate::update::now(),
            messages: self.session.messages.clone(),
            provider: self.config.current_provider.clone(),
            model: self.config.current_model.clone(),
            theme_name: self.config.theme_name.clone(),
            thinking_level: self.config.thinking_level,
            read_only: self.config.read_only,
            session_tree: self.session.session_tree.clone(),
        };
        match std::fs::write(
            path,
            serde_json::to_string_pretty(&session).unwrap_or_default(),
        ) {
            Ok(()) => self.add_system_msg(format!("Session exported to '{}'", path)),
            Err(e) => self.add_system_msg(format!("Could not export: {}", e)),
        }
    }

    fn run_import_command(&mut self, path: &str) {
        match std::fs::read_to_string(path) {
            Ok(json) => match serde_json::from_str::<crate::session::Session>(&json) {
                Ok(session) => {
                    self.session.messages = session.messages;
                    self.config.current_provider = session.provider;
                    self.config.current_model = session.model;
                    self.config.theme_name = session.theme_name;
                    self.config.thinking_level = session.thinking_level;
                    self.config.read_only = session.read_only;
                    self.session.session_display_name = session.display_name.or(Some(session.name));
                    self.session.session_created_at = session.created_at;
                    self.session.session_updated_at = session.updated_at;
                    self.session.session_tree = session.session_tree;
                    self.messages_changed();
                    self.add_system_msg(format!("Session imported from '{}'", path));
                }
                Err(e) => self.add_system_msg(format!("Invalid session file: {}", e)),
            },
            Err(e) => self.add_system_msg(format!("Could not read file: {}", e)),
        }
    }

    fn run_skill_command(&mut self, name: &str) {
        match self.skills.iter().find(|s| s.name == name) {
            Some(skill) => {
                let mut lines = vec![format!("Skill: {}", skill.name)];
                if !skill.description.is_empty() {
                    lines.push(format!("Description: {}", skill.description));
                }
                if !skill.context.is_empty() {
                    lines.push(format!("Context: {}", skill.context));
                }
                self.add_system_msg(lines.join("\n"));
            }
            None => self.add_system_msg(format!(
                "Skill '{}' not found. Use /skills to list loaded skills.",
                name
            )),
        }
    }

    fn run_login_command(&mut self, provider: &str, token: &str) {
        if provider.is_empty() || token.is_empty() {
            self.add_system_msg("Usage: /login provider token".into());
            return;
        }
        let mut storage = crate::auth::AuthStorage::load();
        storage.set(provider, token, None);
        match storage.save() {
            Ok(()) => self.add_system_msg(format!("Logged in to '{}'.", provider)),
            Err(e) => self.add_system_msg(format!("Could not save token: {}", e)),
        }
    }

    fn run_logout_command(&mut self, provider: &str) {
        if provider.is_empty() {
            self.add_system_msg("Usage: /logout provider".into());
            return;
        }
        let mut storage = crate::auth::AuthStorage::load();
        storage.remove(provider);
        match storage.save() {
            Ok(()) => self.add_system_msg(format!("Logged out from '{}'.", provider)),
            Err(e) => self.add_system_msg(format!("Could not remove token: {}", e)),
        }
    }

    fn run_name_command(&mut self, name: &str) {
        crate::commands::handlers::session::run_name(self, name)
    }

    fn run_fork_command(&mut self, index_raw: &str) {
        crate::commands::handlers::session::run_fork(self, index_raw)
    }

    fn run_compact_command(&mut self, keep_raw: &str, focus: &str) {
        crate::commands::handlers::session::run_compact(self, keep_raw, focus)
    }

    fn run_prompt_command(&mut self, name: &str) {
        crate::commands::handlers::system::run_prompt(self, name)
    }

    fn run_thinking_command(&mut self, level: crate::model::ThinkingLevel) {
        crate::commands::handlers::model::run_thinking(self, level)
    }

}
