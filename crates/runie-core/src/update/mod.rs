use crate::model::{AppState, ChatMessage, Role};
use crate::Event;

// Re-export for backward compatibility
pub use crate::tool_markers::has_tool_markers as content_has_tool_markers;
pub use crate::tool_markers::strip_tool_markers;

/// What a form panel should do in response to an event.
#[derive(Debug, Clone)]
pub enum FormAction {
    /// Keep the form open, persist the panel state.
    KeepOpen,
    /// Close the form (no submit).
    Close,
    /// Close the form and dispatch the submit event.
    Submit(Option<crate::Event>),
    /// Go back one step: if the stack is deeper than the root, pop the
    /// current panel and keep the dialog open; if at the root, close
    /// the dialog. This is the semantic of ESC / back.
    Back,
}

mod agent;
mod at_refs;
mod bash;
mod dialog;
mod dialog_actions;
mod dialog_open;
mod dialog_update;
mod edit;
mod edit_approval;
mod input;
mod input_scroll;
mod input_text;
mod line_nav;
mod login_flow;
mod path_complete;
mod queue;
pub mod scoped_models;
mod session;
pub mod settings_dialog;
mod system_actions;
pub mod tab_complete;

pub(crate) fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

impl AppState {
    /// Main event dispatcher - delegates to specialized handlers based on event type.
    pub fn update(&mut self, event: Event) {
        use crate::event::EventCategory;
        if matches!(event.category(), EventCategory::Transient) {
            return self.transient_event(event);
        }
        if event.is_login() {
            return self.login_flow_event(event);
        }
        if self.open_dialog.is_some() {
            return self.update_dialog(event);
        }
        match event.category() {
            EventCategory::Input => self.input_event(event),
            EventCategory::Agent => self.agent_event(event),
            EventCategory::Scroll => self.scroll_event(event),
            EventCategory::Control => self.control_event(event),
            EventCategory::ModelConfig => self.model_config_event(event),
            EventCategory::DialogToggle => self.dialog_toggle_event(event),
            EventCategory::Settings => self.settings_event(event),
            EventCategory::Edit => self.edit_event(event),
            EventCategory::System => self.system_event(event),
            EventCategory::Transient => unreachable!(),
        }
    }

    fn transient_event(&mut self, event: Event) {
        match event {
            Event::TransientMessage { content, level } => self.set_transient(content, level),
            Event::TransientError { content } => {
                self.set_transient(content, crate::event::TransientLevel::Error)
            }
            Event::ClearTransient => self.clear_transient(),
            _ => {}
        }
    }

    fn system_event(&mut self, event: Event) {
        if let Event::SystemMessage { content } = event {
            self.add_system_msg(content);
        }
    }


    fn scroll_event(&mut self, event: Event) {
        let page_size = 5usize;
        match event {
            Event::ScrollUp => {
                if self.session.messages.is_empty() && !self.agent.turn_active {
                    self.input.input_flash = 3;
                }
                self.view.scroll = self.view.scroll.saturating_add(1);
            }
            Event::ScrollDown => {
                if self.view.scroll == 0 {
                    self.input.input_flash = 3;
                }
                self.view.scroll = self.view.scroll.saturating_sub(1);
            }
            Event::PageUp => {
                if self.session.messages.is_empty() && !self.agent.turn_active {
                    self.input.input_flash = 3;
                }
                self.view.scroll = self.view.scroll.saturating_add(page_size);
            }
            Event::PageDown => {
                if self.view.scroll == 0 {
                    self.input.input_flash = 3;
                }
                self.view.scroll = self.view.scroll.saturating_sub(page_size);
            }
            _ => {}
        }
    }

    // === Control Event Handler ===
    fn control_event(&mut self, event: Event) {
        match event {
            Event::Quit => {
                if !self.input.input.is_empty() {
                    self.input.input.clear();
                    self.input.cursor_pos = 0;
                    self.input.input_scroll = 0;
                    self.input.undo_stack.clear();
                    self.input.redo_stack.clear();
                    self.mark_dirty();
                } else {
                    self.should_quit = true;
                }
            }
            Event::Reset => *self = AppState::default(),
            Event::Abort => {
                if self.completion.path_suggestions.is_some() {
                    self.path_completion_close();
                } else {
                    self.abort_queue();
                }
            }
            Event::SpawnAgent { .. } | Event::Suspend | Event::ShareSession | Event::OpenExternalEditor => {}
            Event::ExternalEditorDone { content } => {
                self.input.input = content;
                self.input.cursor_pos = self.input.input.len();
                self.mark_dirty();
            }
            Event::ToggleExpand => self.toggle_expand_all(),
            Event::ForkSession { message_index } => self.fork_session_at(message_index),
            Event::CloneSession => self.clone_session(),
            Event::ToggleSessionTree => self.toggle_session_tree_dialog(),
            Event::SessionTreeFilterCycle => self.cycle_session_tree_filter(),
            Event::SessionTreeSelect { id } => self.session_tree_select(&id),
            Event::AtFilePicker => self.open_at_file_picker(),
            Event::InsertAtRef(path) => self.insert_at_ref(&path),
            _ => {}
        }
    }

    // === Input Event Handler ===
    fn input_event(&mut self, event: Event) {
        match event {
            Event::Input(c) => self.push_input(c),
            Event::Backspace => self.pop_input(),
            Event::Newline => self.insert_newline(),
            Event::CursorLeft => self.cursor_left(),
            Event::CursorRight => self.cursor_right(),
            Event::CursorStart => self.cursor_start(),
            Event::CursorEnd => self.cursor_end(),
            Event::DeleteWord => self.delete_word(),
            Event::DeleteToEnd => self.delete_to_end(),
            Event::DeleteToStart => self.delete_to_start(),
            Event::KillChar => self.kill_char(),
            Event::Undo => self.undo(),
            Event::Redo => self.redo(),
            Event::CursorWordLeft => self.cursor_word_left(),
            Event::CursorWordRight => self.cursor_word_right(),
            Event::Paste(text) => self.paste(&text),
            Event::PasteImage => self.paste_image(),
            Event::Submit => self.submit(),
            Event::HistoryPrev => self.handle_history_prev(),
            Event::HistoryNext => self.handle_history_next(),
            Event::InsertAtRef(path) => self.insert_at_ref(&path),
            _ => {}
        }
    }

    fn handle_history_prev(&mut self) {
        if self.completion.path_suggestions.is_some() {
            self.path_completion_up();
        } else if self.input.input.contains('\n') {
            self.move_cursor_up();
        } else {
            self.history_prev();
        }
    }

    fn handle_history_next(&mut self) {
        if self.completion.path_suggestions.is_some() {
            self.path_completion_down();
        } else if self.input.input.contains('\n') {
            self.move_cursor_down();
        } else {
            self.history_next();
        }
    }

    // === Agent Event Handler ===
    fn agent_event(&mut self, event: Event) {
        match event {
            Event::AgentThinking { id } => {
                self.set_thinking(id);
                self.ensure_turn_complete_last();
            }
            Event::AgentThoughtDone { id } => {
                self.add_thought(id);
                self.ensure_turn_complete_last();
            }
            Event::AgentToolStart { id, name } => {
                self.start_tool(id, name);
                self.ensure_turn_complete_last();
            }
            Event::AgentToolEnd {
                duration_secs,
                output,
            } => {
                self.end_tool(duration_secs, output);
                self.ensure_turn_complete_last();
            }
            Event::AgentResponse { id, content } => {
                self.append_response(id, content);
                self.ensure_turn_complete_last();
            }
            Event::AgentTurnComplete { id, duration_secs } => {
                self.complete_turn(id, duration_secs);
                self.ensure_turn_complete_last();
            }
            Event::AgentDone { id } => self.finish_turn(id),
            Event::AgentError { id, message } => {
                self.add_error(id, message);
                self.ensure_turn_complete_last();
            }
            _ => {}
        }
    }

    // === Model & Config Event Handler ===
    fn model_config_event(&mut self, event: Event) {
        match event {
            Event::SwitchModel { provider, model } => self.switch_model(provider, model),
            Event::SwitchTheme { name } => self.switch_theme(name),
            Event::CycleModelNext => self.cycle_model(1),
            Event::CycleModelPrev => self.cycle_model(-1),
            Event::CycleThinkingLevel => self.cycle_thinking_level(),
            Event::SetThinkingLevel(level) => self.set_thinking_level(level),
            Event::ToggleReadOnly => self.toggle_read_only(),
            Event::TrustProject => self.trust_project(),
            Event::UntrustProject => self.untrust_project(),
            Event::FollowUp => self.queue_follow_up(),
            Event::Dequeue => self.dequeue(),
            Event::ToggleScopedModelsDialog => self.open_scoped_models_dialog(),
            Event::ScopedModelToggle { name } => scoped_models::toggle_scoped_model(self, &name),
            Event::ScopedModelEnableAll => scoped_models::enable_all(self),
            Event::ScopedModelDisableAll => scoped_models::disable_all(self),
            Event::ScopedModelToggleProvider { provider } => {
                scoped_models::toggle_provider(self, &provider)
            }
            _ => {}
        }
    }

    // === Dialog Toggle Event Handler ===



























    fn toggle_expand_all(&mut self) {
        self.all_collapsed = !self.all_collapsed;
        self.messages_changed();
    }

    fn switch_model(&mut self, provider: String, model: String) {
        self.config.current_provider = provider.clone();
        self.config.current_model = model.clone();
        self.record_model_usage(&provider, &model);
        self.telemetry.track_event("model_switch", {
            let mut m = std::collections::HashMap::new();
            m.insert("provider".into(), provider.clone());
            m.insert("model".into(), model.clone());
            m
        });
        self.notify(
            format!("Switched to {}/{}", provider, model),
            crate::event::TransientLevel::Success,
        );
    }

    fn switch_theme(&mut self, name: String) {
        self.config.theme_name = name.clone();
        self.notify(
            format!("Theme switched to '{}'", name),
            crate::event::TransientLevel::Success,
        );
    }

    fn cycle_model(&mut self, delta: isize) {
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

    fn cycle_thinking_level(&mut self) {
        self.config.thinking_level = self.config.thinking_level.cycle();
        self.notify(
            format!("Thinking level: {}", self.config.thinking_level.as_str()),
            crate::event::TransientLevel::Info,
        );
    }

    fn set_thinking_level(&mut self, level: crate::model::ThinkingLevel) {
        self.config.thinking_level = level;
        self.notify(
            format!(
                "Thinking level set to: {}",
                self.config.thinking_level.as_str()
            ),
            crate::event::TransientLevel::Info,
        );
    }

    fn toggle_read_only(&mut self) {
        self.config.read_only = !self.config.read_only;
        let status = if self.config.read_only {
            "enabled"
        } else {
            "disabled"
        };
        self.notify(
            format!("Read-only mode {}", status),
            crate::event::TransientLevel::Warning,
        );
    }

    fn trust_project(&mut self) {
        let cwd = std::env::current_dir().unwrap_or_default();
        let mut tm = crate::trust::TrustManager::load();
        tm.set(&cwd, crate::trust::TrustDecision::Trusted);
        let _ = tm.save();
        self.config.read_only = false;
        self.notify(
            format!("Project '{}' trusted. Read-only disabled.", cwd.display()),
            crate::event::TransientLevel::Success,
        );
    }

    fn untrust_project(&mut self) {
        let cwd = std::env::current_dir().unwrap_or_default();
        let mut tm = crate::trust::TrustManager::load();
        tm.set(&cwd, crate::trust::TrustDecision::Untrusted);
        let _ = tm.save();
        self.config.read_only = true;
        self.notify(
            format!("Project '{}' untrusted. Read-only enabled.", cwd.display()),
            crate::event::TransientLevel::Warning,
        );
    }

    pub fn peek_queue(&self) -> Option<&(String, String)> {
        self.agent.request_queue.front()
    }

    pub fn pop_queue(&mut self) -> Option<(String, String)> {
        self.agent.request_queue.pop_front()
    }

    fn set_transient(&mut self, content: String, level: crate::event::TransientLevel) {
        self.transient_message = Some(content);
        self.transient_level = Some(level);
        self.transient_until = match level {
            crate::event::TransientLevel::Error => None,
            _ => Some(std::time::Instant::now() + std::time::Duration::from_secs(5)),
        };
        self.mark_dirty();
    }

    fn clear_transient(&mut self) {
        self.transient_message = None;
        self.transient_until = None;
        self.transient_level = None;
        self.mark_dirty();
    }

    pub(crate) fn add_system_msg(&mut self, content: String) {
        self.session.messages.push(ChatMessage {
            role: Role::System,
            content,
            timestamp: now(),
            id: "system".to_string(),
            ..Default::default()
        });
        self.messages_changed();
    }

    /// Emit a transient notification in the hints line (not in the feed).
    pub(crate) fn notify(&mut self, content: String, level: crate::event::TransientLevel) {
        self.set_transient(content, level);
    }

    /// Move TurnComplete to the end of messages and bump its timestamp.
    /// Called after every agent event to ensure TurnComplete remains last.
    /// Only moves the TurnComplete for the current turn (matching current_request_id
    /// or falling back to the last assistant message's id), so earlier turns'
    /// TurnComplete are not affected.
    fn ensure_turn_complete_last(&mut self) {
        let target_id = self.agent.current_request_id.clone().or_else(|| {
            self.last_assistant_index
                .and_then(|idx| self.session.messages.get(idx).map(|m| m.id.clone()))
        });
        let Some(target_id) = target_id else { return };
        if let Some(idx) = self
            .session
            .messages
            .iter()
            .position(|m| m.role == Role::TurnComplete && m.id == target_id)
        {
            let mut tc = self.session.messages.remove(idx);
            tc.timestamp = now();
            self.session.messages.push(tc);
            self.messages_changed();
        }
    }

}
