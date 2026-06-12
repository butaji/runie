use crate::dialog::{Panel, PanelStack};
use crate::login_flow::LoginStep;
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
mod edit_approval;
mod input;
mod input_scroll;
mod input_text;
mod line_nav;
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
            self.login_flow_event(event);
            return;
        }

        // Dialog events are handled separately
        if self.open_dialog.is_some() {
            self.update_dialog(event);
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
                self.scroll_event(event)
            }
            Event::Quit
            | Event::Reset
            | Event::Abort
            | Event::ExternalEditorDone { .. }
            | Event::SpawnAgent { .. }
            | Event::Suspend
            | Event::ShareSession
            | Event::OpenExternalEditor => self.control_event(event),
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
            | Event::Dequeue => self.model_config_event(event),
            Event::ToggleExpand
            | Event::ToggleSessionTree
            | Event::SessionTreeFilterCycle
            | Event::ForkSession { .. }
            | Event::CloneSession => self.session_event(event),
            Event::ToggleCommandPalette
            | Event::ToggleModelSelector
            | Event::ToggleScopedModelsDialog
            | Event::ScopedModelToggle { .. }
            | Event::ScopedModelEnableAll
            | Event::ScopedModelDisableAll
            | Event::ScopedModelToggleProvider { .. }
            | Event::AtFilePicker => self.dialog_toggle_event(event),
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
            | Event::ModelSelectorClose => self.settings_event(event),
            Event::CommandFormInput(_)
            | Event::CommandFormBackspace
            | Event::CommandFormUp
            | Event::CommandFormDown
            | Event::CommandFormSubmit
            | Event::CommandFormClose => self.form_dialog_event(event),
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
            Event::LoginFlowStart
            | Event::LoginFlowSelectProvider { .. }
            | Event::LoginFlowSubmitKey { .. }
            | Event::LoginFlowValidate { .. }
            | Event::LoginFlowValidationDone { .. }
            | Event::LoginFlowValidationFailed { .. }
            | Event::LoginFlowModelsFetched { .. }
            | Event::LoginFlowToggleModel { .. }
            | Event::LoginFlowSave
            | Event::LoginFlowCancel => self.login_flow_event(event),
            Event::SystemMessage { content } => self.add_system_msg(content),
            Event::TransientMessage { content, level } => self.set_transient(content, level),
            Event::TransientError { content } => {
                self.set_transient(content, crate::event::TransientLevel::Error)
            }
            Event::ClearTransient => self.clear_transient(),
            _ => {}
        }
    }

    fn login_flow_event(&mut self, event: Event) {
        match event {
            Event::LoginFlowStart => self.login_flow_start(),
            Event::LoginFlowSelectProvider { provider } => {
                self.login_flow_select_provider(provider)
            }
            Event::LoginFlowSubmitKey { provider, key } => {
                self.login_flow_submit_key(provider, key)
            }
            Event::LoginFlowValidationDone { models, .. } => {
                self.login_flow_validation_done(models)
            }
            Event::LoginFlowValidationFailed { error, .. } => {
                self.login_flow_validation_failed(error)
            }
            Event::LoginFlowModelsFetched { models, .. } => self.login_flow_models_fetched(models),
            Event::LoginFlowToggleModel { model } => self.login_flow_toggle_model(model),
            Event::LoginFlowSave => self.login_flow_save(),
            Event::LoginFlowCancel => self.login_flow_cancel(),
            _ => {}
        }
    }

    fn login_flow_start(&mut self) {
        self.login_flow = Some(crate::login_flow::LoginFlowState::new());
        self.rebuild_login_dialog();
    }

    fn login_flow_select_provider(&mut self, provider: String) {
        let provider_clone = provider.clone();
        if let Some(ref mut flow) = self.login_flow {
            *flow = flow.clone().with_provider(provider);
            self.mark_dirty();
        }
        // Push the key input panel onto the real login stack (root +
        // pushed). ESC / Cancel will pop back to the provider picker.
        self.push_login_panel(crate::login_flow::build_key_input(&provider_clone));
    }

    fn login_flow_submit_key(&mut self, provider: String, key: String) {
        // Compute defaults + final provider first (immutable borrows).
        let final_provider = if provider.is_empty() {
            self.login_flow
                .as_ref()
                .map(|f| f.provider.clone())
                .unwrap_or_default()
        } else {
            provider.clone()
        };
        let defaults: Vec<String> = crate::provider_registry::find_provider(&final_provider)
            .map(|meta| meta.default_models.iter().map(|s| s.to_string()).collect())
            .unwrap_or_default();
        // Update state (mutable borrow).
        if let Some(ref mut flow) = self.login_flow {
            *flow = flow.clone().with_key_and_defaults(key, defaults);
            flow.provider = final_provider.clone();
        }
        // Replace the key input panel with the model selector on the
        // real login stack. The key input is "consumed" (submitted) —
        // it should NOT remain in the back stack, otherwise Esc from
        // the model selector would pop back to a stale key input.
        if let Some(flow) = self.login_flow.as_ref() {
            self.replace_top_login_panel_with(crate::login_flow::build_model_selector(flow));
        }
    }

    fn login_flow_validation_done(&mut self, models: Vec<String>) {
        if let Some(ref mut flow) = self.login_flow {
            // Non-blocking: enrich the model list in place on the top
            // panel (model selector). We do NOT push a new panel.
            *flow = flow.clone().with_fetched_models(models);
            self.replace_top_login_panel();
            self.mark_dirty();
        }
    }

    fn login_flow_models_fetched(&mut self, models: Vec<String>) {
        if let Some(ref mut flow) = self.login_flow {
            if flow.step == LoginStep::ModelSelect {
                *flow = flow.clone().with_fetched_models(models);
                self.replace_top_login_panel();
                self.mark_dirty();
            }
        }
    }

    fn login_flow_validation_failed(&mut self, error: String) {
        // Non-blocking: surface a transient warning, do NOT change the step
        // or the panel stack.
        if let Some(ref flow) = self.login_flow {
            if flow.step == LoginStep::ModelSelect {
                self.set_transient(
                    format!("Could not verify key: {}", error),
                    crate::event::TransientLevel::Warning,
                );
                self.mark_dirty();
            }
        }
    }

    fn login_flow_toggle_model(&mut self, model: String) {
        if let Some(ref mut flow) = self.login_flow {
            flow.toggle_model(&model);
            // Refresh the top panel to reflect the new toggle state.
            self.replace_top_login_panel();
            self.mark_dirty();
        }
    }

    fn login_flow_save(&mut self) {
        if let Some(ref flow) = self.login_flow {
            let base_url = crate::provider_registry::find_provider(&flow.provider)
                .map(|p| p.base_url.to_string())
                .unwrap_or_default();
            let _ = crate::login_config::save_provider_config(
                &flow.provider,
                &base_url,
                &flow.key,
                &flow.selected_models.iter().cloned().collect::<Vec<_>>(),
            );
        }
        // Save closes the whole dialog (final action).
        self.open_dialog = None;
        self.login_flow = None;
        self.mark_dirty();
    }

    fn login_flow_cancel(&mut self) {
        // Cancel pops one level. At the root (provider picker), the pop
        // is a no-op and we close the dialog. This mirrors the ESC
        // stack-navigation semantic so the cancel button and ESC behave
        // identically.
        self.pop_login_panel_or_close();
    }

    /// Pop the top panel of the login stack. If we're at the root, close
    /// the entire dialog (and clear `login_flow`). The pop also updates
    /// `LoginFlowState::step` to reflect the panel we returned to.
    fn pop_login_panel_or_close(&mut self) {
        if self.login_flow.is_none() {
            return;
        }
        let mut stack = self.take_or_create_login_stack();
        if stack.len() > 1 {
            stack.pop();
            // Update step to reflect the panel we returned to.
            if let Some(flow) = self.login_flow.as_mut() {
                flow.step = match stack.current().map(|p| p.id.as_str()) {
                    Some("login-provider") => LoginStep::ProviderPicker,
                    Some("login-key") => LoginStep::KeyInput,
                    Some("login-models") => LoginStep::ModelSelect,
                    _ => flow.step.clone(),
                };
            }
            self.open_dialog = Some(crate::commands::DialogState::PanelStack(stack));
            self.mark_dirty();
        } else {
            // At the root: close the dialog.
            self.open_dialog = None;
            self.login_flow = None;
            self.mark_dirty();
        }
    }

    /// Push a panel onto the login stack (and set the step on the state).
    fn push_login_panel(&mut self, panel: Panel) {
        if let Some(flow) = self.login_flow.as_mut() {
            flow.step = match panel.id.as_str() {
                "login-provider" => LoginStep::ProviderPicker,
                "login-key" => LoginStep::KeyInput,
                "login-models" => LoginStep::ModelSelect,
                _ => flow.step.clone(),
            };
        }
        let mut stack = self.take_or_create_login_stack();
        stack.push(panel);
        self.open_dialog = Some(crate::commands::DialogState::PanelStack(stack));
    }

    /// Replace the top panel of the login stack with a freshly built one
    /// from the current `LoginFlowState`. Used to update the model
    /// selector when models are fetched or a model is toggled.
    fn replace_top_login_panel(&mut self) {
        let flow = self.login_flow.as_ref().cloned();
        let Some(flow) = flow else {
            return;
        };
        let mut stack = self.take_or_create_login_stack();
        if let Some(last) = stack.panels.last_mut() {
            *last = match last.id.as_str() {
                "login-models" => crate::login_flow::build_model_selector(&flow),
                "login-key" => crate::login_flow::build_key_input(&flow.provider),
                "login-provider" => crate::login_flow::build_provider_picker(),
                _ => crate::login_flow::build_model_selector(&flow),
            };
        }
        self.open_dialog = Some(crate::commands::DialogState::PanelStack(stack));
    }

    /// Replace the top panel of the login stack with `new_top`, popping
    /// the current top first. Used when a panel is "consumed" (e.g. the
    /// key input is submitted → model selector). The consumed panel
    /// is removed from the back stack so Esc from the new top goes to
    /// the logical parent, not back to the consumed panel.
    fn replace_top_login_panel_with(&mut self, new_top: crate::dialog::Panel) {
        let mut stack = self.take_or_create_login_stack();
        if !stack.is_empty() {
            stack.pop();
        }
        stack.push(new_top);
        self.open_dialog = Some(crate::commands::DialogState::PanelStack(stack));
    }

    /// Take the current login PanelStack out of `open_dialog`, or build a
    /// fresh root stack if there is no dialog (e.g. we are being called
    /// from an item activation that took the dialog out, or the dialog
    /// was never opened). The login flow owns its dialog; rebuilding the
    /// root is always safe.
    fn take_or_create_login_stack(&mut self) -> PanelStack {
        if let Some(crate::commands::DialogState::PanelStack(stack)) = self.open_dialog.take() {
            stack
        } else {
            crate::login_flow::build_login_root()
        }
    }

    fn rebuild_login_dialog(&mut self) {
        // Open the login dialog with the root panel (provider picker).
        // Subsequent steps push panels onto this stack instead of
        // rebuilding it from scratch. If another dialog (e.g. the
        // command palette) is currently open, push it onto the global
        // back stack so Esc returns to it (Android-like).
        if self.login_flow.is_some() {
            if let Some(current) = self.open_dialog.take() {
                self.dialog_back_stack.push(current);
            }
            let stack = crate::login_flow::build_login_root();
            self.open_dialog = Some(crate::commands::DialogState::PanelStack(stack));
            self.mark_dirty();
        }
    }

    // === Scroll Event Handler ===
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
            Event::SpawnAgent { .. }
            | Event::Suspend
            | Event::ShareSession
            | Event::OpenExternalEditor => {}
            Event::ExternalEditorDone { content } => {
                self.input.input = content;
                self.input.cursor_pos = self.input.input.len();
                self.mark_dirty();
            }
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
            _ => {}
        }
    }

    // === Dialog Toggle Event Handler ===
    fn dialog_toggle_event(&mut self, event: Event) {
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

    fn open_command_palette(&mut self) {
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

    fn open_model_selector(&mut self) {
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

    fn open_settings_dialog(&mut self) {
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

    fn open_scoped_models_dialog(&mut self) {
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

    fn open_session_tree_dialog(&mut self) {
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
    fn edit_event(&mut self, event: Event) {
        match event {
            Event::PendingEdit {
                path,
                original,
                proposed,
                diff,
            } => {
                self.pending_edits
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

    /// Handles dialog-specific events.
    /// Esc (Abort) always closes any dialog. Global events pass through.
    fn update_dialog(&mut self, event: Event) {
        if matches!(event, Event::Abort) {
            self.open_dialog = None;
            self.mark_dirty();
            return;
        }
        if matches!(
            event,
            Event::SwitchTheme { .. }
                | Event::SwitchModel { .. }
                | Event::CycleModelNext
                | Event::CycleModelPrev
                | Event::CycleThinkingLevel
                | Event::SetThinkingLevel(_)
                | Event::ToggleReadOnly
                | Event::TrustProject
                | Event::UntrustProject
        ) {
            self.model_config_event(event);
            return;
        }
        if matches!(event, Event::Quit) {
            self.should_quit = true;
            return;
        }

        let Some(mut dialog) = self.open_dialog.take() else {
            return;
        };
        let stack = dialog.panel_stack_mut();
        let activated = self.update_panel_stack(event, stack);
        // If the panel stack activated an item, it may have closed or replaced
        // the dialog (e.g. opening a settings dialog). Otherwise restore the
        // original variant so CommandPalette/Settings/etc. identity is preserved.
        // Exception: if the handler already set `open_dialog` (e.g. login flow
        // rebuild on keep_open Emit), leave it alone.
        if !activated && self.open_dialog.is_none() {
            // If the panel stack was closed at its root AND the global
            // back stack has a previous dialog (e.g. the command
            // palette that pushed this sub-dialog), restore it instead
            // of closing the whole UI. This is the Android-like Back
            // behavior: pop one level, close only at the root of all.
            if let Some(previous) = self.dialog_back_stack.pop() {
                self.open_dialog = Some(previous);
            } else {
                self.open_dialog = Some(dialog);
            }
        }
        self.mark_dirty();
    }

    /// Update a panel stack in response to an event. Returns `true` if an item
    /// was activated (which may have closed or replaced the dialog).
    fn update_panel_stack(&mut self, event: Event, stack: &mut crate::dialog::PanelStack) -> bool {
        use Event::*;

        // Form dialog handling - check if current panel is a form
        let is_form = stack.current().is_some_and(|p| p.is_form());
        if is_form {
            return self.update_form_panel(event, stack);
        }

        match event {
            // ESC / close-key: stack nav. Pop if deeper, close at root.
            SettingsClose | PaletteClose | ModelSelectorClose | DialogBack => {
                if stack.len() > 1 {
                    stack.pop();
                    // fall through to mark_dirty + return false (keep open
                    // with the popped stack; update_dialog restores the
                    // original DialogState variant around the mutated stack)
                } else {
                    // At the root of this dialog. If the global back
                    // stack has a previous dialog (e.g. the command
                    // palette that pushed this sub-dialog), restore it
                    // — Android-like: pop one level, close only at the
                    // absolute root.
                    if let Some(previous) = self.dialog_back_stack.pop() {
                        self.open_dialog = Some(previous);
                        self.mark_dirty();
                        return false; // keep open with restored dialog
                    } else {
                        self.open_dialog = None;
                        self.mark_dirty();
                        return true; // closed at the absolute root
                    }
                }
            }
            HistoryPrev | SettingsUp | PaletteUp | ModelSelectorUp => stack.select_up(),
            HistoryNext | SettingsDown | PaletteDown | ModelSelectorDown => stack.select_down(),
            CursorLeft | SettingsLeft => {
                stack.pop();
            }
            Submit | SettingsSelect | PaletteSelect | ModelSelectorSelect => {
                // Always return after activation: the handler may have
                // replaced `open_dialog` (e.g. login flow rebuild), and
                // we must not overwrite it with the pre-activation stack.
                return self.try_activate_panel(stack);
            }
            PaletteFilter(c) | ModelSelectorFilter(c) | Input(c) => stack.push_filter(c),
            PaletteBackspace | ModelSelectorBackspace | Backspace => stack.pop_filter(),
            _ => {}
        }
        // The caller (`update_dialog`) persists the dialog by restoring the
        // original `DialogState` variant with the modified stack. We must
        // not overwrite `open_dialog` here, or the original variant
        // (e.g. CommandPalette, Settings) is lost. Handlers that need to
        // replace the dialog (e.g. login flow on keep_open Emit) set
        // `open_dialog` directly and return early via the Submit arm.
        self.mark_dirty();
        false
    }

    fn update_form_panel(&mut self, event: Event, stack: &mut crate::dialog::PanelStack) -> bool {
        let action = {
            let panel = stack.current_mut().expect("form panel");
            Self::form_panel_action(panel, event)
        };

        // Stack navigation: pop if the stack is deeper than the root,
        // otherwise close the entire dialog.
        if matches!(&action, FormAction::Back) {
            if stack.len() > 1 {
                stack.pop();
                self.open_dialog = Some(crate::commands::DialogState::PanelStack(stack.clone()));
                return false; // keep open with popped stack
            } else {
                // At the root of this dialog. If the global back stack
                // has a previous dialog, restore it (Android-like).
                if let Some(previous) = self.dialog_back_stack.pop() {
                    self.open_dialog = Some(previous);
                    self.mark_dirty();
                    return false; // keep open with restored dialog
                } else {
                    self.open_dialog = None;
                    self.mark_dirty();
                    return true; // closed at the absolute root
                }
            }
        }

        let keep_open = matches!(&action, FormAction::KeepOpen);
        if keep_open {
            self.open_dialog = Some(crate::commands::DialogState::PanelStack(stack.clone()));
        }
        self.apply_form_action(action);
        !keep_open
    }

    /// Map a single event to an action on a form panel. Pure: no I/O on `self`.
    pub fn form_panel_action(panel: &mut crate::dialog::Panel, event: Event) -> FormAction {
        use Event::*;
        use FormAction as A;
        match event {
            // ESC / close-key: stack nav. `update_form_panel` decides
            // whether to pop (stack deeper) or close (at root).
            SettingsClose | CommandFormClose | DialogBack => A::Back,
            CommandFormUp | HistoryPrev | SettingsUp | PaletteUp | ModelSelectorUp => {
                let _ = panel.select_up();
                A::KeepOpen
            }
            CommandFormDown | HistoryNext | SettingsDown | PaletteDown | ModelSelectorDown => {
                let _ = panel.select_down();
                A::KeepOpen
            }
            CommandFormInput(c) | Input(c) => {
                // If on a form field, type the character into the field.
                // If on a button, check for accelerator match instead.
                if panel.selected_form_field().is_some() {
                    Self::form_panel_edit_char(panel, c, true);
                    A::KeepOpen
                } else if let Some(crate::dialog::ItemAction::Emit(evt)) =
                    panel.find_button_by_accel(c)
                {
                    A::Submit(Some(evt.clone()))
                } else {
                    A::KeepOpen
                }
            }
            CommandFormBackspace | Backspace => {
                Self::form_panel_edit_char(panel, ' ', false);
                A::KeepOpen
            }
            CommandFormSubmit | Submit | SettingsSelect | PaletteSelect => {
                // If the selection is on a button (Action/FormSubmit), activate it
                // instead of submitting the whole form.
                if let Some(item) = panel.selected_item() {
                    match item {
                        crate::dialog::PanelItem::Action {
                            action: crate::dialog::ItemAction::Emit(evt),
                            ..
                        } => {
                            return A::Submit(Some(evt.clone()));
                        }
                        crate::dialog::PanelItem::Action { .. }
                        | crate::dialog::PanelItem::FormSubmit => {
                            return A::Submit(None);
                        }
                        _ => {}
                    }
                }
                A::Submit(Self::form_build_submit(panel))
            }
            _ => A::KeepOpen,
        }
    }

    /// Append (push=true) or delete one char (push=false) from the selected form field.
    fn form_panel_edit_char(panel: &mut crate::dialog::Panel, c: char, push: bool) {
        let Some(idx) = panel.selected_form_field() else {
            return;
        };
        let crate::dialog::PanelItem::FormField { value, key, .. } = &mut panel.items[idx] else {
            return;
        };
        if push {
            value.push(c);
        } else {
            value.pop();
        }
        panel.form_values.insert(key.clone(), value.clone());
    }

    fn try_activate_panel(&mut self, stack: &mut crate::dialog::PanelStack) -> bool {
        if let Some(action) = stack.activate() {
            if self.handle_panel_action(action, stack) {
                return true;
            }
        }
        false
    }

    /// Handle a panel item action. Returns `true` if the dialog was closed.
    fn handle_panel_action(
        &mut self,
        action: crate::dialog::ItemAction,
        stack: &mut crate::dialog::PanelStack,
    ) -> bool {
        use crate::dialog::ItemAction;
        match action {
            ItemAction::Push(_) | ItemAction::Pop => {
                stack.pop();
                false
            }
            ItemAction::Close => {
                self.open_dialog = None;
                self.mark_dirty();
                true
            }
            ItemAction::Emit(evt) => {
                let keep_open = stack
                    .current()
                    .map(|p| p.keep_open_on_activate)
                    .unwrap_or(false);
                if !keep_open {
                    self.open_dialog = None;
                }
                self.mark_dirty();
                self.update(evt);
                !keep_open
            }
            ItemAction::Toggle(key) => {
                self.panel_toggle_item(stack, &key);
                false
            }
            ItemAction::Cycle(key) => {
                self.panel_cycle_item(stack, &key);
                false
            }
        }
    }

    fn panel_toggle_item(&mut self, stack: &mut crate::dialog::PanelStack, key: &str) {
        use crate::dialog::PanelItem;
        if let Some(PanelItem::Toggle { value, .. }) =
            stack.current_mut().and_then(|p| p.selected_item_mut())
        {
            *value = !*value;
        }
        self.apply_panel_setting(key);
    }

    fn panel_cycle_item(&mut self, stack: &mut crate::dialog::PanelStack, key: &str) {
        use crate::dialog::PanelItem;
        if let Some(PanelItem::Select {
            current, options, ..
        }) = stack.current_mut().and_then(|p| p.selected_item_mut())
        {
            if let Some(idx) = options.iter().position(|o| o == current) {
                let next = (idx + 1) % options.len();
                *current = options[next].clone();
            }
        }
        self.apply_panel_setting(key);
    }

    fn apply_panel_setting(&mut self, key: &str) {
        match key {
            "read_only" => {
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
            "steering_mode" => {
                self.steering_mode = match self.steering_mode {
                    crate::model::DeliveryMode::OneAtATime => crate::model::DeliveryMode::All,
                    crate::model::DeliveryMode::All => crate::model::DeliveryMode::OneAtATime,
                };
            }
            "follow_up_mode" => {
                self.follow_up_mode = match self.follow_up_mode {
                    crate::model::DeliveryMode::OneAtATime => crate::model::DeliveryMode::All,
                    crate::model::DeliveryMode::All => crate::model::DeliveryMode::OneAtATime,
                };
            }
            _ => {}
        }
    }

    fn process_command_result(&mut self, result: crate::commands::CommandResult) {
        match result {
            crate::commands::CommandResult::Message(msg) => self.add_system_msg(msg),
            crate::commands::CommandResult::Event(evt) => self.update(evt),
            crate::commands::CommandResult::OpenDialog(d) => {
                // Android-like: if a dialog is already open (e.g. the
                // command palette, the main menu), push it onto the
                // global back stack so Esc returns to it. The new
                // dialog becomes the top.
                if let Some(current) = self.open_dialog.take() {
                    self.dialog_back_stack.push(current);
                }
                match d {
                    crate::commands::DialogType::CommandPalette => self.open_command_palette(),
                    crate::commands::DialogType::ModelSelector => self.open_model_selector(),
                    crate::commands::DialogType::Settings => self.open_settings_dialog(),
                    crate::commands::DialogType::ScopedModels => self.open_scoped_models_dialog(),
                }
            }
            crate::commands::CommandResult::OpenPanelStack(stack) => {
                if let Some(current) = self.open_dialog.take() {
                    self.dialog_back_stack.push(current);
                }
                self.open_dialog = Some(crate::commands::DialogState::PanelStack(stack));
                self.mark_dirty();
            }
            crate::commands::CommandResult::None => {}
        }
    }

    /// Open a filterable @-file picker as a PanelStack dialog.
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

    /// Insert filepath into input and close any dialog.
    pub(crate) fn insert_at_ref(&mut self, path: &str) {
        self.input.input = path.to_string();
        self.input.cursor_pos = self.input.input.len();
        self.open_dialog = None;
        self.mark_dirty();
    }

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

    /// Handle command form dialog events. Defers to `form_panel_action` after
    /// routing through the panel-stack. This is the entry point for the
    /// legacy `CommandForm*` events; `update_panel_stack` calls
    /// `update_form_panel` directly for non-CommandForm events.
    fn form_dialog_event(&mut self, event: Event) {
        let action = {
            let Some(d) = &mut self.open_dialog else {
                return;
            };
            let crate::commands::DialogState::PanelStack(stack) = d else {
                return;
            };
            let Some(panel) = stack.current_mut() else {
                return;
            };
            if !panel.is_form() {
                return;
            };
            Self::form_panel_action(panel, event)
        };
        self.apply_form_action(action);
    }

    /// Apply a `FormAction` to the current dialog. Mirrors the KeepOpen /
    /// Close / Submit paths in `update_form_panel`. `Back` is handled
    /// in `update_form_panel` itself (stack-level) and never reaches here;
    /// we include it to keep the match exhaustive.
    fn apply_form_action(&mut self, action: FormAction) {
        match action {
            FormAction::Close => {
                self.open_dialog = None;
                self.mark_dirty();
            }
            FormAction::Submit(evt) => {
                // Do NOT close the dialog here. The submit event handler
                // owns the dialog lifecycle: it may push a new panel
                // (login flow submit → model selector), close the dialog
                // (save-and-close forms), or keep it open. Closing here
                // would discard the current stack and force the handler
                // to rebuild from scratch (losing intermediate panels).
                self.mark_dirty();
                if let Some(e) = evt {
                    self.update(e);
                }
            }
            FormAction::KeepOpen => {
                self.mark_dirty();
            }
            FormAction::Back => {
                // Handled in `update_form_panel` (pop or close based on
                // stack depth). This branch is defensive in case future
                // code paths route a Back action through here.
            }
        }
    }
    /// Build the submit event for a form panel by reading form values and
    /// dispatching via the form-command table. The panel's `id` selects which
    /// command to run. Returns `None` for unknown command ids.
    pub(crate) fn form_build_submit(panel: &mut crate::dialog::Panel) -> Option<crate::Event> {
        let values = panel.get_form_values().clone();
        let cmd = panel.id.clone();
        match cmd.as_str() {
            "save" => Some(crate::Event::RunSaveCommand {
                name: values.get("name").cloned().unwrap_or_default(),
            }),
            "load" => Some(crate::Event::RunLoadCommand {
                name: values.get("name").cloned().unwrap_or_default(),
            }),
            "delete" => Some(crate::Event::RunDeleteCommand {
                name: values.get("name").cloned().unwrap_or_default(),
            }),
            "import" => Some(crate::Event::RunImportCommand {
                path: values.get("path").cloned().unwrap_or_default(),
            }),
            "export" => Some(crate::Event::RunExportCommand {
                path: values.get("path").cloned().unwrap_or_default(),
            }),
            "skill" => Some(crate::Event::RunSkillCommand {
                name: values.get("name").cloned().unwrap_or_default(),
            }),
            "login" => Some(crate::Event::RunLoginCommand {
                provider: values.get("provider").cloned().unwrap_or_default(),
                token: values.get("token").cloned().unwrap_or_default(),
            }),
            "logout" => Some(crate::Event::RunLogoutCommand {
                provider: values.get("provider").cloned().unwrap_or_default(),
            }),
            "name" => Some(crate::Event::RunNameCommand {
                name: values.get("name").cloned().unwrap_or_default(),
            }),
            "fork" => {
                let index = values.get("index").cloned().unwrap_or_default();
                Some(crate::Event::RunForkCommand {
                    message_index: index,
                })
            }
            "compact" => {
                let keep = values.get("keep").cloned().unwrap_or_default();
                let focus = values.get("focus").cloned().unwrap_or_default();
                Some(crate::Event::RunCompactCommand { keep, focus })
            }
            "prompt" => Some(crate::Event::RunPromptCommand {
                name: values.get("name").cloned().unwrap_or_default(),
            }),
            "login-key" => Some(crate::Event::LoginFlowSubmitKey {
                provider: String::new(),
                key: values.get("key").cloned().unwrap_or_default(),
            }),
            _ => None,
        }
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
