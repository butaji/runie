//! Actor implementations for runie
//!
//! These actors handle specific responsibilities in the application:
//! - ToolActor: Executes tools asynchronously
//! - QueueAgent: Manages the message queue and spawns agent when idle
//! - SessionManager: Persists domain events to JSONL
//! - ConfigAgent: Watches config files for changes

mod config_agent;
mod queue_agent;
mod session_manager;
mod tool_actor;
pub mod tools;

pub use config_agent::run_config_agent;
pub use queue_agent::run_queue_agent;
pub use session_manager::{run_session_manager, SessionState};
pub use tool_actor::{run_tool_actor, ToolOutput};
pub use tools::ToolInvocation;
