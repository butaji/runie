use crate::model::{AppState, ChatMessage, Role};
use crate::Event;

// Re-export for backward compatibility
pub use crate::tool_markers::has_tool_markers as content_has_tool_markers;
pub use crate::tool_markers::strip_tool_markers;

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
mod session;
pub mod settings_dialog;
mod system_actions;
mod palette;

pub(crate) fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

impl AppState {
    /// Main event dispatcher - delegates to specialized handlers based on event type.
    pub fn update(&mut self, event: Event) {
        // Dialog events are handled separately
        if self.open_dialog.is_some() {
            self.update_dialog(event);
            return;
        }
        
        // Dispatch to specialized handlers
        match event {
            Event::Input(_) | Event::Backspace | Event::Newline | Event::CursorLeft 
            | Event::CursorRight | Event::CursorStart | Event::CursorEnd 
            | Event::DeleteWord | Event::DeleteToEnd | Event::DeleteToStart 
            | Event::KillChar | Event::Undo | Event::Redo | Event::CursorWordLeft 
            | Event::CursorWordRight | Event::Paste(_) | Event::PasteImage | Event::Submit
            | Event::HistoryPrev | Event::HistoryNext => self.input_event(event),
            Event::AgentThinking { .. } | Event::AgentThoughtDone { .. } 
            | Event::AgentToolStart { .. } | Event::AgentToolEnd { .. } 
            | Event::AgentResponse { .. } | Event::AgentTurnComplete { .. } 
            | Event::AgentDone { .. } | Event::AgentError { .. } => self.agent_event(event),
            Event::ScrollUp | Event::ScrollDown | Event::PageUp | Event::PageDown => self.scroll_event(event),
            Event::Quit | Event::Reset | Event::Abort | Event::ExternalEditorDone { .. } 
            | Event::SpawnAgent | Event::Suspend | Event::ShareSession | Event::OpenExternalEditor => self.control_event(event),
            Event::SwitchModel { .. } | Event::SwitchTheme { .. } | Event::CycleModelNext 
            | Event::CycleModelPrev | Event::CycleThinkingLevel | Event::SetThinkingLevel(_) 
            | Event::ToggleReadOnly | Event::TrustProject | Event::UntrustProject 
            | Event::FollowUp | Event::Dequeue => self.model_config_event(event),
            Event::ToggleExpand | Event::ToggleSessionTree | Event::SessionTreeFilterCycle 
            | Event::ForkSession { .. } | Event::CloneSession => self.session_event(event),
            Event::ToggleCommandPalette | Event::ToggleModelSelector | Event::ToggleScopedModelsDialog 
            | Event::ScopedModelToggle { .. } | Event::ScopedModelEnableAll 
            | Event::ScopedModelDisableAll | Event::ScopedModelToggleProvider { .. }
            | Event::AtFilePicker => self.dialog_toggle_event(event),
            Event::InsertAtRef(_) => self.input_event(event),
            Event::ToggleSettingsDialog | Event::SettingsUp | Event::SettingsDown 
            | Event::SettingsLeft | Event::SettingsRight | Event::SettingsSelect | Event::SettingsClose 
            | Event::PaletteFilter(_) | Event::PaletteBackspace | Event::PaletteUp 
            | Event::PaletteDown | Event::PaletteSelect | Event::PaletteClose 
            | Event::ModelSelectorFilter(_) | Event::ModelSelectorBackspace | Event::ModelSelectorUp 
            | Event::ModelSelectorDown | Event::ModelSelectorSelect | Event::ModelSelectorClose => self.settings_event(event),
            Event::PendingEdit { .. } | Event::ApproveEdit | Event::RejectEdit 
            | Event::ReloadAll | Event::ShowDiagnostics | Event::TogglePathCompletion 
            | Event::PathCompletionUp | Event::PathCompletionDown | Event::PathCompletionSelect 
            | Event::PathCompletionClose => self.edit_event(event),
            Event::SystemMessage { content } => self.add_system_msg(content),
            Event::TransientMessage { content, level } => self.set_transient(content, level),
            Event::TransientError { content } => self.set_transient(content, crate::event::TransientLevel::Error),
            Event::ClearTransient => self.clear_transient(),
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
            Event::Quit => self.should_quit = true,
            Event::Reset => *self = AppState::default(),
            Event::Abort => {
                if self.completion.path_suggestions.is_some() {
                    self.path_completion_close();
                } else {
                    self.abort_queue();
                }
            }
            Event::SpawnAgent | Event::Suspend | Event::ShareSession | Event::OpenExternalEditor => {}
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
        match event {
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
            Event::AtFilePicker => {
                self.open_at_file_picker();
            }
            Event::ScopedModelToggle { name } => scoped_models::toggle_scoped_model(self, &name),
            Event::ScopedModelEnableAll => scoped_models::enable_all(self),
            Event::ScopedModelDisableAll => scoped_models::disable_all(self),
            Event::ScopedModelToggleProvider { provider } => scoped_models::toggle_provider(self, &provider),
            _ => {}
        }
    }

    // === Settings Event Handler ===
    // === Edit Event Handler ===
    fn edit_event(&mut self, event: Event) {
        match event {
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
            _ => {}
        }
    }

    /// Handles dialog-specific events.
    /// Esc (Abort) always closes any dialog. Global events pass through.
    fn update_dialog(&mut self, event: Event) {
        if matches!(event, Event::Abort) { self.open_dialog = None; self.mark_dirty(); return; }
        if matches!(event, Event::SwitchTheme { .. } | Event::SwitchModel { .. }
            | Event::CycleModelNext | Event::CycleModelPrev
            | Event::CycleThinkingLevel | Event::SetThinkingLevel(_)
            | Event::ToggleReadOnly | Event::TrustProject | Event::UntrustProject) {
            self.model_config_event(event); return;
        }
        if matches!(event, Event::Quit) { self.should_quit = true; return; }

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
            crate::commands::DialogState::SessionTree { filter, selected } => {
                self.update_session_tree(event, filter, selected);
            }
            crate::commands::DialogState::PanelStack(mut stack) => {
                self.update_panel_stack(event, &mut stack);
            }
        }
    }

    fn update_panel_stack(&mut self, event: Event, stack: &mut crate::dialog::PanelStack) {
        use Event::*;
        match event {
            SettingsClose => { self.open_dialog = None; self.mark_dirty(); return; }
            HistoryPrev | SettingsUp | PaletteUp | ModelSelectorUp => stack.select_up(),
            HistoryNext | SettingsDown | PaletteDown | ModelSelectorDown => stack.select_down(),
            CursorLeft | SettingsLeft => { stack.pop(); }
            Submit | SettingsSelect | PaletteSelect => {
                if self.try_activate_panel(stack) { return; }
            }
            PaletteFilter(c) | ModelSelectorFilter(c) | Input(c) => stack.push_filter(c),
            PaletteBackspace | ModelSelectorBackspace | Backspace => stack.pop_filter(),
            _ => {}
        }
        self.open_dialog = Some(crate::commands::DialogState::PanelStack(stack.clone()));
        self.mark_dirty();
    }

    fn try_activate_panel(&mut self, stack: &mut crate::dialog::PanelStack) -> bool {
        if let Some(action) = stack.activate() {
            if self.handle_panel_action(action, stack) { return true; }
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
                self.open_dialog = None;
                self.mark_dirty();
                self.update(evt);
                true
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
        if let Some(PanelItem::Select { current, options, .. }) =
            stack.current_mut().and_then(|p| p.selected_item_mut())
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
                let status = if self.config.read_only { "enabled" } else { "disabled" };
                self.notify(format!("Read-only mode {}", status), crate::event::TransientLevel::Warning);
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

    /// Open a filterable @-file picker as a PanelStack dialog.
    pub(crate) fn open_at_file_picker(&mut self) {
        use crate::dialog::{ItemAction, Panel, PanelStack};
        let files = crate::file_refs::find_files("", ".", 50);
        let mut panel = Panel::new("at-files", " @ files ").with_filter();
        if files.is_empty() {
            panel = panel.header("No files found");
        } else {
            panel = panel.header(&format!("{} files", files.len()));
            for path in files {
                panel = panel.item(
                    &path,
                    ItemAction::Emit(crate::Event::InsertAtRef(path.clone())),
                );
            }
        }
        self.open_dialog = Some(crate::commands::DialogState::PanelStack(PanelStack::new(panel)));
        self.mark_dirty();
    }

    /// Insert @filepath into input and close any dialog.
    pub(crate) fn insert_at_ref(&mut self, path: &str) {
        self.input.input = format!("@{}", path);
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
        self.telemetry.track_event(
            "model_switch",
            {
                let mut m = std::collections::HashMap::new();
                m.insert("provider".into(), provider.clone());
                m.insert("model".into(), model.clone());
                m
            },
        );
        self.notify(format!("Switched to {}/{}", provider, model), crate::event::TransientLevel::Success);
    }

    fn switch_theme(&mut self, name: String) {
        self.config.theme_name = name.clone();
        self.notify(format!("Theme switched to '{}'", name), crate::event::TransientLevel::Success);
    }

    fn cycle_model(&mut self, delta: isize) {
        let enabled: Vec<usize> = self
            .config.scoped_models
            .iter()
            .enumerate()
            .filter(|(_, m)| m.enabled)
            .map(|(i, _)| i)
            .collect();
        if enabled.is_empty() { return; }
        let current_pos = enabled.iter().position(|&i| i == self.config.scoped_index).unwrap_or(0);
        let len = enabled.len() as isize;
        let new_pos = ((current_pos as isize + delta).rem_euclid(len)) as usize;
        self.config.scoped_index = enabled[new_pos];
        let model = &self.config.scoped_models[self.config.scoped_index];
        self.switch_model(model.provider.clone(), model.name.clone());
    }

    fn cycle_thinking_level(&mut self) {
        self.config.thinking_level = self.config.thinking_level.cycle();
        self.notify(format!("Thinking level: {}", self.config.thinking_level.as_str()), crate::event::TransientLevel::Info);
    }

    fn set_thinking_level(&mut self, level: crate::model::ThinkingLevel) {
        self.config.thinking_level = level;
        self.notify(format!("Thinking level set to: {}", self.config.thinking_level.as_str()), crate::event::TransientLevel::Info);
    }

    fn toggle_read_only(&mut self) {
        self.config.read_only = !self.config.read_only;
        let status = if self.config.read_only { "enabled" } else { "disabled" };
        self.notify(format!("Read-only mode {}", status), crate::event::TransientLevel::Warning);
    }

    fn trust_project(&mut self) {
        let cwd = std::env::current_dir().unwrap_or_default();
        let mut tm = crate::trust::TrustManager::load();
        tm.set(&cwd, crate::trust::TrustDecision::Trusted);
        let _ = tm.save();
        self.config.read_only = false;
        self.notify(format!("Project '{}' trusted. Read-only disabled.", cwd.display()), crate::event::TransientLevel::Success);
    }

    fn untrust_project(&mut self) {
        let cwd = std::env::current_dir().unwrap_or_default();
        let mut tm = crate::trust::TrustManager::load();
        tm.set(&cwd, crate::trust::TrustDecision::Untrusted);
        let _ = tm.save();
        self.config.read_only = true;
        self.notify(format!("Project '{}' untrusted. Read-only enabled.", cwd.display()), crate::event::TransientLevel::Warning);
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
        let target_id = self.agent.current_request_id.clone()
            .or_else(|| self.last_assistant_index
                .and_then(|idx| self.session.messages.get(idx).map(|m| m.id.clone())));
        let Some(target_id) = target_id else { return };
        if let Some(idx) = self.session.messages.iter().position(|m| m.role == Role::TurnComplete && m.id == target_id) {
            let mut tc = self.session.messages.remove(idx);
            tc.timestamp = now();
            self.session.messages.push(tc);
            self.messages_changed();
        }
    }

    fn update_session_tree(&mut self, event: Event, filter: crate::session_tree::SessionTreeFilter, selected: usize) {
        let tree = match self.session.session_tree.as_ref() {
            Some(t) => t,
            None => {
                self.open_dialog = Some(crate::commands::DialogState::SessionTree { filter, selected });
                return;
            }
        };
        let visible = tree.filtered_walk(filter);
        let count = visible.len();
        if let Some(next) = Self::session_tree_next_state(event, filter, selected, count) {
            self.open_dialog = Some(next);
            self.mark_dirty();
        } else {
            self.open_dialog = None;
            self.mark_dirty();
        }
    }

    fn session_tree_next_state(
        event: Event,
        filter: crate::session_tree::SessionTreeFilter,
        selected: usize,
        count: usize,
    ) -> Option<crate::commands::DialogState> {
        match event {
            Event::HistoryPrev => Some(crate::commands::DialogState::SessionTree {
                filter,
                selected: selected.saturating_sub(1),
            }),
            Event::HistoryNext => Some(crate::commands::DialogState::SessionTree {
                filter,
                selected: (selected + 1).min(count.saturating_sub(1)),
            }),
            Event::SessionTreeFilterCycle => Some(crate::commands::DialogState::SessionTree {
                filter: filter.cycle(),
                selected,
            }),
            _ => None,
        }
    }
}
