use super::{content_has_tool_markers, now, strip_tool_markers};
use crate::event::AgentEvent;
use crate::labels::{thought_with_time, tool_done, tool_running};
use crate::model::{AppState, ChatMessage, Role};
use crate::update::dialog::dialog_toggle_event;

pub fn agent_event(state: &mut AppState, event: AgentEvent) {
    match event {
        AgentEvent::Thinking { id } => {
            state.set_thinking(id);
            state.ensure_turn_complete_last();
        }
        AgentEvent::ThoughtDone { id } => {
            state.add_thought(id);
            state.ensure_turn_complete_last();
        }
        AgentEvent::ToolStart { id, name, .. } => {
            state.start_tool(id, name);
            state.ensure_turn_complete_last();
        }
        AgentEvent::ToolEnd {
            duration_secs,
            output,
            ..
        } => {
            state.end_tool(duration_secs, output);
            state.ensure_turn_complete_last();
        }
        // Transient streaming delta — update buffer, don't persist
        AgentEvent::ResponseDelta { id, content } => {
            state.append_response_delta(id, content);
        }
        // Complete response — append to message list
        AgentEvent::Response { id, content } => {
            state.append_response(id, content);
            state.ensure_turn_complete_last();
        }
        AgentEvent::TurnComplete { id, duration_secs } => {
            state.complete_turn(id, duration_secs);
            state.ensure_turn_complete_last();
        }
        AgentEvent::Done { id } => state.finish_turn(id),
        AgentEvent::Error { id, message } => {
            state.add_error(id, message);
            state.ensure_turn_complete_last();
        }
    }
}

impl AppState {
    pub(crate) fn set_thinking(&mut self, id: String) {
        self.agent.streaming = true;
        self.agent.current_request_id = Some(id);
        self.agent.thinking_started_at = Some(std::time::Instant::now());
        self.agent.turn_active = true;
        self.agent.current_action = Some("Thinking".to_string());
        self.agent
            .turn_started_at
            .get_or_insert_with(std::time::Instant::now);
        // Reset streaming buffer for new turn
        self.agent.streaming_buffer.reset();
        // Init speed tracking for this turn
        self.agent.turn_tokens_out = 0;
        self.agent.last_speed_update = Some(std::time::Instant::now());
        self.agent.tokens_at_last_speed = self.agent.tokens_out;
        self.agent.speed_tps = 0.0;
        // Keep existing rolling window - it auto-evicts to 1000 tokens
        self.agent.speed_window.record(self.agent.tokens_out);
        self.messages_changed();
    }

    pub(crate) fn add_thought(&mut self, id: String) {
        let duration = self.thinking_elapsed_secs().unwrap_or(0.0);
        self.agent.current_action = None;
        self.agent.thinking_started_at = None;
        let mut insert_idx = self.session.messages.len();
        let thought_content = if let Some(idx) = self
            .session
            .messages
            .iter()
            .position(|m| m.role == Role::Assistant && m.id == id)
        {
            let assistant = &self.session.messages[idx];
            let has_tools = content_has_tool_markers(&assistant.content);
            let stripped = strip_tool_markers(&assistant.content);
            if has_tools && !stripped.trim().is_empty() {
                self.session.messages.remove(idx);
                insert_idx = idx;
                format!("{}\n{}", thought_with_time(duration), stripped)
            } else {
                insert_idx = idx;
                thought_with_time(duration)
            }
        } else {
            thought_with_time(duration)
        };
        let thought_id = format!("{}#thought.{}", id, self.agent.thought_seq);
        self.agent.thought_seq += 1;
        let thought = ChatMessage {
            role: Role::Thought,
            content: thought_content,
            timestamp: now(),
            id: thought_id,
            ..Default::default()
        };
        self.session.messages.insert(insert_idx, thought);
        self.messages_changed();
    }

    pub(crate) fn start_tool(&mut self, id: String, name: String) {
        self.agent.current_request_id = Some(id.clone());
        self.agent.current_tool_name = Some(name.clone());
        self.agent.tool_started_at = Some(std::time::Instant::now());
        self.agent.intermediate_step_count += 1;
        self.agent.current_action = Some(format!("Running {}", name));
        let tool_id = format!("tool.{}.{}", id, self.agent.intermediate_step_count);
        self.session.messages.push(ChatMessage {
            role: Role::Tool,
            content: tool_running(&name),
            timestamp: now(),
            id: tool_id,
            ..Default::default()
        });
        self.config.telemetry.track_event("tool_usage", {
            let mut m = std::collections::HashMap::new();
            m.insert("tool".into(), name);
            m
        });
        self.messages_changed();
    }

    pub(crate) fn end_tool(&mut self, duration_secs: f64, output: String) {
        self.agent.current_action = None;
        self.agent.tool_started_at = None;
        if let Some(name) = self.agent.current_tool_name.take() {
            if let Some(idx) = self
                .session
                .messages
                .iter()
                .rposition(|m| m.role == Role::Tool && m.content.contains("⠋ Running "))
            {
                if let Some(last) = self.session.messages.get_mut(idx) {
                    last.content = if output.trim().is_empty() {
                        tool_done(&name, duration_secs)
                    } else {
                        format!("{}\n{}", tool_done(&name, duration_secs), output)
                    };
                    last.timestamp = now();
                }
            }
        }
        self.messages_changed();
    }

    pub(crate) fn append_response(&mut self, id: String, content: String) {
        self.track_response_tokens(&content);
        if let Some(idx) = self.find_cached_assistant_index(&id) {
            self.append_to_message(idx, &content);
            return;
        }
        if let Some(idx) = self.find_assistant_by_id(&id) {
            self.agent.last_assistant_index = Some(idx);
            self.append_to_message(idx, &content);
            return;
        }
        self.create_assistant_message(id, content);
    }

    pub(crate) fn append_response_delta(&mut self, id: String, content: String) {
        self.track_response_tokens(&content);
        self.agent.streaming_buffer.push_delta(&content);
        // Find existing assistant message and append stable content, or create new one
        let stable_lines = self.agent.streaming_buffer.flush();
        if !stable_lines.is_empty() {
            let stable = stable_lines.join("");
            if let Some(idx) = self.find_cached_assistant_index(&id) {
                self.append_to_message(idx, &stable);
            } else if let Some(idx) = self.find_assistant_by_id(&id) {
                self.agent.last_assistant_index = Some(idx);
                self.append_to_message(idx, &stable);
            } else {
                self.create_assistant_message(id.clone(), stable);
            }
        }
    }

    fn track_response_tokens(&mut self, content: &str) {
        if content.is_empty() {
            return;
        }
        let n = self.agent.token_tracker.estimate_output(content);
        self.agent.tokens_out += n;
        self.agent.turn_tokens_out += n;
    }

    fn find_cached_assistant_index(&self, id: &str) -> Option<usize> {
        let idx = self.agent.last_assistant_index?;
        let msg = self.session.messages.get(idx)?;
        if msg.role == Role::Assistant && msg.id == id {
            Some(idx)
        } else {
            None
        }
    }

    fn find_assistant_by_id(&self, id: &str) -> Option<usize> {
        self.session
            .messages
            .iter()
            .position(|m| m.role == Role::Assistant && m.id == id)
    }

    fn append_to_message(&mut self, idx: usize, content: &str) {
        if let Some(msg) = self.session.messages.get_mut(idx) {
            if !content.is_empty() {
                msg.content.push_str(content);
            }
            msg.timestamp = now();
            self.messages_changed();
        }
    }

    fn create_assistant_message(&mut self, id: String, content: String) {
        if content.is_empty() {
            return;
        }
        let idx = self.session.messages.len();
        self.session.messages.push(ChatMessage {
            role: Role::Assistant,
            content,
            timestamp: now(),
            id: id.clone(),
            provider: self.config.current_provider.clone(),
            ..Default::default()
        });
        self.agent.current_request_id = Some(id);
        self.agent.last_assistant_index = Some(idx);
        self.messages_changed();
    }

    pub(crate) fn complete_turn(&mut self, id: String, duration_secs: f64) {
        let content = format!("Turn completed in {:.1}s", duration_secs);
        let ts = now();
        if let Some(idx) = self
            .session
            .messages
            .iter()
            .position(|m| m.role == Role::TurnComplete && m.id == id)
        {
            self.session.messages[idx].content = content;
            self.session.messages[idx].timestamp = ts;
        } else {
            self.session.messages.push(ChatMessage {
                role: Role::TurnComplete,
                content,
                timestamp: ts,
                id,
                ..Default::default()
            });
        }
        self.messages_changed();
        self.agent.turn_started_at = None;
    }

    pub(crate) fn finish_turn(&mut self, id: String) {
        // Flush any remaining tail content from streaming buffer
        let remaining_tail = self.agent.streaming_buffer.force_flush().join("");
        if !remaining_tail.is_empty() {
            if let Some(idx) = self.agent.last_assistant_index {
                self.append_to_message(idx, &remaining_tail);
            }
        }
        self.agent.streaming_buffer.reset();
        self.strip_tools_from_assistant();
        self.remove_empty_assistant();
        self.clear_turn_state(&id);
        self.deliver_queued();
        self.maybe_end_streaming();
        self.reorder_agent_after_tools();
        self.move_turn_complete_to_end(&id);
        self.messages_changed();
    }

    fn strip_tools_from_assistant(&mut self) {
        for msg in self.session.messages.iter_mut() {
            if msg.role == Role::Assistant {
                msg.content = strip_tool_markers(&msg.content);
            }
        }
    }

    fn remove_empty_assistant(&mut self) {
        self.session
            .messages
            .retain(|msg| !(msg.role == Role::Assistant && msg.content.trim().is_empty()));
    }

    fn clear_turn_state(&mut self, id: &str) {
        if self.agent.current_request_id.as_deref() == Some(id) {
            self.agent.current_request_id = None;
        }
        self.agent.current_tool_name = None;
        self.agent.current_action = None;
        self.agent.intermediate_step_count = 0;
        self.agent.thought_seq = 0;
        self.agent.turn_active = false;
        self.agent.turn_started_at = None;
        self.view.vim_nav_pending = false;
        self.agent.inflight = self.agent.inflight.saturating_sub(1);
        // Reset per-turn speed tracking (but keep speed_window for continuity)
        self.agent.turn_tokens_out = 0;
        self.agent.speed_tps = 0.0;
        self.agent.last_speed_update = None;
    }

    fn maybe_end_streaming(&mut self) {
        if self.agent.inflight == 0 && self.agent.request_queue.is_empty() {
            self.agent.streaming = false;
            if self.agent.current_request_id.is_none() {
                self.agent.thinking_started_at = None;
            }
        }
    }

    fn reorder_agent_after_tools(&mut self) {
        let last_assistant = self
            .session
            .messages
            .iter()
            .rposition(|m| m.role == Role::Assistant);
        let last_tool = self
            .session
            .messages
            .iter()
            .rposition(|m| m.role == Role::Tool);
        if let (Some(a_idx), Some(t_idx)) = (last_assistant, last_tool) {
            if a_idx < t_idx {
                let mut agent = self.session.messages.remove(a_idx);
                agent.timestamp = now();
                self.session.messages.insert(t_idx, agent);
                // Update cached index if it was affected
                if self.agent.last_assistant_index == Some(a_idx) {
                    self.agent.last_assistant_index = Some(t_idx);
                }
            }
        }
    }

    fn move_turn_complete_to_end(&mut self, id: &str) {
        if let Some(idx) = self
            .session
            .messages
            .iter()
            .position(|m| m.role == Role::TurnComplete && m.id == id)
        {
            let mut turn_complete = self.session.messages.remove(idx);
            turn_complete.timestamp = now();
            self.session.messages.push(turn_complete);
            // Update cached index if it was affected
            if let Some(last_idx) = self.agent.last_assistant_index {
                if last_idx >= idx {
                    self.agent.last_assistant_index = Some(last_idx.saturating_sub(1));
                }
            }
        }
    }

    pub(crate) fn add_error(&mut self, id: String, message: String) {
        self.agent.streaming = false;
        let mut error = ChatMessage {
            role: Role::Assistant,
            content: format!("Error: {}", message),
            timestamp: now(),
            id: format!("error.{}", id),
            provider: self.config.current_provider.clone(),
            ..Default::default()
        };
        if let Some(idx) = self
            .session
            .messages
            .iter()
            .position(|m| m.role == Role::TurnComplete)
        {
            error.timestamp = self.session.messages[idx].timestamp;
            self.session.messages.insert(idx, error);
        } else {
            self.session.messages.push(error);
        }
        self.messages_changed();
    }
}

// ── @-ref handling (merged from at_refs.rs) ───────────────────────────────────

impl AppState {
    /// Legacy @-trigger popup disabled — file picker now uses PanelStack dialog.
    /// Clears stale state and ghost completions on any text change.
    pub(crate) fn handle_at_trigger(&mut self) {
        self.clear_ghost();
        self.completion.at_suggestions = None;
        self.completion.at_selected = None;
        self.completion.last_at_query = None;
    }

    /// Insert the currently selected @ suggestion into the input.
    /// Wraps the path in [...] format.
    pub(crate) fn insert_at_suggestion(&mut self) {
        let suggestions = match &self.completion.at_suggestions {
            Some(s) if !s.is_empty() => s,
            _ => {
                // No suggestions, just clear state
                self.completion.at_suggestions = None;
                self.completion.at_selected = None;
                return;
            }
        };

        let selected_idx = self.completion.at_selected.unwrap_or(0);
        if let Some(selected) = suggestions.get(selected_idx) {
            // Insert the selected suggestion wrapped in [...]
            self.input.input.push_str(&format!("[{}]", selected));
            self.input.cursor_pos = self.input.input.len();
        }

        // Clear completion state
        self.completion.at_suggestions = None;
        self.completion.at_selected = None;
        self.mark_dirty();
    }
}

// ── Model config events (merged from model_config.rs) ─────────────────────────

use crate::event::ModelConfigEvent;
use crate::Event;

pub fn model_config_event(state: &mut AppState, event: ModelConfigEvent) {
    let invalidate = handle_main_events(state, &event)
        || handle_scoped_events(state, &event)
        || handle_settings_events(state, &event);
    if invalidate {
        state.view.cached_settings_valid = false;
    }
}

fn handle_main_events(state: &mut AppState, event: &ModelConfigEvent) -> bool {
    match event {
        ModelConfigEvent::SwitchModel { provider, model } => {
            state.switch_model(provider.clone(), model.clone());
            true
        }
        ModelConfigEvent::SwitchTheme { name } => {
            state.switch_theme(name.clone());
            true
        }
        ModelConfigEvent::CycleModelNext => {
            state.cycle_model(1);
            false
        }
        ModelConfigEvent::CycleModelPrev => {
            state.cycle_model(-1);
            false
        }
        ModelConfigEvent::CycleThinkingLevel => {
            state.cycle_thinking_level();
            true
        }
        ModelConfigEvent::SetThinkingLevel(level) => {
            state.set_thinking_level(*level);
            true
        }
        ModelConfigEvent::ToggleReadOnly => {
            state.toggle_read_only();
            true
        }
        _ => false,
    }
}

fn handle_scoped_events(state: &mut AppState, event: &ModelConfigEvent) -> bool {
    match event {
        ModelConfigEvent::TrustProject => {
            state.trust_project();
            false
        }
        ModelConfigEvent::UntrustProject => {
            state.untrust_project();
            false
        }
        ModelConfigEvent::ReloadAll => {
            state.reload_all();
            false
        }
        ModelConfigEvent::ScopedModelToggle { name } => {
            toggle_scoped_model(state, name);
            false
        }
        ModelConfigEvent::ScopedModelEnableAll => {
            enable_all(state);
            false
        }
        ModelConfigEvent::ScopedModelDisableAll => {
            disable_all(state);
            false
        }
        ModelConfigEvent::ScopedModelToggleProvider { provider } => {
            toggle_provider(state, provider);
            false
        }
        _ => false,
    }
}

/// Handle settings dialog navigation and selection events.
/// When a dialog is open, delegate to update_dialog for proper panel stack handling.
fn handle_settings_events(state: &mut AppState, event: &ModelConfigEvent) -> bool {
    match event {
        ModelConfigEvent::ToggleSettingsDialog => {
            dialog_toggle_event(
                state,
                crate::event::DialogEvent::ToggleSettingsDialog,
            );
            true
        }
        ModelConfigEvent::ToggleScopedModelsDialog => {
            dialog_toggle_event(
                state,
                crate::event::DialogEvent::ToggleScopedModelsDialog,
            );
            true
        }
        ModelConfigEvent::SettingsClose => {
            crate::update::dialog::update_dialog(state, Event::ModelConfig(event.clone()));
            true
        }
        ModelConfigEvent::SettingsSelect
        | ModelConfigEvent::SettingsDown
        | ModelConfigEvent::SettingsUp
        | ModelConfigEvent::SettingsLeft
        | ModelConfigEvent::SettingsRight => {
            if state.open_dialog.is_some() {
                crate::update::dialog::update_dialog(state, Event::ModelConfig(event.clone()));
            }
            true
        }
        _ => false,
    }
}

// ── Scoped models (merged from scoped_models.rs) ─────────────────────────────


pub fn toggle_scoped_model(state: &mut AppState, name: &str) {
    if let Some(idx) = state
        .config
        .scoped_models
        .iter()
        .position(|m| m.name == name)
    {
        state.config.scoped_models[idx].enabled = !state.config.scoped_models[idx].enabled;
        state.mark_dirty();
    }
}

pub fn enable_all(state: &mut AppState) {
    for m in &mut state.config.scoped_models {
        m.enabled = true;
    }
    state.mark_dirty();
}

pub fn disable_all(state: &mut AppState) {
    for m in &mut state.config.scoped_models {
        m.enabled = false;
    }
    state.mark_dirty();
}

pub fn toggle_provider(state: &mut AppState, provider: &str) {
    let all_enabled = state
        .config
        .scoped_models
        .iter()
        .filter(|m| m.provider == provider)
        .all(|m| m.enabled);
    for m in &mut state.config.scoped_models {
        if m.provider == provider {
            m.enabled = !all_enabled;
        }
    }
    state.mark_dirty();
}
