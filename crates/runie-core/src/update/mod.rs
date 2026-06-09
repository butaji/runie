use crate::model::{AppState, ChatMessage, Role};
use crate::Event;

mod agent;
mod at_refs;
mod bash;
mod edit_approval;
mod input;
mod line_nav;
mod model_selector;
mod path_complete;
mod queue;
mod scoped_models;
pub mod settings_dialog;
mod system_actions;
mod palette;

pub(crate) fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

pub(crate) fn strip_tool_markers(content: &str) -> String {
    if let Some(pos) = content.find("TOOL:") {
        let before = &content[..pos];
        return before.trim_end().to_string();
    }
    let mut result = String::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('{') {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                if val.get("name").is_some() && val.get("arguments").is_some() {
                    continue;
                }
            }
        }
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(line);
    }
    result
}

pub(crate) fn content_has_tool_markers(content: &str) -> bool {
    if content.contains("TOOL:") {
        return true;
    }
    content.lines().any(|line| {
        let trimmed = line.trim();
        if !trimmed.starts_with('{') {
            return false;
        }
        serde_json::from_str::<serde_json::Value>(trimmed)
            .map(|v| v.get("name").is_some() && v.get("arguments").is_some())
            .unwrap_or(false)
    })
}

impl AppState {
    pub fn update(&mut self, event: Event) {
        if self.open_dialog.is_some() {
            self.update_dialog(event);
            return;
        }
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
            Event::HistoryPrev => {
                if self.path_suggestions.is_some() {
                    self.path_completion_up();
                } else if self.input.contains('\n') {
                    self.move_cursor_up();
                } else {
                    self.history_prev();
                }
            }
            Event::HistoryNext => {
                if self.path_suggestions.is_some() {
                    self.path_completion_down();
                } else if self.input.contains('\n') {
                    self.move_cursor_down();
                } else {
                    self.history_next();
                }
            }
            Event::Undo => self.undo(),
            Event::Redo => self.redo(),
            Event::CursorWordLeft => self.cursor_word_left(),
            Event::CursorWordRight => self.cursor_word_right(),
            Event::Paste(text) => self.paste(&text),
            Event::Submit => self.submit(),
            Event::ScrollUp => {
                if self.messages.is_empty() && !self.turn_active {
                    self.input_flash = 3;
                }
                self.scroll = self.scroll.saturating_add(1);
            }
            Event::ScrollDown => {
                if self.scroll == 0 {
                    self.input_flash = 3;
                }
                self.scroll = self.scroll.saturating_sub(1);
            }
            Event::Quit => self.should_quit = true,
            Event::Reset => *self = AppState::default(),
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
            Event::AgentToolEnd { duration_secs, output } => {
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
            Event::OpenExternalEditor => {}
            Event::ExternalEditorDone { content } => {
                self.input = content;
                self.cursor_pos = self.input.len();
                self.mark_dirty();
            }
            Event::Abort => {
                if self.path_suggestions.is_some() {
                    self.path_completion_close();
                } else {
                    self.abort_queue();
                }
            }
            Event::SpawnAgent => {}
            Event::ToggleExpand => self.toggle_expand_all(),
            Event::ToggleCommandPalette => {
                self.open_dialog = Some(crate::commands::DialogState::CommandPalette {
                    filter: String::new(),
                    selected: 0,
                });
                self.mark_dirty();
            }
            Event::ToggleModelSelector => {
                if matches!(self.open_dialog, Some(crate::commands::DialogState::ModelSelector { .. })) {
                    self.open_dialog = None;
                } else {
                    self.open_dialog = Some(crate::commands::DialogState::ModelSelector {
                        filter: String::new(),
                        selected: 0,
                    });
                }
                self.mark_dirty();
            }
            Event::ToggleScopedModelsDialog => {
                if matches!(self.open_dialog, Some(crate::commands::DialogState::ScopedModels { .. })) {
                    self.open_dialog = None;
                } else {
                    self.open_dialog = Some(crate::commands::DialogState::ScopedModels { selected: 0 });
                }
                self.mark_dirty();
            }
            Event::ScopedModelToggle { name } => scoped_models::toggle_scoped_model(self, &name),
            Event::ScopedModelEnableAll => scoped_models::enable_all(self),
            Event::ScopedModelDisableAll => scoped_models::disable_all(self),
            Event::ScopedModelToggleProvider { provider } => scoped_models::toggle_provider(self, &provider),
            Event::ToggleSettingsDialog => {
                if matches!(self.open_dialog, Some(crate::commands::DialogState::Settings { .. })) {
                    self.open_dialog = None;
                } else {
                    self.open_dialog = Some(crate::commands::DialogState::Settings {
                        category: crate::settings::SettingsCategory::Models,
                        selected: 0,
                    });
                }
                self.mark_dirty();
            }
            Event::SettingsUp |
            Event::SettingsDown |
            Event::SettingsLeft |
            Event::SettingsRight |
            Event::SettingsSelect |
            Event::SettingsClose => {},
            Event::PaletteFilter(_) |
            Event::PaletteBackspace |
            Event::PaletteUp |
            Event::PaletteDown |
            Event::PaletteSelect |
            Event::PaletteClose => {}
            Event::ModelSelectorFilter(_) |
            Event::ModelSelectorBackspace |
            Event::ModelSelectorUp |
            Event::ModelSelectorDown |
            Event::ModelSelectorSelect |
            Event::ModelSelectorClose => {}
            Event::PendingEdit { path, original, proposed, diff } => {
                self.pending_edits.push(crate::edit_preview::EditPreview::new(
                    std::path::PathBuf::from(path),
                    original,
                    proposed,
                    diff,
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
            Event::Suspend => {} // Handled in main event loop
        }
    }

    fn update_dialog(&mut self, event: Event) {
        let Some(dialog) = self.open_dialog.take() else { return };
        match dialog {
            crate::commands::DialogState::CommandPalette { filter, selected } => {
                self.update_palette(event, filter, selected);
            }
            crate::commands::DialogState::ScopedModels { selected } => {
                scoped_models::update_scoped_models(self, event, selected);
            }
            crate::commands::DialogState::Settings { category, selected } => {
                settings_dialog::update_settings_dialog(self, event, category, selected);
            }
            crate::commands::DialogState::ModelSelector { filter, selected } => {
                self.update_model_selector(event, filter, selected);
            }
        }
    }

    fn process_command_result(&mut self, result: crate::commands::CommandResult) {
        match result {
            crate::commands::CommandResult::Message(msg) => self.add_system_msg(msg),
            crate::commands::CommandResult::Event(evt) => self.update(evt),
            crate::commands::CommandResult::OpenDialog(d) => {
                self.open_dialog = Some(match d {
                    crate::commands::Dialog::CommandPalette => {
                        crate::commands::DialogState::CommandPalette {
                            filter: String::new(),
                            selected: 0,
                        }
                    }
                    crate::commands::Dialog::ModelSelector => {
                        crate::commands::DialogState::ModelSelector {
                            filter: String::new(),
                            selected: 0,
                        }
                    }
                    crate::commands::Dialog::Settings => crate::commands::DialogState::Settings {
                        category: crate::settings::SettingsCategory::Models,
                        selected: 0,
                    },
                    crate::commands::Dialog::ScopedModels => {
                        crate::commands::DialogState::ScopedModels { selected: 0 }
                    }
                });
                self.mark_dirty();
            }
            crate::commands::CommandResult::None => {}
        }
    }

    fn toggle_expand_all(&mut self) {
        self.all_collapsed = !self.all_collapsed;
        self.messages_changed();
    }

    fn switch_model(&mut self, provider: String, model: String) {
        self.current_provider = provider.clone();
        self.current_model = model.clone();
        self.record_model_usage(&provider, &model);
        self.telemetry.track_event(
            "model_switch",
            {
                let mut m = std::collections::HashMap::new();
                m.insert("provider".into(), provider.clone());
                m.insert("model".into(), model.clone());
                m
            },
        );
        self.add_system_msg(format!("Switched to {}/{}", provider, model));
    }

    fn switch_theme(&mut self, name: String) {
        self.theme_name = name.clone();
        self.add_system_msg(format!("Theme switched to '{}'", name));
    }

    fn cycle_model(&mut self, delta: isize) {
        let enabled: Vec<usize> = self
            .scoped_models
            .iter()
            .enumerate()
            .filter(|(_, m)| m.enabled)
            .map(|(i, _)| i)
            .collect();
        if enabled.is_empty() { return; }
        let current_pos = enabled.iter().position(|&i| i == self.scoped_index).unwrap_or(0);
        let len = enabled.len() as isize;
        let new_pos = ((current_pos as isize + delta).rem_euclid(len)) as usize;
        self.scoped_index = enabled[new_pos];
        let model = &self.scoped_models[self.scoped_index];
        self.switch_model(model.provider.clone(), model.name.clone());
    }

    fn cycle_thinking_level(&mut self) {
        self.thinking_level = self.thinking_level.cycle();
        self.add_system_msg(format!("Thinking level: {}", self.thinking_level.as_str()));
    }

    fn set_thinking_level(&mut self, level: crate::model::ThinkingLevel) {
        self.thinking_level = level;
        self.add_system_msg(format!("Thinking level set to: {}", self.thinking_level.as_str()));
    }

    fn toggle_read_only(&mut self) {
        self.read_only = !self.read_only;
        let status = if self.read_only { "enabled" } else { "disabled" };
        self.add_system_msg(format!("Read-only mode {}", status));
    }

    fn trust_project(&mut self) {
        let cwd = std::env::current_dir().unwrap_or_default();
        let mut tm = crate::trust::TrustManager::load();
        tm.set(&cwd, crate::trust::TrustDecision::Trusted);
        let _ = tm.save();
        self.read_only = false;
        self.add_system_msg(format!("Project '{}' trusted. Read-only disabled.", cwd.display()));
    }

    fn untrust_project(&mut self) {
        let cwd = std::env::current_dir().unwrap_or_default();
        let mut tm = crate::trust::TrustManager::load();
        tm.set(&cwd, crate::trust::TrustDecision::Untrusted);
        let _ = tm.save();
        self.read_only = true;
        self.add_system_msg(format!("Project '{}' untrusted. Read-only enabled.", cwd.display()));
    }

    pub fn peek_queue(&self) -> Option<&(String, String)> {
        self.request_queue.front()
    }

    pub fn pop_queue(&mut self) -> Option<(String, String)> {
        self.request_queue.pop_front()
    }

    pub(crate) fn add_system_msg(&mut self, content: String) {
        self.messages.push(ChatMessage {
            role: Role::System,
            content,
            timestamp: now(),
            id: "system".to_string(),
            ..Default::default()
        });
        self.messages_changed();
    }

    /// Move TurnComplete to the end of messages and bump its timestamp.
    /// Called after every agent event to ensure TurnComplete remains last.
    fn ensure_turn_complete_last(&mut self) {
        if let Some(idx) = self.messages.iter().position(|m| m.role == Role::TurnComplete) {
            let mut tc = self.messages.remove(idx);
            tc.timestamp = now();
            self.messages.push(tc);
            self.messages_changed();
        }
    }
}
