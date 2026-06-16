//! Event update handlers — merged dispatcher (formerly split between mod.rs and dispatch.rs).

use crate::event::{CommandEvent, DialogEvent, SessionEvent, SystemEvent};
use crate::model::AppState;
use crate::Event;

use crate::session::Session;

// Re-export for backward compatibility
pub use crate::tool_markers::has_tool_markers as content_has_tool_markers;
pub use crate::tool_markers::strip_tool_markers;


mod agent;
pub(crate) mod dialog;
pub(crate) mod input;
mod session;
mod system;
mod tools;
mod login_flow;

// These are still separate (not merged):
mod path_complete;
pub mod settings_dialog;

pub(crate) use crate::message::now;

impl AppState {
    /// Main event dispatcher — merged from update() and dispatch_event().
    pub fn update(&mut self, event: Event) {
        match event {
            Event::Show
            | Event::Hide
            | Event::FocusOrchestrator
            | Event::FocusSubagent(_)
            | Event::UpdateStatus { .. }
            | Event::SetSubagents(_)
            | Event::SetOrchestratorStatus(_) => {
                self.handle_sidebar_event(event);
                return;
            }
            Event::StateChanged { .. }
            | Event::PlanStarted
            | Event::PlanningStarted
            | Event::PlanGenerated { .. }
            | Event::PlanningFailed { .. }
            | Event::SubagentDispatched { .. }
            | Event::SubagentStatusChanged { .. }
            | Event::SubagentCompleted { .. }
            | Event::SubagentFailed { .. }
            | Event::SynthesisStarted
            | Event::SynthesisComplete { .. }
            | Event::Finished { .. }
            | Event::Cancelled => {
                self.handle_orchestrator_event(event);
                return;
            }
            _ => {}
        }
        if self.try_handle_dialog_event_input(&event) {
            return;
        }
        if self.try_handle_vim_dialog_back_input(&event) {
            return;
        }
        if self.try_handle_vim_nav_event_input(&event) {
            return;
        }
        if is_dialog_event(&event) {
            self.handle_dialog_event(&event);
        } else {
            dispatch_event(self, event);
        }
    }

    fn handle_sidebar_event(&mut self, event: crate::event::SidebarEvent) {
        match event {
            crate::event::SidebarEvent::Show => {
                self.sidebar.visible = true;
            }
            crate::event::SidebarEvent::Hide => {
                self.sidebar.visible = false;
            }
            crate::event::SidebarEvent::FocusOrchestrator => {
                self.sidebar.focus_orchestrator();
            }
            crate::event::SidebarEvent::FocusSubagent(idx) => {
                self.sidebar.focus_subagent_by_index(idx);
            }
            crate::event::SidebarEvent::UpdateStatus { id, status } => {
                if let Some(entry) = self.sidebar.agents.iter_mut().find(|a| a.id == id) {
                    entry.status = status;
                }
            }
            crate::event::SidebarEvent::SetSubagents(list) => {
                self.sidebar.set_subagents(list);
            }
            crate::event::SidebarEvent::SetOrchestratorStatus(status) => {
                self.sidebar.set_orchestrator_status(status);
            }
            _ => {}
        }
        self.mark_dirty();
    }

    fn handle_orchestrator_event(&mut self, event: crate::orchestrator_actor::OrchestratorEvent) {
        use crate::orchestrator_actor::OrchestratorEvent;
        use crate::orchestrator::TaskStatus;
        use crate::state::{AgentEntry, AgentStatus};

        match event {
            OrchestratorEvent::PlanStarted => {
                self.sidebar.visible = true;
                self.sidebar.set_orchestrator_status(AgentStatus::Running);
                self.sidebar.agents.truncate(1); // clear old subagents
            }
            OrchestratorEvent::PlanningStarted => {
                self.sidebar.set_orchestrator_status(AgentStatus::Running);
            }
            OrchestratorEvent::PlanGenerated { plan } => {
                let entries: Vec<AgentEntry> = plan.tasks.iter().map(|t| {
                    let status = match t.status {
                        TaskStatus::Pending => AgentStatus::Pending,
                        TaskStatus::Running => AgentStatus::Running,
                        TaskStatus::AwaitingUser => AgentStatus::AwaitingUser,
                        TaskStatus::Done => AgentStatus::Done,
                        TaskStatus::Failed => AgentStatus::Failed,
                    };
                    AgentEntry {
                        id: t.id.clone(),
                        label: t.task_description.chars().take(20).collect(),
                        status,
                    }
                }).collect();
                self.sidebar.set_subagents(entries);
            }
            OrchestratorEvent::SubagentStatusChanged { task_id, status } => {
                let agent_status = match status {
                    TaskStatus::Pending => AgentStatus::Pending,
                    TaskStatus::Running => AgentStatus::Running,
                    TaskStatus::AwaitingUser => AgentStatus::AwaitingUser,
                    TaskStatus::Done => AgentStatus::Done,
                    TaskStatus::Failed => AgentStatus::Failed,
                };
                if let Some(entry) = self.sidebar.agents.iter_mut().find(|a| a.id == task_id) {
                    entry.status = agent_status;
                }
            }
            OrchestratorEvent::Cancelled => {
                self.sidebar.visible = false;
                self.sidebar.agents.clear();
            }
            OrchestratorEvent::StateChanged { to, .. } => {
                self.orchestrator_state = *to;
            }
            _ => {}
        }
        self.mark_dirty();
    }

    fn handle_dialog_event(&mut self, event: &Event) {
        if is_login_flow_dialog_event(event) || is_providers_dialog_event(event) {
            dispatch_event(self, event.clone());
            return;
        }
        if self.try_handle_dialog_event_dialog(event) {
            return;
        }
        dispatch_event(self, event.clone());
    }

    fn try_handle_dialog_event_input(&mut self, event: &crate::event::InputEvent) -> bool {
        if self.open_dialog.is_none() {
            return false;
        }
        // Welcome dialog closes on any printable input or Submit
        if matches!(self.open_dialog, Some(crate::commands::DialogState::Welcome)) {
            match event {
                crate::event::InputEvent::Input(_) | crate::event::InputEvent::Submit => {
                    self.open_dialog = None;
                    self.mark_dirty();
                    return false; // also pass to input handler
                }
                _ => return false, // let other keys pass through to input
            }
        }
        match event {
            crate::event::InputEvent::Input(_)
            | crate::event::InputEvent::Submit
            | crate::event::InputEvent::Backspace
            | crate::event::InputEvent::HistoryPrev
            | crate::event::InputEvent::HistoryNext
            | crate::event::InputEvent::CursorLeft
            | crate::event::InputEvent::CursorRight => {
                dialog::update_dialog(self, event.clone());
                return true;
            }
            _ => {}
        }
        false
    }

    fn try_handle_vim_dialog_back_input(&mut self, event: &crate::event::InputEvent) -> bool {
        if *event != crate::event::InputEvent::Backspace || !self.view.vim_nav_mode {
            return false;
        }
        self.handle_vim_dialog_back();
        true
    }

    fn try_handle_vim_nav_event_input(&mut self, event: &crate::event::InputEvent) -> bool {
        if !self.view.vim_nav_mode {
            return false;
        }
        let Some(handled) = self.handle_vim_nav_event(event) else {
            return false;
        };
        !handled
    }

    fn try_handle_dialog_event_dialog(&mut self, event: &Event) -> bool {
        if self.open_dialog.is_none() {
            return false;
        }
        if self.login_flow.is_some() && matches!(event, DialogEvent::ProvidersAdd) {
            return false;
        }
        dialog::update_dialog(self, event.clone());
        true
    }

    fn handle_vim_dialog_back(&mut self) {
        if self.view.vim_nav_mode {
            self.view.vim_nav_mode = false;
            self.mark_dirty();
            return;
        }
        if self.view.vim_nav_pending {
            self.view.vim_nav_pending = false;
            self.view.vim_nav_mode = true;
            self.mark_dirty();
            return;
        }
        if self.agent.turn_active {
            self.agent.turn_active = false;
            self.agent.inflight = 0;
            self.view.vim_nav_pending = true;
            self.mark_dirty();
            return;
        }
        self.view.vim_nav_mode = true;
        self.view.selected_post = self.current_bottom_post_index();
        self.mark_dirty();
    }

    fn current_bottom_post_index(&self) -> Option<usize> {
        let bottom = crate::snapshot::compute_current_bottom_element(
            &self.view.elements_cache,
            &self.view.line_counts,
            self.view.total_lines,
            self.view.scroll,
            self.view.last_visible_height,
        )?;
        self.view
            .posts
            .iter()
            .find(|p| p.start <= bottom && bottom < p.end)
            .map(|p| p.index)
    }

    #[allow(dead_code)]
    fn handle_vim_nav_event_input(&mut self, _event: &crate::event::InputEvent) -> Option<bool> {
        None
    }
}

fn is_login_flow_dialog_event(event: &DialogEvent) -> bool {
    matches!(event, DialogEvent::ProvidersAdd)
}

fn is_providers_dialog_event(event: &DialogEvent) -> bool {
    matches!(
        event,
        DialogEvent::ProvidersDialog
            | DialogEvent::ProvidersSelectModel { .. }
            | DialogEvent::ProvidersDisconnect { .. }
            | DialogEvent::ProvidersAdd
    )
}

fn is_dialog_event(event: &Event) -> bool {
    matches!(
        event,
        Event::ToggleWelcome
            | Event::ToggleCommandPalette
            | Event::ToggleSettingsDialog
            | Event::ToggleModelSelector
            | Event::ToggleScopedModelsDialog
            | Event::ToggleVimMode
            | Event::AtFilePicker
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
            | Event::ModelSelectorClose
            | Event::TogglePathCompletion
            | Event::PathCompletionUp
            | Event::PathCompletionDown
            | Event::PathCompletionSelect
            | Event::PathCompletionClose
            | Event::CommandFormInput(_)
            | Event::CommandFormBackspace
            | Event::CommandFormUp
            | Event::CommandFormDown
            | Event::CommandFormSubmit
            | Event::CommandFormClose
            | Event::SettingsUp
            | Event::SettingsDown
            | Event::SettingsLeft
            | Event::SettingsRight
            | Event::SettingsSelect
            | Event::SettingsClose
            | Event::SettingsSwitchCategory { .. }
            | Event::ScopedModelEnableAll
            | Event::ScopedModelDisableAll
            | Event::DialogBack
            | Event::ProvidersDialog
            | Event::ProvidersSelectModel { .. }
            | Event::ProvidersDisconnect { .. }
            | Event::ProvidersAdd
            | Event::OpenAgentsManager
            | Event::AgentsManagerSetField { .. }
            | Event::AgentsManagerSave { .. }
            | Event::AgentsManagerDelete { .. }
            | Event::CopyToClipboard(_)
            | Event::CopySelectedBlock
            | Event::CopyBlockMetadata
            | Event::InsertAtRef(_)
    )
}

// ── Central dispatcher (formerly in dispatch.rs) ─────────────────────────────────

/// Dispatch an event when no dialog is open and no special early-return
/// handler has consumed it.
fn dispatch_event(state: &mut AppState, event: Event) {
    match event {
        // Input
        Event::Input(_) | Event::Backspace | Event::Newline | Event::Submit | Event::Escape | Event::CursorLeft |
        Event::CursorRight | Event::CursorStart | Event::CursorEnd | Event::DeleteWord | Event::DeleteToEnd |
        Event::DeleteToStart | Event::KillChar | Event::HistoryPrev | Event::HistoryNext | Event::Undo |
        Event::Redo | Event::CursorWordLeft | Event::CursorWordRight | Event::PageUp | Event::PageDown |
        Event::GoToTop | Event::GoToBottom | Event::Paste(_) | Event::PasteImage | Event::MouseClick { .. } |
        Event::MouseRelease { .. } | Event::MouseDrag { .. } | Event::MouseMove { .. } | Event::MouseScrollUp |
        Event::MouseScrollDown | Event::FocusGained | Event::FocusLost | Event::TerminalSize { .. } => input::input_event(state, event),
        // Agent
        Event::Thinking { .. } | Event::ThoughtDone { .. } | Event::ToolStart { .. } | Event::ToolEnd { .. } |
        Event::ResponseDelta { .. } | Event::Response { .. } | Event::TurnComplete { .. } | Event::Done { .. } |
        Event::Error { .. } => agent::agent_event(state, event),
        // Replay
        Event::MessageReplayed { id, role, content, timestamp, provider } => {
            state.replay_message(id.clone(), role.clone(), content.clone(), timestamp, provider.clone());
        }
        // Scroll
        Event::Up | Event::Down => input::scroll_event(state, event),
        // Control
        Event::Quit | Event::Reset | Event::Abort | Event::FollowUp | Event::SpawnAgent { .. } |
        Event::ToggleExpand | Event::Dequeue | Event::OpenExternalEditor | Event::ExternalEditorDone { .. } |
        Event::ShareSession | Event::Suspend | Event::ToggleVimMode | Event::CopyLastResponse |
        Event::OpenSessionList | Event::NewSession | Event::ResumeSession | Event::SelectSession { .. } |
        Event::StarSession { .. } | Event::RenameSession { .. } | Event::DeleteSession { .. } => system::control_event(state, event),
        // ModelConfig
        Event::SwitchModel { .. } | Event::SwitchTheme { .. } | Event::CycleModelNext | Event::CycleModelPrev |
        Event::ToggleScopedModelsDialog | Event::ScopedModelToggle { .. } | Event::ScopedModelEnableAll |
        Event::ScopedModelDisableAll | Event::ScopedModelToggleProvider { .. } | Event::ToggleSettingsDialog |
        Event::SettingsUp | Event::SettingsDown | Event::SettingsLeft | Event::SettingsRight |
        Event::SettingsSelect | Event::SettingsClose | Event::SettingsSwitchCategory { .. } |
        Event::CycleThinkingLevel | Event::SetThinkingLevel(_) | Event::ToggleReadOnly | Event::TrustProject |
        Event::UntrustProject | Event::ReloadAll | Event::KeybindingsReloaded => agent::model_config_event(state, event),
        // Dialog
        Event::ToggleWelcome | Event::ToggleCommandPalette | Event::PaletteFilter(_) | Event::PaletteBackspace |
        Event::PaletteUp | Event::PaletteDown | Event::PaletteSelect | Event::PaletteClose |
        Event::ToggleModelSelector | Event::ModelSelectorFilter(_) | Event::ModelSelectorBackspace |
        Event::ModelSelectorUp | Event::ModelSelectorDown | Event::ModelSelectorSelect | Event::ModelSelectorClose |
        Event::TogglePathCompletion | Event::PathCompletionUp | Event::PathCompletionDown |
        Event::PathCompletionSelect | Event::PathCompletionClose | Event::CommandFormInput(_) |
        Event::CommandFormBackspace | Event::CommandFormUp | Event::CommandFormDown | Event::CommandFormSubmit |
        Event::CommandFormClose | Event::DialogBack | Event::ProvidersDialog |
        Event::ProvidersSelectModel { .. } | Event::ProvidersDisconnect { .. } | Event::ProvidersAdd |
        Event::OpenAgentsManager | Event::AgentsManagerSetField { .. } | Event::AgentsManagerSave { .. } |
        Event::AgentsManagerDelete { .. } | Event::CopyToClipboard(_) | Event::CopySelectedBlock |
        Event::CopyBlockMetadata | Event::AtFilePicker | Event::InsertAtRef(_) => dispatch_dialog_event(state, event),
        // Edit
        Event::PendingEdit { .. } | Event::ApproveEdit | Event::RejectEdit => tools::update(state, event),
        // System
        Event::SystemMessage { .. } | Event::TransientMessage { .. } | Event::TransientError { .. } |
        Event::ClearTransient | Event::ShowDiagnostics => handle_system_event(state, event),
        // Session
        Event::ForkSession { .. } | Event::CloneSession | Event::ToggleSessionTree | Event::SessionTreeFilterCycle |
        Event::SessionTreeSelect { .. } => handle_session_event(state, event),
        // Command
        Event::RunLoadCommand { .. } | Event::RunSaveCommand { .. } | Event::RunDeleteCommand { .. } |
        Event::RunImportCommand { .. } | Event::RunExportCommand { .. } | Event::RunSkillCommand { .. } |
        Event::RunLoginCommand { .. } | Event::RunLogoutCommand { .. } | Event::RunNameCommand { .. } |
        Event::RunForkCommand { .. } | Event::RunCompactCommand { .. } | Event::RunPromptCommand { .. } |
        Event::RunThinkingCommand { .. } | Event::RunPaletteCommand { .. } => handle_command_event(state, event),
        // LoginFlow
        Event::Start | Event::SelectProvider { .. } | Event::SubmitKey { .. } | Event::ValidationDone { .. } |
        Event::ValidationFailed { .. } | Event::ModelsFetched { .. } | Event::ToggleModel { .. } | Event::Save |
        Event::Cancel => login_flow::login_flow_event(state, event),
        _ => {}
    }
}

fn dispatch_dialog_event(state: &mut AppState, event: DialogEvent) {
    if is_toggle_dialog_event(&event) {
        dialog::dialog_toggle_event(state, event);
    } else if is_form_dialog_event(&event) {
        dialog::handle_form_dialog(state, event);
    } else if let DialogEvent::InsertAtRef(path) = event {
        dialog::insert_at_ref(state, &path);
    } else if matches!(event, DialogEvent::DialogBack) {
        handle_dialog_back_no_dialog(state);
    }
}

fn handle_dialog_back_no_dialog(state: &mut AppState) {
    if state.open_dialog.is_none() && state.config.vim_mode {
        state.view.vim_nav_mode = true;
        state.view.selected_post = state.current_bottom_post_index();
        state.mark_dirty();
    }
}

fn is_toggle_dialog_event(event: &DialogEvent) -> bool {
    matches!(
        event,
        DialogEvent::ToggleWelcome
            | DialogEvent::ToggleCommandPalette
            | DialogEvent::ToggleSettingsDialog
            | DialogEvent::ToggleModelSelector
            | DialogEvent::AtFilePicker
            | DialogEvent::PaletteFilter(_)
            | DialogEvent::PaletteBackspace
            | DialogEvent::PaletteUp
            | DialogEvent::PaletteDown
            | DialogEvent::PaletteSelect
            | DialogEvent::PaletteClose
            | DialogEvent::ModelSelectorFilter(_)
            | DialogEvent::ModelSelectorBackspace
            | DialogEvent::ModelSelectorUp
            | DialogEvent::ModelSelectorDown
            | DialogEvent::ModelSelectorSelect
            | DialogEvent::ModelSelectorClose
            | DialogEvent::TogglePathCompletion
            | DialogEvent::PathCompletionUp
            | DialogEvent::PathCompletionDown
            | DialogEvent::PathCompletionSelect
            | DialogEvent::PathCompletionClose
            | DialogEvent::ToggleVimMode
            | DialogEvent::OpenAgentsManager
            | DialogEvent::AgentsManagerSetField { .. }
            | DialogEvent::AgentsManagerSave { .. }
            | DialogEvent::AgentsManagerDelete { .. }
            | DialogEvent::ProvidersDialog
            | DialogEvent::ProvidersAdd
            | DialogEvent::ProvidersSelectModel { .. }
            | DialogEvent::ProvidersDisconnect { .. }
            | DialogEvent::ToggleScopedModelsDialog
            | DialogEvent::ScopedModelEnableAll
            | DialogEvent::ScopedModelDisableAll
    )
}

fn is_form_dialog_event(event: &DialogEvent) -> bool {
    matches!(
        event,
        DialogEvent::CommandFormInput(_)
            | DialogEvent::CommandFormBackspace
            | DialogEvent::CommandFormUp
            | DialogEvent::CommandFormDown
            | DialogEvent::CommandFormSubmit
            | DialogEvent::CommandFormClose
    )
}

fn handle_session_event(state: &mut AppState, event: SessionEvent) {
    match event {
        SessionEvent::ForkSession { message_index } => {
            state.fork_session_at(message_index);
            state.view.cached_session_tree_valid = false;
        }
        SessionEvent::CloneSession => {
            state.clone_session();
            state.view.cached_session_tree_valid = false;
        }
        SessionEvent::ToggleSessionTree => {
            state.toggle_session_tree_dialog();
            state.view.cached_session_tree_valid = false;
        }
        SessionEvent::SessionTreeFilterCycle => {
            state.cycle_session_tree_filter();
        }
        SessionEvent::SessionTreeSelect { id } => {
            state.session_tree_select(&id);
        }
        _ => {}
    }
}

fn handle_command_event(state: &mut AppState, event: CommandEvent) {
    use crate::commands::CommandResult;
    match &event {
        CommandEvent::RunLoadCommand { name } => run_load_command(state, name),
        CommandEvent::RunSaveCommand { name } => run_save_command(state, name),
        CommandEvent::RunDeleteCommand { name } => run_delete_command(state, name),
        CommandEvent::RunImportCommand { path } => run_import_command(state, path),
        CommandEvent::RunExportCommand { path } => run_export_command(state, path),
        CommandEvent::RunSkillCommand { name } => run_skill_command(state, name),
        CommandEvent::RunLoginCommand { .. } => {
            dialog::process_command_result(
                state,
                CommandResult::OpenDialog(crate::commands::DialogType::CommandPalette),
            );
        }
        CommandEvent::RunLogoutCommand { provider } => run_logout_command(state, provider),
        CommandEvent::RunNameCommand { name } => {
            crate::commands::dsl::handlers::session::run_name(state, name);
        }
        CommandEvent::RunForkCommand { message_index } => {
            crate::commands::dsl::handlers::session::run_fork(state, message_index);
        }
        CommandEvent::RunCompactCommand { keep, focus } => {
            crate::commands::dsl::handlers::session::run_compact(state, keep, focus);
        }
        CommandEvent::RunPromptCommand { name } => {
            crate::commands::dsl::handlers::system::run_prompt(state, name);
        }
        CommandEvent::RunThinkingCommand { level } => {
            crate::commands::dsl::handlers::model::run_thinking(state, *level);
        }
        CommandEvent::RunPaletteCommand { name, args } => {
            run_palette_command(state, name, args);
        }
        _ => {}
    }
}

fn run_load_command(state: &mut AppState, name: &str) {
    use crate::commands::CommandResult;
    let result = crate::session_replay::load_session(name, state)
        .map(|_| CommandResult::Message(format!("Session '{}' loaded.", name)))
        .unwrap_or_else(|_| {
            CommandResult::Message(format!(
                "Session '{}' not found. Use /sessions to list saved sessions.",
                name
            ))
        });
    dialog::process_command_result(state, result);
}

fn run_save_command(state: &mut AppState, name: &str) {
    use crate::commands::CommandResult;
    let result = crate::session_replay::save_session(name, state)
        .map(|_| CommandResult::Message(format!("Session '{}' saved.", name)))
        .unwrap_or_else(|e| {
            CommandResult::Message(format!("Could not save session: {}", e))
        });
    dialog::process_command_result(state, result);
}

fn run_delete_command(state: &mut AppState, name: &str) {
    use crate::commands::CommandResult;
    let result = crate::session_replay::delete_session(name)
        .map(|_| CommandResult::Message(format!("Session '{}' deleted.", name)))
        .unwrap_or_else(|_| {
            CommandResult::Message(format!(
                "Session '{}' not found. Use /sessions to list saved sessions.",
                name
            ))
        });
    dialog::process_command_result(state, result);
}

fn run_import_command(state: &mut AppState, path: &str) {
    use crate::commands::CommandResult;
    let result = std::fs::read_to_string(path)
        .ok()
        .and_then(|json| serde_json::from_str::<Session>(&json).ok())
        .map(|session| {
            state.session.messages = session.messages;
            state.config.current_provider = session.provider;
            state.config.current_model = session.model;
            state.config.theme_name = session.theme_name;
            state.config.thinking_level = session.thinking_level;
            state.config.read_only = session.read_only;
            state.session.session_display_name = session.display_name.or(Some(session.name));
            state.session.session_created_at = session.created_at;
            state.session.session_updated_at = session.updated_at;
            state.session.session_tree = session.session_tree;
            state.messages_changed();
            CommandResult::Message(format!("Session imported from '{}'", path))
        })
        .unwrap_or_else(|| {
            CommandResult::Message(format!("Could not import session from '{}'", path))
        });
    dialog::process_command_result(state, result);
}

fn run_export_command(state: &mut AppState, path: &str) {
    use crate::commands::CommandResult;
    let session = Session {
        name: state.session.session_display_name.clone().unwrap_or_else(|| "exported".into()),
        display_name: state.session.session_display_name.clone(),
        created_at: state.session.session_created_at,
        updated_at: now(),
        messages: state.session.messages.clone(),
        provider: state.config.current_provider.clone(),
        model: state.config.current_model.clone(),
        theme_name: state.config.theme_name.clone(),
        thinking_level: state.config.thinking_level,
        read_only: state.config.read_only,
        session_tree: state.session.session_tree.clone(),
    };
    let result =
        std::fs::write(path, serde_json::to_string_pretty(&session).unwrap_or_default())
            .map(|_| CommandResult::Message(format!("Session exported to '{}'", path)))
            .unwrap_or_else(|e| CommandResult::Message(format!("Could not export: {}", e)));
    dialog::process_command_result(state, result);
}

fn run_skill_command(state: &mut AppState, name: &str) {
    use crate::commands::CommandResult;
    let result = state
        .skills
        .iter()
        .find(|s| s.name == name)
        .map(|skill| {
            let mut lines = vec![format!("Skill: {}", skill.name)];
            if !skill.description.is_empty() {
                lines.push(format!("Description: {}", skill.description));
            }
            if !skill.context.is_empty() {
                lines.push(format!("Context: {}", skill.context));
            }
            CommandResult::Message(lines.join("\n"))
        })
        .unwrap_or_else(|| {
            CommandResult::Message(format!(
                "Skill '{}' not found. Use /skills to list loaded skills.",
                name
            ))
        });
    dialog::process_command_result(state, result);
}

fn run_logout_command(state: &mut AppState, provider: &str) {
    use crate::commands::CommandResult;
    if provider.is_empty() {
        dialog::process_command_result(
            state,
            CommandResult::OpenDialog(crate::commands::DialogType::CommandPalette),
        );
        return;
    }
    match crate::login_config::remove_provider_config(provider) {
        Ok(()) => {
            if state.config.current_provider == provider {
                let configured = crate::login_config::list_configured_providers();
                if let Some((name, _, models)) = configured.first() {
                    state.config.current_provider = name.clone();
                    state.config.current_model = models.first().cloned().unwrap_or_default();
                } else {
                    state.config.current_provider.clear();
                    state.config.current_model.clear();
                }
            }
            dialog::process_command_result(
                state,
                CommandResult::Message(format!(
                    "Disconnected '{}'. Use /providers to manage providers.",
                    provider
                )),
            );
        }
        Err(e) => dialog::process_command_result(
            state,
            CommandResult::Message(format!("Could not remove provider config: {}", e)),
        ),
    }
}

fn run_palette_command(state: &mut AppState, name: &str, args: &str) {
    use crate::commands::CommandResult;
    let result = if let Some(cmd) = state.registry.get(name) {
        let cmd_name = cmd.name.clone();
        cmd.flow.clone().exec(state, &cmd_name, args)
    } else {
        CommandResult::Message(format!("Unknown command: /{}", name))
    };
    dialog::process_command_result(state, result);
}

fn handle_system_event(state: &mut AppState, event: SystemEvent) {
    match event {
        SystemEvent::SystemMessage { content } => state.add_system_msg(content),
        SystemEvent::TransientMessage { content, level } => state.set_transient(content, level),
        SystemEvent::TransientError { content } => {
            state.set_transient(content, crate::event::TransientLevel::Error)
        }
        SystemEvent::ClearTransient => state.clear_transient(),
        SystemEvent::ShowDiagnostics => state.show_diagnostics(),
        SystemEvent::ToggleReadOnly => state.toggle_read_only(),
        SystemEvent::TrustProject => state.trust_project(),
        SystemEvent::UntrustProject => state.untrust_project(),
        SystemEvent::OpenAgentsManager => {
            state.set_transient(
                "Agents manager not yet implemented".into(),
                crate::event::TransientLevel::Info,
            );
        }
        _ => {}
    }
}
