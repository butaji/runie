//! Tool executor: preflight + sequential/parallel dispatch.

pub mod actor;
pub mod ask_user;
pub mod background;
pub mod executor;
pub mod git;
pub mod mcp;
pub mod path_policy;
pub mod policy;
pub mod question_broker;
pub mod registry;
pub mod subagent;
pub mod todo;
pub mod web;
pub mod workspace;

/// Declare the runtime tool set as data at the integration boundary.
#[macro_export]
macro_rules! register_tools {
    ($registry:expr; $($tool:ty),+ $(,)?) => {{
        $( $registry.register(std::sync::Arc::new(<$tool>::default())); )+
    }};
}

pub use actor::{ToolCommand, ToolExecutorActor, ToolOutcome, ToolPriority};
pub use ask_user::{AskUserQuestionTool, UserQuestionOption, UserQuestionRequest};
pub use background::{
    BackgroundCancelRequest, BackgroundCancelTool, BackgroundJobsTool, BackgroundShellRequest,
    BackgroundShellTool,
};
pub use executor::{execute_parallel, execute_sequential, ToolExecContext, ToolExecHooks};
pub use git::{
    GitCommitPrepareRequest, GitCommitPrepareTool, GitCommitTool, GitDiffTool, GitPushRequest,
    GitPushTool, GitRevertRequest, GitRevertTool, GitReviewTool, GitStatusTool, GitWorktreeTool,
};
pub use mcp::{
    McpCallHook, McpCallRequest, McpHttpClient, McpHttpSession, McpServer, McpStdioClient, McpTool,
    McpToolSpec,
};
pub use policy::{decide as approval_decision, ApprovalDecision, ApprovalMode, ApprovalModeStore};
pub use question_broker::{
    decode_question_traces, encode_question_traces, PendingUserQuestion, UserQuestionBroker,
    UserQuestionTrace,
};
pub use registry::ToolRegistry;
pub use subagent::{
    SubagentCapability, SubagentRequest, SubagentResult, SubagentRole, SubagentTool,
};
pub use todo::{TodoActor, TodoItem, TodoSnapshot, TodoStatus, TodoWriteTool};
pub use web::{
    WebSearchHttpClient, WebSearchRequest, WebSearchResponse, WebSearchResult, WebSearchTool,
};
pub use workspace::{BashTool, EditFileTool, GlobTool, GrepTool, ReadFileTool, WriteFileTool};
