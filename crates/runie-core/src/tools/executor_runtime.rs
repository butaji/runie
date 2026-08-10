use super::*;
pub(super) async fn await_tool_result<F>(
    tool_future: F,
    abort: Option<tokio::sync::watch::Receiver<bool>>,
    signal: tokio_util::sync::CancellationToken,
) -> Result<AgentToolResult, String>
where
    F: std::future::Future,
    F::Output: IntoToolResult,
{
    tokio::select! {
        result = tool_future => result.into_tool_result(),
        aborted = wait_for_tool_abort(abort) => {
            signal.cancel();
            if aborted { Err("aborted".into()) } else { Err("tool execution aborted".into()) }
        }
    }
}

pub(super) trait IntoToolResult {
    fn into_tool_result(self) -> Result<AgentToolResult, String>;
}

impl<E: std::fmt::Display> IntoToolResult for Result<AgentToolResult, E> {
    fn into_tool_result(self) -> Result<AgentToolResult, String> {
        self.map_err(|error| error.to_string())
    }
}

pub(super) fn tool_update_is_live(settled: &AtomicBool) -> bool {
    !settled.load(Ordering::Acquire)
}

pub(super) async fn wait_for_tool_abort(
    mut abort: Option<tokio::sync::watch::Receiver<bool>>,
) -> bool {
    let Some(ref mut receiver) = abort else {
        return std::future::pending::<bool>().await;
    };
    loop {
        if *receiver.borrow() {
            return true;
        }
        if receiver.changed().await.is_err() {
            return false;
        }
    }
}

pub(super) fn take_updates(ctx: &ToolExecContext) -> Vec<crate::types::AgentEvent> {
    ctx.updates
        .as_ref()
        .map(|updates| std::mem::take(&mut *updates.lock().expect("tool update event lock")))
        .unwrap_or_default()
}

pub(super) async fn apply_after_tool_hook(
    call: &ToolCall,
    ctx: &ToolExecContext,
    mut result: AgentToolResult,
    signal: tokio_util::sync::CancellationToken,
    is_error: bool,
) -> (AgentToolResult, bool) {
    let Some(hook) = &ctx.hooks.after_tool_call else {
        return (result, is_error);
    };
    let override_ = hook(AfterToolCallInputs {
        assistant_message: ctx.assistant_message.clone(),
        tool_call: call.clone(),
        args: call.arguments.clone(),
        result: result.clone(),
        is_error,
        context: ctx.context.clone(),
        signal,
    })
    .await;
    if let Some(content) = override_.content {
        result.content = content;
    }
    if let Some(details) = override_.details {
        result.details = details;
    }
    if let Some(t) = override_.terminate {
        result.terminate = t;
    }
    if let Some(usage) = override_.usage {
        result.usage = Some(usage);
    }
    (result, override_.is_error.unwrap_or(is_error))
}

pub(super) fn synthetic_error_result(reason: &str) -> AgentToolResult {
    AgentToolResult {
        content: vec![ToolResultContent::Text {
            text: reason.to_string(),
        }],
        details: serde_json::json!({}),
        usage: None,
        added_tool_names: vec![],
        terminate: false,
    }
}

#[cfg(test)]
#[path = "executor_tests.rs"]
mod tests;
