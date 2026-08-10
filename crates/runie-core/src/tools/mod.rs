//! Tool executor: preflight + sequential/parallel dispatch.

pub mod actor;
pub mod executor;
pub mod path_policy;
pub mod policy;
pub mod registry;
pub mod workspace;

/// Declare the runtime tool set as data at the integration boundary.
#[macro_export]
macro_rules! register_tools {
    ($registry:expr; $($tool:ty),+ $(,)?) => {{
        $( $registry.register(std::sync::Arc::new(<$tool>::default())); )+
    }};
}

pub use actor::{ToolCommand, ToolExecutorActor, ToolOutcome};
pub use executor::{execute_parallel, execute_sequential, ToolExecContext, ToolExecHooks};
pub use policy::{decide as approval_decision, ApprovalDecision, ApprovalMode};
pub use registry::ToolRegistry;
pub use workspace::{BashTool, EditFileTool, GlobTool, GrepTool, ReadFileTool, WriteFileTool};
