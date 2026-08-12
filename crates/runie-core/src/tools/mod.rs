//! Tool executor: preflight + sequential/parallel dispatch.

pub mod actor;
pub mod ask_user;
pub mod background;
pub mod executor;
pub mod git;
mod git_conflict_actor;
pub mod mcp;
pub mod path_policy;
pub mod policy;
pub mod question_broker;
pub mod question_query;
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
pub use executor::{
    execute_parallel, execute_sequential, reduce_scheduler_event, SchedulerEvent,
    SchedulerMetricRow, SchedulerMetrics, ToolExecContext, ToolExecHooks,
};
pub use git::{
    begin_conflict_recovery, classify_conflicts, plan_conflict_recovery, reduce_conflict_recovery,
    GitCommitPrepareRequest, GitCommitPrepareTool, GitCommitTool, GitConflictAction,
    GitConflictEntry, GitConflictRecoveryEvent, GitConflictRecoveryPlan, GitConflictRecoveryState,
    GitConflictRecoveryStatus, GitConflictSummary, GitDiffTool, GitPushRequest, GitPushTool,
    GitRevertRequest, GitRevertTool, GitReviewTool, GitStatusTool, GitWorktreeTool,
};
pub use git_conflict_actor::{GitConflictActor, GitConflictSnapshot};
pub use mcp::{
    McpCallHook, McpCallRequest, McpConnectionStatus, McpHttpActor, McpHttpClient, McpHttpSession,
    McpHttpStatus, McpNotificationActor, McpNotificationSnapshot, McpReconnectDecision,
    McpReconnectPolicy, McpReconnectState, McpServer, McpStatusRow, McpStdioActor, McpStdioClient,
    McpStdioSession, McpStdioStatus, McpStreamEvent, McpTool, McpToolSpec, McpTransport,
    MCP_NOTIFICATION_QUEUE_CAPACITY,
};
pub use policy::{
    decide as approval_decision, decide_registered, record_approval_trace, reduce_approval_mode,
    ApprovalDecision, ApprovalDecisionKind, ApprovalMode, ApprovalModeEvent, ApprovalModeStore,
    ApprovalTrace,
};
pub use question_broker::{
    decode_question_traces, encode_question_traces, question_history_page, question_history_rows,
    question_history_rows_page, PendingUserQuestion, UserQuestionBroker, UserQuestionHistoryPage,
    UserQuestionHistoryRow, UserQuestionTrace,
};
pub use question_query::{parse_question_history_query, QuestionHistoryQuery};
pub use registry::ToolRegistry;
pub use subagent::{
    reduce_subagent_event, SubagentCapability, SubagentEvent, SubagentExecution,
    SubagentLifecycleState, SubagentLifecycleStatus, SubagentRequest, SubagentResourceUsage,
    SubagentResult, SubagentRole, SubagentTool,
};
pub use todo::{
    reduce_todo_event, summarize_todo_plan, TodoActor, TodoEvent, TodoItem, TodoPlanStatus,
    TodoPlanSummary, TodoSnapshot, TodoStatus, TodoWriteTool,
};
pub use web::{
    source_cards, WebSearchHttpClient, WebSearchRequest, WebSearchResponse, WebSearchResult,
    WebSearchTool, WebSearchWireFormat, WebSourceCard,
};
pub use workspace::{BashTool, EditFileTool, GlobTool, GrepTool, ReadFileTool, WriteFileTool};
