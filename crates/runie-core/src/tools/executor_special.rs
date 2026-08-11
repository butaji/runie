use super::{ToolCall, ToolExecContext};
use crate::types::AgentToolResult;

pub(super) async fn execute_question(
    call: &ToolCall,
    ctx: &ToolExecContext,
) -> Result<AgentToolResult, String> {
    let Some(hook) = &ctx.hooks.ask_user_question else {
        return Err("ask_user_question requires an interactive question hook".into());
    };
    let request = serde_json::from_value(call.arguments.clone())
        .map_err(|error| format!("invalid question: {error}"))?;
    Ok(crate::tools::ask_user::answer_result(hook(request).await?))
}

pub(super) async fn execute_web_search(
    call: &ToolCall,
    ctx: &ToolExecContext,
) -> Result<AgentToolResult, String> {
    let Some(hook) = &ctx.hooks.web_search else {
        return Err("web_search requires an owning web search hook".into());
    };
    let request = serde_json::from_value(call.arguments.clone())
        .map_err(|error| format!("invalid web search request: {error}"))?;
    Ok(crate::tools::web::result(hook(request).await?))
}
