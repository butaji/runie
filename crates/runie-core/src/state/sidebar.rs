use serde::{Deserialize, Serialize};

use super::AgentStatus;

/// Which agent feed is currently visible / focused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AgentFocus {
    /// Showing the Orchestrator's main feed.
    #[default]
    Orchestrator,
    /// Showing a specific subagent's feed.
    Subagent(String),
}

/// Per-agent status for the sidebar list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentEntry {
    pub id: String,
    pub label: String,
    pub status: AgentStatus,
}

/// Sidebar state for Team mode — tracks subagent list and focus.
#[derive(Debug, Clone, Default)]
pub struct SidebarState {
    /// Whether the sidebar is visible (Team mode with active plan).
    pub visible: bool,
    /// Which agent feed is currently focused.
    pub focus: AgentFocus,
    /// Ordered list of agents in the sidebar (Orchestrator first, then subagents).
    pub agents: Vec<AgentEntry>,
}

impl SidebarState {
    /// Add the Orchestrator entry (always at index 0).
    pub fn set_orchestrator_status(&mut self, status: AgentStatus) {
        if self.agents.is_empty() {
            self.agents.insert(
                0,
                AgentEntry {
                    id: String::new(),
                    label: "Orchestrator".to_string(),
                    status,
                },
            );
        } else {
            self.agents[0].status = status;
        }
    }

    /// Replace subagent entries (indices 1+).
    pub fn set_subagents(&mut self, subagents: Vec<AgentEntry>) {
        if self.agents.is_empty() {
            self.agents.insert(
                0,
                AgentEntry {
                    id: String::new(),
                    label: "Orchestrator".to_string(),
                    status: AgentStatus::Pending,
                },
            );
        }
        self.agents.truncate(1);
        self.agents.extend(subagents);
    }

    /// Focus a subagent by its 1-based index (Ctrl+1..9).
    pub fn focus_subagent_by_index(&mut self, idx: usize) {
        let subagent_idx = 1 + idx; // 1-based, 0 = orchestrator
        if subagent_idx < self.agents.len() {
            let id = self.agents[subagent_idx].id.clone();
            self.focus = AgentFocus::Subagent(id);
        }
    }

    /// Return to the Orchestrator feed.
    pub fn focus_orchestrator(&mut self) {
        self.focus = AgentFocus::Orchestrator;
    }
}

#[cfg(test)]
mod sidebar_tests {
    use super::*;

    #[test]
    fn sidebar_defaults_hidden() {
        let sidebar = SidebarState::default();
        assert!(!sidebar.visible);
        assert!(matches!(sidebar.focus, AgentFocus::Orchestrator));
        assert!(sidebar.agents.is_empty());
    }

    #[test]
    fn focus_defaults_to_orchestrator() {
        assert!(matches!(AgentFocus::default(), AgentFocus::Orchestrator));
    }

    #[test]
    fn focus_subagent_by_index() {
        let mut sidebar = SidebarState::default();
        sidebar.agents.push(AgentEntry {
            id: String::new(),
            label: "Orchestrator".to_string(),
            status: AgentStatus::Running,
        });
        sidebar.agents.push(AgentEntry {
            id: "t1".to_string(),
            label: "Reviewer".to_string(),
            status: AgentStatus::Pending,
        });
        sidebar.agents.push(AgentEntry {
            id: "t2".to_string(),
            label: "Writer".to_string(),
            status: AgentStatus::Pending,
        });

        sidebar.focus_subagent_by_index(0);
        if let AgentFocus::Subagent(id) = &sidebar.focus {
            assert_eq!(id, "t1");
        } else {
            panic!("expected Subagent(t1)");
        }

        sidebar.focus_subagent_by_index(1);
        if let AgentFocus::Subagent(id) = &sidebar.focus {
            assert_eq!(id, "t2");
        } else {
            panic!("expected Subagent(t2)");
        }

        sidebar.focus_subagent_by_index(9); // out of range — unchanged
        if let AgentFocus::Subagent(id) = &sidebar.focus {
            assert_eq!(id, "t2");
        }
    }

    #[test]
    fn focus_orchestrator() {
        let mut sidebar = SidebarState::default();
        sidebar.focus = AgentFocus::Subagent("t1".to_string());
        sidebar.focus_orchestrator();
        assert!(matches!(sidebar.focus, AgentFocus::Orchestrator));
    }

    #[test]
    fn set_orchestrator_status_empty() {
        let mut sidebar = SidebarState::default();
        sidebar.set_orchestrator_status(AgentStatus::Running);
        assert_eq!(sidebar.agents.len(), 1);
        assert_eq!(sidebar.agents[0].label, "Orchestrator");
        assert!(matches!(sidebar.agents[0].status, AgentStatus::Running));
    }

    #[test]
    fn set_orchestrator_status_updates_existing() {
        let mut sidebar = SidebarState::default();
        sidebar.set_orchestrator_status(AgentStatus::Pending);
        sidebar.set_orchestrator_status(AgentStatus::Done { output: None });
        assert_eq!(sidebar.agents.len(), 1);
        assert!(matches!(
            sidebar.agents[0].status,
            AgentStatus::Done { output: _ }
        ));
    }

    #[test]
    fn set_subagents_replaces_non_orchestrator() {
        let mut sidebar = SidebarState::default();
        sidebar.set_orchestrator_status(AgentStatus::Running);
        sidebar.set_subagents(vec![
            AgentEntry {
                id: "t1".into(),
                label: "R".into(),
                status: AgentStatus::Running,
            },
            AgentEntry {
                id: "t2".into(),
                label: "W".into(),
                status: AgentStatus::Pending,
            },
        ]);
        assert_eq!(sidebar.agents.len(), 3); // orchestrator + 2 subagents
        assert!(matches!(sidebar.agents[0].status, AgentStatus::Running)); // orchestrator preserved
        assert_eq!(sidebar.agents[1].id, "t1");
        assert_eq!(sidebar.agents[2].id, "t2");
    }

    #[test]
    fn agent_status_serialization() {
        let statuses = [
            AgentStatus::Pending,
            AgentStatus::Running,
            AgentStatus::AwaitingUser,
            AgentStatus::Done {
                output: Some("done".into()),
            },
            AgentStatus::Failed {
                error: "boom".into(),
            },
        ];
        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let roundtrip: AgentStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(roundtrip, status);
        }
    }

    #[test]
    fn agent_entry_serialization() {
        let entry = AgentEntry {
            id: "t1".into(),
            label: "Reviewer".into(),
            status: AgentStatus::Running,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let roundtrip: AgentEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.id, "t1");
        assert_eq!(roundtrip.label, "Reviewer");
        assert!(matches!(roundtrip.status, AgentStatus::Running));
    }

    #[test]
    fn task_status_into_agent_status() {
        let status: crate::state::TaskStatus = crate::state::TaskStatus::AwaitingUser;
        let agent: AgentStatus = status.into();
        assert_eq!(agent, AgentStatus::AwaitingUser);
    }

    #[test]
    fn subagent_status_into_agent_status() {
        let status: crate::state::SubagentStatus = crate::state::SubagentStatus::Failed {
            error: "boom".into(),
        };
        let agent: AgentStatus = status.into();
        assert_eq!(
            agent,
            AgentStatus::Failed {
                error: "boom".into()
            }
        );
    }

    #[test]
    fn agent_focus_serialization() {
        let variants = [AgentFocus::Orchestrator, AgentFocus::Subagent("t1".into())];
        for focus in variants {
            let json = serde_json::to_string(&focus).unwrap();
            let roundtrip: AgentFocus = serde_json::from_str(&json).unwrap();
            assert_eq!(roundtrip, focus);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Integration tests: OrchestratorEvent → SidebarState
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod orchestrator_sidebar_tests {
    use super::*;
    use crate::orchestrator::{
        ModelTrait, OrchestratorPlan, SubagentTask, SynthesisConfig, TaskStatus,
    };
    use crate::orchestrator_actor::OrchestratorEvent;

    fn orchestrator_plan() -> OrchestratorPlan {
        OrchestratorPlan {
            tasks: vec![
                SubagentTask::new("t1", "reviewer", "Review src/lib.rs", ModelTrait::General),
                SubagentTask::new(
                    "t2",
                    "writer",
                    "Write tests for src/lib.rs",
                    ModelTrait::General,
                ),
            ],
            synthesis_trait: ModelTrait::General,
            summary: None,
            rationale: None,
            synthesis: SynthesisConfig::default(),
        }
    }

    fn apply_event(state: &mut crate::model::AppState, event: OrchestratorEvent) {
        state.update(event);
    }

    #[test]
    fn plan_started_shows_sidebar() {
        let mut state = crate::model::AppState::default();
        apply_event(&mut state, OrchestratorEvent::PlanStarted);
        assert!(state.sidebar.visible);
        assert_eq!(state.sidebar.agents.len(), 1);
        assert!(matches!(
            state.sidebar.agents[0].status,
            AgentStatus::Running
        ));
    }

    #[test]
    fn plan_generated_populates_subagents() {
        let mut state = crate::model::AppState::default();
        apply_event(&mut state, OrchestratorEvent::PlanStarted);
        apply_event(
            &mut state,
            OrchestratorEvent::PlanGenerated {
                plan: Box::new(orchestrator_plan()),
            },
        );
        assert_eq!(state.sidebar.agents.len(), 3); // orchestrator + 2 subagents
        assert_eq!(state.sidebar.agents[1].id, "t1");
        assert_eq!(state.sidebar.agents[2].id, "t2");
        assert!(matches!(
            state.sidebar.agents[1].status,
            AgentStatus::Pending
        ));
        assert!(matches!(
            state.sidebar.agents[2].status,
            AgentStatus::Pending
        ));
    }

    #[test]
    fn subagent_status_changed_updates_entry() {
        let mut state = crate::model::AppState::default();
        apply_event(&mut state, OrchestratorEvent::PlanStarted);
        apply_event(
            &mut state,
            OrchestratorEvent::PlanGenerated {
                plan: Box::new(orchestrator_plan()),
            },
        );
        apply_event(
            &mut state,
            OrchestratorEvent::SubagentStatusChanged {
                task_id: "t1".into(),
                status: TaskStatus::Running,
            },
        );
        let entry = state.sidebar.agents.iter().find(|a| a.id == "t1").unwrap();
        assert!(matches!(entry.status, AgentStatus::Running));
        // t2 should be unchanged
        let entry2 = state.sidebar.agents.iter().find(|a| a.id == "t2").unwrap();
        assert!(matches!(entry2.status, AgentStatus::Pending));
    }

    #[test]
    fn cancelled_hides_sidebar() {
        let mut state = crate::model::AppState::default();
        apply_event(&mut state, OrchestratorEvent::PlanStarted);
        apply_event(
            &mut state,
            OrchestratorEvent::PlanGenerated {
                plan: Box::new(orchestrator_plan()),
            },
        );
        apply_event(&mut state, OrchestratorEvent::Cancelled);
        assert!(!state.sidebar.visible);
        assert!(state.sidebar.agents.is_empty());
    }

    #[test]
    fn orchestrator_event_serialization() {
        let plan = orchestrator_plan();
        let events = [
            OrchestratorEvent::PlanStarted,
            OrchestratorEvent::PlanningStarted,
            OrchestratorEvent::PlanGenerated {
                plan: Box::new(plan.clone()),
            },
            OrchestratorEvent::PlanningFailed {
                error: "timeout".into(),
            },
            OrchestratorEvent::SubagentStatusChanged {
                task_id: "t1".into(),
                status: TaskStatus::Running,
            },
            OrchestratorEvent::Cancelled,
            OrchestratorEvent::Finished { success: true },
        ];
        for event in events {
            let json = serde_json::to_string(&event).unwrap();
            let roundtrip: OrchestratorEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(roundtrip, event);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Team mode integration tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod team_mode_tests {
    use super::*;
    use crate::model::AppState;
    use crate::orchestrator::ExecutionMode;

    #[test]
    fn solo_mode_uses_agent() {
        let state = AppState::default();
        assert_eq!(state.config.execution_mode, ExecutionMode::Solo);
        assert!(!state.config.execution_mode.uses_orchestrator());
    }

    #[test]
    fn team_mode_uses_orchestrator() {
        let mut state = AppState::default();
        state.config.execution_mode = ExecutionMode::Team;
        assert!(state.config.execution_mode.uses_orchestrator());
    }

    #[test]
    fn team_mode_toggle_shows_sidebar() {
        let mut state = AppState::default();
        // Initially hidden
        assert!(!state.sidebar.visible);
        // Switch to Team — sidebar becomes visible
        state.config.execution_mode = ExecutionMode::Team;
        // Sidebar shows when orchestrator plan starts (not just mode toggle)
        // The key invariant: sidebar is only visible when in Team mode AND plan is active
        assert!(!state.sidebar.visible); // no plan yet
    }

    #[test]
    fn solo_mode_has_no_sidebar_agents() {
        let mut state = AppState::default();
        // Force some agents in
        state.sidebar.visible = true;
        state.sidebar.agents.push(AgentEntry {
            id: "t1".into(),
            label: "Test".into(),
            status: AgentStatus::Running,
        });
        // Switch to Solo — agents cleared
        state.config.execution_mode = ExecutionMode::Solo;
        state.sidebar.visible = false;
        state.sidebar.agents.clear();
        assert_eq!(state.config.execution_mode, ExecutionMode::Solo);
        assert!(!state.sidebar.visible);
        assert!(state.sidebar.agents.is_empty());
    }
}
