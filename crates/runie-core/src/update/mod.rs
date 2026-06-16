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
            Event::Input(e) => self.handle_input_event(e),
            Event::Agent(e) => dispatch_event(self, Event::Agent(e)),
            Event::Scroll(e) => dispatch_event(self, Event::Scroll(e)),
            Event::Control(e) => dispatch_event(self, Event::Control(e)),
            Event::ModelConfig(e) => dispatch_event(self, Event::ModelConfig(e)),
            Event::Dialog(e) => self.handle_dialog_event(e),
            Event::Edit(e) => dispatch_event(self, Event::Edit(e)),
            Event::System(e) => dispatch_event(self, Event::System(e)),
            Event::Session(e) => dispatch_event(self, Event::Session(e)),
            Event::Command(e) => dispatch_event(self, Event::Command(e)),
            Event::LoginFlow(e) => dispatch_event(self, Event::LoginFlow(e)),
            Event::Sidebar(e) => self.handle_sidebar_event(e),
            Event::Orchestrator(e) => dispatch_event(self, Event::Orchestrator(e)),
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
                self.orchestrator_state = to;
            }
            _ => {}
        }
        self.mark_dirty();
    }

    fn handle_input_event(&mut self, event: crate::event::InputEvent) {
        if self.try_handle_dialog_event_input(&event) {
            return;
        }
        if self.try_handle_vim_dialog_back_input(&event) {
            return;
        }
        if self.try_handle_vim_nav_event_input(&event) {
            return;
        }
        dispatch_event(self, Event::Input(event));
    }

    fn handle_dialog_event(&mut self, event: DialogEvent) {
        if is_login_flow_dialog_event(&event) || is_providers_dialog_event(&event) {
            dispatch_event(self, Event::Dialog(event));
            return;
        }
        if self.try_handle_dialog_event_dialog(&event) {
            return;
        }
        dispatch_event(self, Event::Dialog(event));
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
                dialog::update_dialog(self, Event::Input(event.clone()));
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
        let Some(handled) = self.handle_vim_nav_event(&Event::Input(event.clone())) else {
            return false;
        };
        !handled
    }

    fn try_handle_dialog_event_dialog(&mut self, event: &DialogEvent) -> bool {
        if self.open_dialog.is_none() {
            return false;
        }
        if self.login_flow.is_some() && matches!(event, DialogEvent::ProvidersAdd) {
            return false;
        }
        dialog::update_dialog(self, Event::Dialog(event.clone()));
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

// ── Central dispatcher (formerly in dispatch.rs) ─────────────────────────────────

/// Dispatch an event when no dialog is open and no special early-return
/// handler has consumed it.
fn dispatch_event(state: &mut AppState, event: Event) {
    match event {
        Event::Input(e) => input::input_event(state, e),
        Event::Agent(e) => agent::agent_event(state, e),
        Event::Scroll(e) => input::scroll_event(state, e),
        Event::Control(e) => system::control_event(state, e),
        Event::ModelConfig(e) => agent::model_config_event(state, e),
        Event::Dialog(e) => handle_dialog_event(state, e),
        Event::Edit(e) => tools::update(state, e),
        Event::Session(e) => handle_session_event(state, e),
        Event::Command(e) => handle_command_event(state, e),
        Event::System(e) => handle_system_event(state, e),
        Event::LoginFlow(e) => login_flow::login_flow_event(state, e),
        // Sidebar events are handled directly in AppState::update
        Event::Sidebar(_) => {}
        // Orchestrator events drive sidebar state
        Event::Orchestrator(e) => state.handle_orchestrator_event(e),
    }
}

fn handle_dialog_event(state: &mut AppState, event: DialogEvent) {
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
    }
}

fn run_load_command(state: &mut AppState, name: &str) {
    use crate::commands::CommandResult;
    let result = crate::session::load(name)
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
            CommandResult::Message(format!("Session '{}' loaded.", name))
        })
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
    let session = Session {
        name: name.to_string(),
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
    let result = crate::session::save(name, &session)
        .map(|_| CommandResult::Message(format!("Session '{}' saved.", name)))
        .unwrap_or_else(|e| {
            CommandResult::Message(format!("Could not save session: {}", e))
        });
    dialog::process_command_result(state, result);
}

fn run_delete_command(state: &mut AppState, name: &str) {
    use crate::commands::CommandResult;
    let result = crate::session::delete(name)
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
    }
}
