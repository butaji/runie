use crate::model::{AppState, ChatMessage, Role};
use crate::Event;

// Re-export for backward compatibility
pub use crate::tool_markers::has_tool_markers as content_has_tool_markers;
pub use crate::tool_markers::strip_tool_markers;

mod agent;
mod at_refs;
mod bash;
mod control;
pub(crate) mod dialog_stack;
mod dialog_toggle;
mod edit_approval;
mod form;
pub use form::FormAction;
mod input;
mod input_scroll;
mod input_text;
mod line_nav;
mod login_flow;
mod model_config;
mod model_selector;
mod path_complete;
mod queue;
pub mod scoped_models;
mod scroll;
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
        // Login flow events bypass dialog routing — they manage their own dialog state.
        if matches!(
            event,
            Event::LoginFlowStart
                | Event::LoginFlowSelectProvider { .. }
                | Event::LoginFlowSubmitKey { .. }
                | Event::LoginFlowValidate { .. }
                | Event::LoginFlowValidationDone { .. }
                | Event::LoginFlowValidationFailed { .. }
                | Event::LoginFlowModelsFetched { .. }
                | Event::LoginFlowToggleModel { .. }
                | Event::LoginFlowSave
                | Event::LoginFlowCancel
        ) {
            login_flow::login_flow_event(self, event);
            return;
        }

        // Providers dialog events need special handling even when a dialog is open.
        if matches!(
            event,
            Event::ProvidersDialog
                | Event::ProvidersSelectModel { .. }
                | Event::ProvidersDisconnect { .. }
                | Event::ProvidersAdd
        ) {
            login_flow::providers_event(self, event);
            return;
        }

        // Dialog events are handled separately
        if self.open_dialog.is_some() {
            // Intercept Esc/DialogBack while login flow is active: redirect
            // to login_flow_cancel so Esc closes the login flow (returning to
            // the providers dialog) instead of being absorbed by the panel
            // stack handler which would leave the dialog open.
            if self.login_flow.is_some() && event == Event::DialogBack {
                login_flow::login_flow_cancel(self);
                return;
            }
            dialog_stack::update_dialog(self, event);
            return;
        }

        // Dispatch to specialized handlers
        match event {
            Event::Input(_)
            | Event::Backspace
            | Event::Newline
            | Event::CursorLeft
            | Event::CursorRight
            | Event::CursorStart
            | Event::CursorEnd
            | Event::DeleteWord
            | Event::DeleteToEnd
            | Event::DeleteToStart
            | Event::KillChar
            | Event::Undo
            | Event::Redo
            | Event::CursorWordLeft
            | Event::CursorWordRight
            | Event::Paste(_)
            | Event::PasteImage
            | Event::Submit
            | Event::HistoryPrev
            | Event::HistoryNext => self.input_event(event),
            Event::AgentThinking { .. }
            | Event::AgentThoughtDone { .. }
            | Event::AgentToolStart { .. }
            | Event::AgentToolEnd { .. }
            | Event::AgentResponse { .. }
            | Event::AgentTurnComplete { .. }
            | Event::AgentDone { .. }
            | Event::AgentError { .. } => self.agent_event(event),
            Event::ScrollUp | Event::ScrollDown | Event::PageUp | Event::PageDown => {
                scroll::scroll_event(self, event)
            }
            Event::Quit
            | Event::Reset
            | Event::Abort
            | Event::ExternalEditorDone { .. }
            | Event::SpawnAgent { .. }
            | Event::Suspend
            | Event::ShareSession
            | Event::OpenExternalEditor => control::control_event(self, event),
            Event::SwitchModel { .. }
            | Event::SwitchTheme { .. }
            | Event::CycleModelNext
            | Event::CycleModelPrev
            | Event::CycleThinkingLevel
            | Event::SetThinkingLevel(_)
            | Event::ToggleReadOnly
            | Event::TrustProject
            | Event::UntrustProject
            | Event::FollowUp
            | Event::Dequeue => model_config::model_config_event(self, event),
            Event::ToggleExpand
            | Event::ToggleSessionTree
            | Event::SessionTreeFilterCycle
            | Event::ForkSession { .. }
            | Event::CloneSession
            | Event::SessionTreeSelect { .. } => control::control_event(self, event),
            Event::ToggleCommandPalette
            | Event::ToggleModelSelector
            | Event::ToggleScopedModelsDialog
            | Event::ScopedModelToggle { .. }
            | Event::ScopedModelEnableAll
            | Event::ScopedModelDisableAll
            | Event::ScopedModelToggleProvider { .. }
            | Event::AtFilePicker => dialog_toggle::dialog_toggle_event(self, event),
            Event::InsertAtRef(_) => self.input_event(event),
            Event::ToggleSettingsDialog
            | Event::SettingsUp
            | Event::SettingsDown
            | Event::SettingsLeft
            | Event::SettingsRight
            | Event::SettingsSelect
            | Event::SettingsClose
            | Event::PaletteFilter(_)
            | Event::PaletteBackspace
            | Event::PaletteUp
            | Event::PaletteDown
            | Event::PaletteSelect
            | Event::PaletteClose
            | Event::ModelSelectorFilter(_)
            | Event::ModelSelectorBackspace
            | Event::ModelSelectorUp
            | Event::ModelSelectorDown
            | Event::ModelSelectorSelect
            | Event::ModelSelectorClose => dialog_toggle::dialog_toggle_event(self, event),
            Event::CommandFormInput(_)
            | Event::CommandFormBackspace
            | Event::CommandFormUp
            | Event::CommandFormDown
            | Event::CommandFormSubmit
            | Event::CommandFormClose => dialog_stack::handle_form_dialog(self, event),
            Event::PendingEdit { .. }
            | Event::ApproveEdit
            | Event::RejectEdit
            | Event::ReloadAll
            | Event::ShowDiagnostics
            | Event::TogglePathCompletion
            | Event::PathCompletionUp
            | Event::PathCompletionDown
            | Event::PathCompletionSelect
            | Event::PathCompletionClose
            | Event::RunSaveCommand { .. }
            | Event::RunLoadCommand { .. }
            | Event::RunDeleteCommand { .. }
            | Event::RunImportCommand { .. }
            | Event::RunExportCommand { .. }
            | Event::RunSkillCommand { .. }
            | Event::RunLoginCommand { .. }
            | Event::RunLogoutCommand { .. }
            | Event::RunNameCommand { .. }
            | Event::RunForkCommand { .. }
            | Event::RunCompactCommand { .. }
            | Event::RunPromptCommand { .. }
            | Event::RunThinkingCommand { .. }
            | Event::RunPaletteCommand { .. } => self.edit_event(event),
            Event::SystemMessage { content } => self.add_system_msg(content),
            Event::TransientMessage { content, level } => self.set_transient(content, level),
            Event::TransientError { content } => {
                self.set_transient(content, crate::event::TransientLevel::Error)
            }
            Event::ClearTransient => self.clear_transient(),
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
            Event::SpawnAgent { .. }
            | Event::Suspend
            | Event::ShareSession
            | Event::OpenExternalEditor => {}
            Event::ExternalEditorDone { content } => {
                self.input.input = content;
                self.input.cursor_pos = self.input.input.len();
                self.mark_dirty();
            }
            Event::ToggleExpand => self.toggle_expand_all(),
            Event::ToggleSessionTree => self.toggle_session_tree_dialog(),
            Event::SessionTreeFilterCycle => self.cycle_session_tree_filter(),
            Event::ForkSession { message_index } => self.fork_session_at(message_index),
            Event::CloneSession => self.clone_session(),
            Event::SessionTreeSelect { id } => self.session_tree_select(&id),
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
            Event::InsertAtRef(path) => dialog_stack::insert_at_ref(self, &path),
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

    // === Edit Event Handler ===
    fn edit_event(&mut self, event: Event) {
        match event {
            Event::PendingEdit {
                path,
                original,
                proposed,
                diff,
            } => {
                self.session.pending_edits
                    .push(crate::edit_preview::EditPreview::new(
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
            // Form submit events - execute the command
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
        dialog_stack::process_command_result(self, result);
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
                self.session.session_display_name =
                    session.display_name.or(Some(session.name));
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
                    self.session.session_display_name =
                        session.display_name.or(Some(session.name));
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

    /// Connect a provider programmatically. Opens the providers dialog to guide
    /// the user through the full login flow.
    fn run_login_command(&mut self, _provider: &str, _token: &str) {
        // Redirect to the providers dialog for a guided flow
        login_flow::providers_event(self, crate::Event::ProvidersDialog);
    }

    /// Disconnect a provider programmatically. Removes it from config.toml.
    fn run_logout_command(&mut self, provider: &str) {
        if provider.is_empty() {
            // Redirect to the providers dialog
            login_flow::providers_event(self, crate::Event::ProvidersDialog);
            return;
        }
        match crate::login_config::remove_provider_config(provider) {
            Ok(()) => {
                // If this was the active provider, switch to another one or clear.
                if self.config.current_provider == provider {
                    let configured = crate::login_config::list_configured_providers();
                    if let Some((name, _, models)) = configured.first() {
                        self.config.current_provider = name.clone();
                        self.config.current_model = models.first().cloned().unwrap_or_default();
                    } else {
                        self.config.current_provider.clear();
                        self.config.current_model.clear();
                    }
                }
                self.add_system_msg(format!(
                    "Disconnected '{}'. Use /providers to manage providers.",
                    provider
                ));
            }
            Err(e) => self.add_system_msg(format!("Could not remove provider config: {}", e)),
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

    // === View & Config Helpers ===

    fn toggle_expand_all(&mut self) {
        self.view.all_collapsed = !self.view.all_collapsed;
        self.messages_changed();
    }

    fn switch_model(&mut self, provider: String, model: String) {
        if self.config.current_provider == provider && self.config.current_model == model {
            return;
        }
        self.config.current_provider = provider.clone();
        self.config.current_model = model.clone();
        self.record_model_usage(&provider, &model);
        self.config.telemetry.track_event("model_switch", {
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
        if self.config.theme_name == name {
            return;
        }
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
        let target_id = self
            .agent
            .current_request_id
            .clone()
            .or_else(|| {
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
            tc.timestamp = now();
            self.session.messages.push(tc);
            self.messages_changed();
        }
    }
}
