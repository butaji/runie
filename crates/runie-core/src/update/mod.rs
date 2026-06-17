//! Event update handlers — merged dispatcher (formerly split between mod.rs and dispatch.rs).

use crate::event::DialogEvent;
use crate::model::AppState;
use crate::Event;

// Re-export for backward compatibility
pub use crate::tool_markers::has_tool_markers as content_has_tool_markers;
pub use crate::tool_markers::strip_tool_markers;

mod agent;
pub(crate) mod command;
pub(crate) mod dialog;
pub(crate) mod dialog_input;
mod dispatch;
pub(crate) mod input;
pub(crate) mod login_flow;
mod session;
mod system;
mod tools;

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
        if dispatch::is_dialog_event(&event) {
            self.handle_dialog_event(&event);
        } else {
            dispatch::dispatch_event(self, event);
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
        use crate::state::AgentEntry;

        match event {
            OrchestratorEvent::PlanStarted => {
                self.sidebar.visible = true;
                self.sidebar
                    .set_orchestrator_status(crate::state::AgentStatus::Running);
                self.sidebar.agents.truncate(1); // clear old subagents
            }
            OrchestratorEvent::PlanningStarted => {
                self.sidebar
                    .set_orchestrator_status(crate::state::AgentStatus::Running);
            }
            OrchestratorEvent::PlanGenerated { plan } => {
                let entries: Vec<AgentEntry> = plan
                    .tasks
                    .iter()
                    .map(|t| AgentEntry {
                        id: t.id.clone(),
                        label: t.task_description.chars().take(20).collect(),
                        status: t.status.clone(),
                    })
                    .collect();
                self.sidebar.set_subagents(entries);
            }
            OrchestratorEvent::SubagentStatusChanged { task_id, status } => {
                if let Some(entry) = self.sidebar.agents.iter_mut().find(|a| a.id == task_id) {
                    entry.status = status;
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
            dispatch::dispatch_event(self, event.clone());
            return;
        }
        if self.try_handle_dialog_event_dialog(event) {
            return;
        }
        dispatch::dispatch_event(self, event.clone());
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
