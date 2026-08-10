//! Tool executor: preflight + sequential/parallel dispatch.

pub mod actor;
pub mod executor;
pub mod policy;
pub mod registry;
pub mod workspace;

pub use actor::{ToolCommand, ToolExecutorActor, ToolOutcome};
pub use executor::{execute_parallel, execute_sequential, ToolExecContext, ToolExecHooks};
pub use policy::{decide as approval_decision, ApprovalDecision, ApprovalMode};
pub use registry::ToolRegistry;
pub use workspace::{BashTool, EditFileTool, GlobTool, GrepTool, ReadFileTool, WriteFileTool};
