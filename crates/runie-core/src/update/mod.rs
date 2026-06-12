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
mod control;
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
            return transient_event(self, event);
        }
        if event.is_login() {
            return login_flow::update(self, event);
        }
        if self.open_dialog.is_some() {
            return dialog_update::update(self, event);
        }
        match event.category() {
            EventCategory::Input => input_event(self, event),
            EventCategory::Agent => agent_event(self, event),
            EventCategory::Scroll => scroll_event(self, event),
            EventCategory::Control => control::update(self, event),
            EventCategory::ModelConfig => model_config_event(self, event),
            EventCategory::DialogToggle => dialog_open::update(self, event),
            EventCategory::Settings => settings_dialog::update(self, event),
            EventCategory::Edit => edit::update(self, event),
            EventCategory::System => system_event(self, event),
            EventCategory::Transient => unreachable!(),
        }
    }
}

fn transient_event(state: &mut AppState, event: Event) {
    match event {
        Event::TransientMessage { content, level } => state.set_transient(content, level),
        Event::TransientError { content } => {
            state.set_transient(content, crate::event::TransientLevel::Error)
        }
        Event::ClearTransient => state.clear_transient(),
        _ => {}
    }
}

fn system_event(state: &mut AppState, event: Event) {
    if let Event::SystemMessage { content } = event {
        state.add_system_msg(content);
    }
}

fn scroll_event(state: &mut AppState, event: Event) {
    let page_size = 5usize;
    match event {
        Event::ScrollUp => {
            if state.session.messages.is_empty() && !state.agent.turn_active {
                state.input.input_flash = 3;
            }
            state.view.scroll = state.view.scroll.saturating_add(1);
        }
        Event::ScrollDown => {
            if state.view.scroll == 0 {
                state.input.input_flash = 3;
            }
            state.view.scroll = state.view.scroll.saturating_sub(1);
        }
        Event::PageUp => {
            if state.session.messages.is_empty() && !state.agent.turn_active {
                state.input.input_flash = 3;
            }
            state.view.scroll = state.view.scroll.saturating_add(page_size);
        }
        Event::PageDown => {
            if state.view.scroll == 0 {
                state.input.input_flash = 3;
            }
            state.view.scroll = state.view.scroll.saturating_sub(page_size);
        }
        _ => {}
    }
}

// === Input Event Handler ===
fn input_event(state: &mut AppState, event: Event) {
    match event {
        Event::Input(c) => state.push_input(c),
        Event::Backspace => state.pop_input(),
        Event::Newline => state.insert_newline(),
        Event::CursorLeft => state.cursor_left(),
        Event::CursorRight => state.cursor_right(),
        Event::CursorStart => state.cursor_start(),
        Event::CursorEnd => state.cursor_end(),
        Event::DeleteWord => state.delete_word(),
        Event::DeleteToEnd => state.delete_to_end(),
        Event::DeleteToStart => state.delete_to_start(),
        Event::KillChar => state.kill_char(),
        Event::Undo => state.undo(),
        Event::Redo => state.redo(),
        Event::CursorWordLeft => state.cursor_word_left(),
        Event::CursorWordRight => state.cursor_word_right(),
        Event::Paste(text) => state.paste(&text),
        Event::PasteImage => state.paste_image(),
        Event::Submit => state.submit(),
        Event::HistoryPrev => handle_history_prev(state),
        Event::HistoryNext => handle_history_next(state),
        Event::InsertAtRef(path) => state.insert_at_ref(&path),
        _ => {}
    }
}

fn handle_history_prev(state: &mut AppState) {
    if state.completion.path_suggestions.is_some() {
        state.path_completion_up();
    } else if state.input.input.contains('\n') {
        state.move_cursor_up();
    } else {
        state.history_prev();
    }
}

fn handle_history_next(state: &mut AppState) {
    if state.completion.path_suggestions.is_some() {
        state.path_completion_down();
    } else if state.input.input.contains('\n') {
        state.move_cursor_down();
    } else {
        state.history_next();
    }
}

// === Agent Event Handler ===
fn agent_event(state: &mut AppState, event: Event) {
    match event {
        Event::AgentThinking { id } => {
            state.set_thinking(id);
            state.ensure_turn_complete_last();
        }
        Event::AgentThoughtDone { id } => {
            state.add_thought(id);
            state.ensure_turn_complete_last();
        }
        Event::AgentToolStart { id, name } => {
            state.start_tool(id, name);
            state.ensure_turn_complete_last();
        }
        Event::AgentToolEnd {
            duration_secs,
            output,
        } => {
            state.end_tool(duration_secs, output);
            state.ensure_turn_complete_last();
        }
        Event::AgentResponse { id, content } => {
            state.append_response(id, content);
            state.ensure_turn_complete_last();
        }
        Event::AgentTurnComplete { id, duration_secs } => {
            state.complete_turn(id, duration_secs);
            state.ensure_turn_complete_last();
        }
        Event::AgentDone { id } => state.finish_turn(id),
        Event::AgentError { id, message } => {
            state.add_error(id, message);
            state.ensure_turn_complete_last();
        }
        _ => {}
    }
}

// === Model & Config Event Handler ===
pub(super) fn model_config_event(state: &mut AppState, event: Event) {
    match event {
        Event::SwitchModel { provider, model } => state.switch_model(provider, model),
        Event::SwitchTheme { name } => state.switch_theme(name),
        Event::CycleModelNext => state.cycle_model(1),
        Event::CycleModelPrev => state.cycle_model(-1),
        Event::CycleThinkingLevel => state.cycle_thinking_level(),
        Event::SetThinkingLevel(level) => state.set_thinking_level(level),
        Event::ToggleReadOnly => state.toggle_read_only(),
        Event::TrustProject => state.trust_project(),
        Event::UntrustProject => state.untrust_project(),
        Event::FollowUp => state.queue_follow_up(),
        Event::Dequeue => state.dequeue(),
        Event::ToggleScopedModelsDialog => state.open_scoped_models_dialog(),
        Event::ScopedModelToggle { name } => scoped_models::toggle_scoped_model(state, &name),
        Event::ScopedModelEnableAll => scoped_models::enable_all(state),
        Event::ScopedModelDisableAll => scoped_models::disable_all(state),
        Event::ScopedModelToggleProvider { provider } => {
            scoped_models::toggle_provider(state, &provider)
        }
        _ => {}
    }
}

impl AppState {
    pub(super) fn toggle_expand_all(&mut self) {
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
