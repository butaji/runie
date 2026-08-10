use super::AfterToolCallInputs;
use crate::tools::policy::ApprovalMode;
use crate::types::{BeforeToolCallContext, BeforeToolCallResult};
use std::{future::Future, pin::Pin, sync::Arc};

#[derive(Default, Clone)]
pub struct ToolExecHooks {
    pub approval_mode: ApprovalMode,
    pub before_tool_call: Option<BeforeToolCallHook>,
    pub after_tool_call: Option<AfterToolCallHook>,
    pub ask_user_question: Option<AskUserQuestionHook>,
    pub subagent: Option<SubagentHook>,
    pub web_search: Option<WebSearchHook>,
    pub background_shell: Option<BackgroundShellHook>,
    pub background_jobs: Option<BackgroundJobsHook>,
    pub background_cancel: Option<BackgroundCancelHook>,
    pub todo_write: Option<TodoWriteHook>,
}

pub type BeforeToolCallHook = Arc<
    dyn Fn(BeforeToolCallContext) -> Pin<Box<dyn Future<Output = BeforeToolCallResult> + Send>>
        + Send
        + Sync,
>;
pub type AfterToolCallHook = Arc<
    dyn Fn(
            AfterToolCallInputs,
        ) -> Pin<Box<dyn Future<Output = crate::types::AfterToolCallResult> + Send>>
        + Send
        + Sync,
>;
pub type AskUserQuestionHook = Arc<
    dyn Fn(
            crate::tools::UserQuestionRequest,
        ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, String>> + Send>>
        + Send
        + Sync,
>;
pub type SubagentHook = Arc<
    dyn Fn(
            crate::tools::SubagentRequest,
        ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, String>> + Send>>
        + Send
        + Sync,
>;
pub type WebSearchHook = Arc<
    dyn Fn(
            crate::tools::WebSearchRequest,
        ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, String>> + Send>>
        + Send
        + Sync,
>;
pub type BackgroundShellHook = Arc<
    dyn Fn(
            crate::tools::BackgroundShellRequest,
        ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, String>> + Send>>
        + Send
        + Sync,
>;
pub type BackgroundJobsHook = Arc<
    dyn Fn() -> Pin<Box<dyn Future<Output = Result<serde_json::Value, String>> + Send>>
        + Send
        + Sync,
>;
pub type BackgroundCancelHook = Arc<
    dyn Fn(
            crate::tools::BackgroundCancelRequest,
        ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, String>> + Send>>
        + Send
        + Sync,
>;
pub type TodoWriteHook = Arc<
    dyn Fn(
            crate::tools::TodoSnapshot,
        ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, String>> + Send>>
        + Send
        + Sync,
>;
