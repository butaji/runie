//! Application state types.

mod agent;
mod input;
mod session;
mod sidebar;
mod view;

pub use agent::{AgentState, SpeedWindow};
pub use input::{CommandUsage, InputState};
pub use session::{CompletionState, ConfigState, SessionState};
pub use sidebar::{AgentEntry, AgentFocus, SidebarState};
pub use view::ViewState;

/// Per-agent lifecycle status for the sidebar list.
///
/// Alias for the canonical lifecycle enum shared with the orchestrator and
/// subagent actor.
pub type AgentStatus = crate::orchestrator::AgentLifecycleStatus;

/// Alias for subagent lifecycle status.
pub type SubagentStatus = crate::orchestrator::AgentLifecycleStatus;

/// Alias for task lifecycle status.
pub type TaskStatus = crate::orchestrator::AgentLifecycleStatus;
