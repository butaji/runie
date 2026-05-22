#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used)]

pub mod task;
pub mod subagent;
pub mod handoff;
pub mod orchestrator;

pub use task::{Task, TaskPriority};
pub use subagent::{SubagentHandle, SubagentResult, SubagentStatus};
pub use handoff::{HandoffProtocol, HandoffPayload, HandoffError, DefaultHandoff};
pub use orchestrator::{Orchestrator, OrchestratorError, SimpleOrchestrator};
