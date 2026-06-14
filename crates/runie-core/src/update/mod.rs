use crate::model::AppState;
use crate::Event;

// Re-export for backward compatibility
pub use crate::tool_markers::has_tool_markers as content_has_tool_markers;
pub use crate::tool_markers::strip_tool_markers;

mod agent;
mod at_refs;
mod bash;
mod control;
mod dialog;
pub(crate) mod dialog_form;
mod dialog_panel;
mod dialog_toggle;
mod edit;
mod edit_approval;
mod form;
pub use form::FormAction;
mod input;
mod input_dispatch;
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
mod state_helpers;
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
        if matches!(
            event,
            Event::LoginFlowStart
                | Event::LoginFlowSelectProvider { .. }
                | Event::LoginFlowSubmitKey { .. }
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

        if self.open_dialog.is_some() {
            if self.login_flow.is_some() && event == Event::DialogBack {
                login_flow::login_flow_cancel(self);
                return;
            }
            dialog::update_dialog(self, event);
            return;
        }

        // Vim-mode Esc: with vim enabled, Esc either stops an active turn
        // (first press) or enters feed-navigation mode (second press / when
        // idle). Without vim_mode, fall through to the default handler
        // (which is a no-op for DialogBack when no dialog is open).
        if event == Event::DialogBack && self.config.vim_mode {
            if self.vim_nav_mode {
                self.vim_nav_mode = false;
                self.mark_dirty();
                return;
            }
            if self.vim_nav_pending {
                self.vim_nav_pending = false;
                self.vim_nav_mode = true;
                self.mark_dirty();
                return;
            }
            if self.agent.turn_active {
                // First Esc stops the turn; a subsequent Esc enters nav.
                self.agent.turn_active = false;
                self.agent.inflight = 0;
                self.vim_nav_pending = true;
                self.mark_dirty();
                return;
            }
            self.vim_nav_mode = true;
            // Anchor the selection to the post at the bottom of the
            // viewport (closest to the input box). This matches the
            // user's expectation that pressing Esc from the input box
            // selects the newest/lowest post in the feed.
            self.view.selected_post = crate::snapshot::compute_current_bottom_element(
                &self.view.elements_cache,
                &self.view.line_counts,
                self.view.total_lines,
                self.view.scroll,
                self.view.last_visible_height,
            )
            .and_then(|elem| {
                self.view
                    .posts
                    .iter()
                    .find(|p| p.start <= elem && elem < p.end)
                    .map(|p| p.index)
            });
            self.mark_dirty();
            return;
        }

        // Vim nav mode: route motion keys before generic input handling.
        if self.vim_nav_mode {
            if let Some(handled) = self.handle_vim_nav_event(&event) {
                if !handled {
                    return;
                }
            }
        }

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
            | Event::HistoryNext => input_dispatch::input_event(self, event),
            Event::AgentThinking { .. }
            | Event::AgentThoughtDone { .. }
            | Event::AgentToolStart { .. }
            | Event::AgentToolEnd { .. }
            | Event::AgentResponse { .. }
            | Event::AgentTurnComplete { .. }
            | Event::AgentDone { .. }
            | Event::AgentError { .. } => agent::agent_event(self, event),
            Event::ScrollUp
            | Event::ScrollDown
            | Event::PageUp
            | Event::PageDown
            | Event::GoToTop
            | Event::GoToBottom => scroll::scroll_event(self, event),
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
            | Event::Dequeue
            | Event::ToggleVimMode => model_config::model_config_event(self, event),
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
            Event::OpenAgentsManager
            | Event::AgentsManagerSetField { .. }
            | Event::AgentsManagerSave { .. }
            | Event::AgentsManagerDelete { .. } => {
                crate::commands::agents_manager::agents_manager_event(self, event)
            }
            Event::InsertAtRef(_) => input_dispatch::input_event(self, event),
            Event::CopyToClipboard(_) | Event::CopyLastResponse => {
                control::control_event(self, event)
            }
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
            | Event::CommandFormClose => dialog::handle_form_dialog(self, event),
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
            | Event::RunPaletteCommand { .. } => edit::update(self, event),
            Event::SystemMessage { content } => self.add_system_msg(content),
            Event::TransientMessage { content, level } => self.set_transient(content, level),
            Event::TransientError { content } => {
                self.set_transient(content, crate::event::TransientLevel::Error)
            }
            Event::ClearTransient => self.clear_transient(),
            Event::MouseClick { .. }
            | Event::MouseRelease { .. }
            | Event::MouseDrag { .. }
            | Event::MouseMove { .. }
            | Event::FocusGained
            | Event::FocusLost
            | Event::DialogBack => {}
            Event::SettingsSwitchCategory { category } => {
                settings_dialog::handle_settings_category(self, category)
            }
            // Handled by early returns above; unreachable here.
            Event::ProvidersDialog
            | Event::ProvidersSelectModel { .. }
            | Event::ProvidersDisconnect { .. }
            | Event::ProvidersAdd
            | Event::LoginFlowStart
            | Event::LoginFlowSelectProvider { .. }
            | Event::LoginFlowSubmitKey { .. }
            | Event::LoginFlowValidationDone { .. }
            | Event::LoginFlowValidationFailed { .. }
            | Event::LoginFlowModelsFetched { .. }
            | Event::LoginFlowToggleModel { .. }
            | Event::LoginFlowSave
            | Event::LoginFlowCancel => unreachable!(),
        }
    }
}
