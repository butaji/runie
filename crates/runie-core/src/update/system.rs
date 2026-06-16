//! Shared state helpers used by multiple update handlers.

use crate::event::TransientLevel;
use crate::model::{AppState, ChatMessage, Role};

impl AppState {
    pub(crate) fn push_dialog_to_back_stack(&mut self, dialog: crate::commands::DialogState) {
        self.dialog_back_stack.push(dialog);
    }

    pub fn peek_queue(&self) -> Option<&(String, String)> {
        self.agent.request_queue.front()
    }

    pub fn pop_queue(&mut self) -> Option<(String, String)> {
        self.agent.request_queue.pop_front()
    }

    pub(crate) fn set_transient(&mut self, content: String, level: TransientLevel) {
        self.transient_message = Some(content);
        self.transient_level = Some(level);
        self.transient_until = match level {
            TransientLevel::Error => None,
            _ => Some(std::time::Instant::now() + std::time::Duration::from_secs(5)),
        };
        self.mark_dirty();
    }

    pub(crate) fn clear_transient(&mut self) {
        self.transient_message = None;
        self.transient_until = None;
        self.transient_level = None;
        self.mark_dirty();
    }

    pub(crate) fn add_system_msg(&mut self, content: String) {
        self.session.messages.push(ChatMessage {
            role: Role::System,
            content,
            timestamp: crate::update::now(),
            id: "system".to_string(),
            ..Default::default()
        });
        self.messages_changed();
    }

    /// Emit a transient notification in the hints line (not in the feed).
    pub(crate) fn notify(&mut self, content: String, level: TransientLevel) {
        self.set_transient(content, level);
    }

    /// Move TurnComplete to the end of messages and bump its timestamp.
    /// Called after every agent event to ensure TurnComplete remains last.
    /// Only moves the TurnComplete for the current turn (matching current_request_id
    /// or falling back to the last assistant message's id), so earlier turns'
    /// TurnComplete are not affected.
    pub(crate) fn ensure_turn_complete_last(&mut self) {
        let target_id = self.agent.current_request_id.clone().or_else(|| {
            self.agent
                .last_assistant_index
                .and_then(|idx| self.session.messages.get(idx).map(|m| m.id.clone()))
        });
        let Some(target_id) = target_id else {
            return;
        };
        if let Some(idx) = self
            .session
            .messages
            .iter()
            .position(|m| m.role == Role::TurnComplete && m.id == target_id)
        {
            let mut tc = self.session.messages.remove(idx);
            tc.timestamp = crate::update::now();
            self.session.messages.push(tc);
            self.messages_changed();
        }
    }

    // === View Helpers ===

    pub(crate) fn toggle_expand_all(&mut self) {
        self.view.all_collapsed = !self.view.all_collapsed;
        self.messages_changed();
    }

    pub(crate) fn page_up(&mut self) {
        crate::update::input::scroll_event(self, crate::event::ScrollEvent::PageUp);
    }

    pub(crate) fn page_down(&mut self) {
        crate::update::input::scroll_event(self, crate::event::ScrollEvent::PageDown);
    }

    pub(crate) fn go_to_top(&mut self) {
        crate::update::input::scroll_event(self, crate::event::ScrollEvent::GoToTop);
    }

    pub(crate) fn go_to_bottom(&mut self) {
        crate::update::input::scroll_event(self, crate::event::ScrollEvent::GoToBottom);
    }

    // === Model / Config Helpers ===

    pub(crate) fn configure_token_tracker(&mut self) {
        self.agent.token_tracker = crate::tokens::token_tracker_for(
            &self.config.current_provider,
            &self.config.current_model,
        );
    }

    pub(crate) fn switch_model(&mut self, provider: String, model: String) {
        if self.config.current_provider == provider && self.config.current_model == model {
            return;
        }
        self.config.current_provider = provider.clone();
        self.config.current_model = model.clone();
        self.configure_token_tracker();
        self.record_model_usage(&provider, &model);
        self.config.telemetry.track_event("model_switch", {
            let mut m = std::collections::HashMap::new();
            m.insert("provider".into(), provider.clone());
            m.insert("model".into(), model.clone());
            m
        });
        self.notify(
            format!("Switched to {}/{}", provider, model),
            TransientLevel::Success,
        );
    }

    pub(crate) fn set_provider(&mut self, provider: &str) {
        if self.config.current_provider == provider {
            return;
        }
        let provider = provider.to_string();
        let model = first_model_for_provider(&provider)
            .unwrap_or_else(|| self.config.current_model.clone());
        self.switch_model(provider, model);
    }

    pub(crate) fn set_model(&mut self, model: &str) {
        if self.config.current_model == model {
            return;
        }
        let model = model.to_string();
        self.switch_model(self.config.current_provider.clone(), model);
    }

    pub(crate) fn switch_theme(&mut self, name: String) {
        if self.config.theme_name == name {
            return;
        }
        self.config.theme_name = name.clone();
        self.notify(
            format!("Theme switched to '{}'", name),
            TransientLevel::Success,
        );
    }

    pub(crate) fn cycle_model(&mut self, delta: isize) {
        let enabled: Vec<usize> = self
            .config
            .scoped_models
            .iter()
            .enumerate()
            .filter(|(_, m)| m.enabled)
            .map(|(i, _)| i)
            .collect();
        if enabled.is_empty() {
            return;
        }
        let current_pos = enabled
            .iter()
            .position(|&i| i == self.config.scoped_index)
            .unwrap_or(0);
        let len = enabled.len() as isize;
        let new_pos = ((current_pos as isize + delta).rem_euclid(len)) as usize;
        self.config.scoped_index = enabled[new_pos];
        let model = &self.config.scoped_models[self.config.scoped_index];
        self.switch_model(model.provider.clone(), model.name.clone());
    }

    pub(crate) fn cycle_thinking_level(&mut self) {
        self.config.thinking_level = self.config.thinking_level.cycle();
        self.notify(
            format!("Thinking level: {}", self.config.thinking_level.as_str()),
            TransientLevel::Info,
        );
    }

    pub(crate) fn set_thinking_level(&mut self, level: crate::model::ThinkingLevel) {
        self.config.thinking_level = level;
        self.notify(
            format!(
                "Thinking level set to: {}",
                self.config.thinking_level.as_str()
            ),
            TransientLevel::Info,
        );
    }

    pub(crate) fn toggle_read_only(&mut self) {
        self.config.read_only = !self.config.read_only;
        let status = if self.config.read_only {
            "enabled"
        } else {
            "disabled"
        };
        self.notify(
            format!("Read-only mode {}", status),
            TransientLevel::Warning,
        );
    }

    pub(crate) fn trust_project(&mut self) {
        let cwd = std::env::current_dir().unwrap_or_default();
        let mut tm = crate::trust::TrustManager::load();
        tm.set(&cwd, crate::trust::TrustDecision::Trusted);
        let _ = tm.save();
        self.config.read_only = false;
        self.notify(
            format!("Project '{}' trusted. Read-only disabled.", cwd.display()),
            TransientLevel::Success,
        );
    }

    pub(crate) fn untrust_project(&mut self) {
        let cwd = std::env::current_dir().unwrap_or_default();
        let mut tm = crate::trust::TrustManager::load();
        tm.set(&cwd, crate::trust::TrustDecision::Untrusted);
        let _ = tm.save();
        self.config.read_only = true;
        self.notify(
            format!("Project '{}' untrusted. Read-only enabled.", cwd.display()),
            TransientLevel::Warning,
        );
    }

    pub(crate) fn stop_turn(&mut self) {
        // Stop the current turn and abort any queued messages
        self.agent.turn_active = false;
        self.agent.current_request_id = None;
        self.agent.streaming = false;
        self.agent.current_tool_name = None;
        self.agent.current_action = None;
        self.agent.inflight = 0;
        // Drain the queue back to input
        for msg in self.agent.message_queue.drain(..).rev() {
            if !self.input.input.is_empty() {
                self.input.input.push('\n');
            }
            self.input.input.push_str(&msg.content);
        }
        self.mark_dirty();
    }
}

fn first_model_for_provider(provider: &str) -> Option<String> {
    crate::model_catalog::model_catalog()
        .iter()
        .find(|m| m.provider == provider)
        .map(|m| m.name.clone())
}

// ── Control Event Handler (merged from control.rs) ────────────────────────────

use crate::event::ControlEvent;
use crate::model::AppState as AppState2;

pub fn control_event(state: &mut AppState, event: ControlEvent) {
    match event {
        ControlEvent::Quit => handle_quit(state),
        ControlEvent::Reset => handle_reset(state),
        ControlEvent::Abort => handle_abort(state),
        ControlEvent::ExternalEditorDone { content } => handle_editor_done(state, content),
        ControlEvent::ToggleExpand => state.toggle_expand_all(),
        ControlEvent::FollowUp => state.queue_follow_up(),
        ControlEvent::Dequeue => state.dequeue(),
        ControlEvent::ToggleVimMode => {
            state.config.vim_mode = !state.config.vim_mode;
            state.view.cached_settings_valid = false;
        }
        ControlEvent::NewSession => {
            // Close welcome screen if open
            if matches!(state.open_dialog, Some(crate::commands::DialogState::Welcome)) {
                state.open_dialog = None;
            }
            // Ready for user input — welcome is gone
            state.mark_dirty();
        }
        ControlEvent::ResumeSession | ControlEvent::OpenSessionList => {
            // Close welcome and open session tree
            state.open_dialog = Some(crate::commands::DialogState::Welcome);
            state.mark_dirty();
        }
        ControlEvent::SpawnAgent { .. }
        | ControlEvent::Suspend
        | ControlEvent::ShareSession
        | ControlEvent::OpenExternalEditor => {}
        _ => {}
    }
}

fn handle_quit(state: &mut AppState) {
    if !state.input.input.is_empty() {
        state.input.input.clear();
        state.input.cursor_pos = 0;
        state.input.input_scroll = 0;
        state.input.undo_stack.clear();
        state.input.redo_stack.clear();
        state.mark_dirty();
    } else {
        state.should_quit = true;
    }
}

fn handle_reset(state: &mut AppState) {
    *state = AppState::default();
}

fn handle_abort(state: &mut AppState) {
    if state.completion.path_suggestions.is_some() {
        state.path_completion_close();
    } else if state.open_dialog.is_some() {
        // Close dialog when open
        state.open_dialog = None;
        state.mark_dirty();
    } else {
        state.abort_queue();
    }
}

fn handle_editor_done(state: &mut AppState, content: String) {
    state.input.input = content;
    state.input.cursor_pos = state.input.input.len();
    state.mark_dirty();
}

// ── System actions (merged from system_actions.rs) ───────────────────────────

use crate::model::AppState as AppState3;

impl AppState {
    pub(crate) fn reload_all(&mut self) {
        let config = crate::config_reload::Config::load_from(&crate::config_reload::config_path());
        if let Some(provider) = &config.provider {
            self.config.config_provider = provider.clone();
        }
        if let Some(model) = config.default_model() {
            self.config.config_model = model.to_string();
        }
        if let Some(theme) = &config.theme {
            self.config.theme_name = theme.clone();
        }
        self.config.vim_mode = config.vim_mode();
        self.skills = crate::skills::load_all();
        let prompts_section = config.prompts();
        self.prompts = crate::prompts::load_prompts(
            prompts_section.default.as_deref(),
            prompts_section.custom.as_deref(),
        );
        self.add_system_msg(
            "Reloaded config, keybindings, theme, skills, and prompts.".to_string(),
        );
    }

    pub(crate) fn show_diagnostics(&mut self) {
        let mut lines = vec!["Diagnostics:".to_string()];
        let config_path = crate::config_reload::config_path();
        lines.push(format!(
            "  Config: {}",
            if config_path.exists() {
                config_path.display().to_string()
            } else {
                "not found".to_string()
            }
        ));
        let kb_path = crate::keybindings::default_keybindings_path();
        lines.push(format!(
            "  Keybindings: {}",
            if kb_path.as_ref().map(|p| p.exists()).unwrap_or(false) {
                kb_path.unwrap().display().to_string()
            } else {
                "default".to_string()
            }
        ));
        lines.push(format!("  Theme: {}", self.config.theme_name));
        lines.push(format!(
            "  Provider: {}/{}",
            self.config.current_provider, self.config.current_model
        ));
        lines.push(format!("  Read-only: {}", self.config.read_only));
        lines.push(format!(
            "  Scoped models: {}",
            self.config.scoped_models.len()
        ));
        self.add_system_msg(lines.join("\n"));
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn reload_all_reloads_skills() {
        let mut state = crate::model::AppState {
            skills: vec![crate::skills::Skill {
                name: "dummy".into(),
                description: "dummy".into(),
                context: "".into(),
                user_invocable: false,
                file_path: std::path::PathBuf::from("dummy.md"),
            }],
            ..Default::default()
        };
        state.reload_all();
        // In test environment load_all returns empty (no skill dirs exist)
        assert!(
            state.skills.is_empty(),
            "reload_all should reload skills from disk"
        );
        let last = state.session.messages.last().unwrap();
        assert!(
            last.content.contains("Reloaded"),
            "Should confirm reload: {}",
            last.content
        );
        // Prompts should also be reloaded (empty in test env)
        assert!(
            !state.prompts.is_empty(),
            "reload_all should reload prompts"
        );
        assert_eq!(state.prompts[0].name, "default");
    }
}
