use serde::{Deserialize, Serialize};

/// Lifecycle state of an agent or subagent task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentLifecycleStatus {
    /// Task is queued but not yet started.
    Pending,
    /// Task is currently executing.
    Running,
    /// Task is waiting for user input/approval before continuing.
    AwaitingUser,
    /// Task completed successfully, optionally with output.
    Done { output: Option<String> },
    /// Task failed with an error message.
    Failed { error: String },
}

impl AgentLifecycleStatus {
    /// Whether a status transition from `self` to `next` is valid.
    pub fn can_transition_to(&self, next: AgentLifecycleStatus) -> bool {
        use AgentLifecycleStatus::*;
        match (self, &next) {
            // Pending can start running
            (Pending, Running) => true,
            // Running can await user, complete, or fail
            (Running, AwaitingUser) | (Running, Done { .. }) | (Running, Failed { .. }) => true,
            // AwaitingUser can resume running or fail
            (AwaitingUser, Running) | (AwaitingUser, Failed { .. }) => true,
            // Done and Failed are terminal — no transitions
            (Done { .. }, _) | (Failed { .. }, _) => false,
            // Same state is always allowed
            (a, b) if a == b => true,
            // All other transitions are invalid
            _ => false,
        }
    }

    /// Human-readable label for display.
    pub fn label(&self) -> &'static str {
        match self {
            AgentLifecycleStatus::Pending => "pending",
            AgentLifecycleStatus::Running => "running",
            AgentLifecycleStatus::AwaitingUser => "awaiting",
            AgentLifecycleStatus::Done { .. } => "done",
            AgentLifecycleStatus::Failed { .. } => "failed",
        }
    }

    /// Whether this status is terminal (no further transitions).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            AgentLifecycleStatus::Done { .. } | AgentLifecycleStatus::Failed { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_lifecycle_status_variants() {
        let _ = AgentLifecycleStatus::Pending;
        let _ = AgentLifecycleStatus::Running;
        let _ = AgentLifecycleStatus::AwaitingUser;
        let _ = AgentLifecycleStatus::Done { output: None };
        let _ = AgentLifecycleStatus::Failed {
            error: "boom".into(),
        };
    }

    #[test]
    fn agent_lifecycle_status_serialization() {
        let status = AgentLifecycleStatus::Done {
            output: Some("result".into()),
        };
        let json = serde_json::to_string(&status).unwrap();
        let roundtrip: AgentLifecycleStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, status);
    }

    #[test]
    fn task_status_alias_converts() {
        // TaskStatus is a type alias for AgentLifecycleStatus; ensure it can be
        // constructed and compared through the alias.
        let ts: crate::orchestrator::TaskStatus = crate::orchestrator::TaskStatus::Pending;
        assert_eq!(ts, AgentLifecycleStatus::Pending);
    }

    // ── TaskStatus transitions ──────────────────────────────────────────────

    #[test]
    fn task_status_transitions_valid() {
        assert!(
            crate::orchestrator::TaskStatus::Pending
                .can_transition_to(crate::orchestrator::TaskStatus::Running),
            "Pending → Running must be valid"
        );
        assert!(
            crate::orchestrator::TaskStatus::Running
                .can_transition_to(crate::orchestrator::TaskStatus::Done { output: None }),
            "Running → Done must be valid"
        );
        assert!(
            crate::orchestrator::TaskStatus::Running
                .can_transition_to(crate::orchestrator::TaskStatus::AwaitingUser),
            "Running → AwaitingUser must be valid"
        );
        assert!(
            crate::orchestrator::TaskStatus::Running.can_transition_to(
                crate::orchestrator::TaskStatus::Failed {
                    error: "err".into()
                }
            ),
            "Running → Failed must be valid"
        );
        assert!(
            crate::orchestrator::TaskStatus::AwaitingUser
                .can_transition_to(crate::orchestrator::TaskStatus::Running),
            "AwaitingUser → Running must be valid"
        );
    }

    #[test]
    fn task_status_transitions_invalid() {
        assert!(
            !crate::orchestrator::TaskStatus::Done { output: None }
                .can_transition_to(crate::orchestrator::TaskStatus::Pending),
            "Done → Pending must be invalid"
        );
        assert!(
            !crate::orchestrator::TaskStatus::Failed {
                error: "err".into()
            }
            .can_transition_to(crate::orchestrator::TaskStatus::Running),
            "Failed → Running must be invalid"
        );
        assert!(
            !crate::orchestrator::TaskStatus::Pending
                .can_transition_to(crate::orchestrator::TaskStatus::Done { output: None }),
            "Pending → Done must be invalid (must go through Running)"
        );
        assert!(
            !crate::orchestrator::TaskStatus::Pending
                .can_transition_to(crate::orchestrator::TaskStatus::AwaitingUser),
            "Pending → AwaitingUser must be invalid (must go through Running)"
        );
    }

    #[test]
    fn task_status_is_terminal() {
        assert!(crate::orchestrator::TaskStatus::Done { output: None }.is_terminal());
        assert!(crate::orchestrator::TaskStatus::Failed {
            error: "err".into()
        }
        .is_terminal());
        assert!(!crate::orchestrator::TaskStatus::Pending.is_terminal());
        assert!(!crate::orchestrator::TaskStatus::Running.is_terminal());
        assert!(!crate::orchestrator::TaskStatus::AwaitingUser.is_terminal());
    }
}
