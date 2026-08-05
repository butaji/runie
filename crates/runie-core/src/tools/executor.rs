//! Sequential and parallel tool dispatch.
//!
//! Both paths emit `ToolExecutionStart` / `ToolExecutionEnd` events through
//! the supplied sink. In parallel mode, completion-order and source-order
//! are separated per the TS README §With Tool Calls.

use std::sync::Arc;

use super::registry::ToolRegistry;
use crate::types::{
    AgentMessage, AgentTool, AgentToolResult, AssistantContent, AssistantMessage,
    BeforeToolCallContext, BeforeToolCallResult, ToolCall, ToolExecutionMode, ToolResultContent,
    ToolResultMessage,
};

/// Hooks applied during tool dispatch. `None` for any field means "use
/// default (allow / no override)".
#[derive(Default, Clone)]
pub struct ToolExecHooks {
    pub before_tool_call:
        Option<Arc<dyn Fn(BeforeToolCallContext) -> BeforeToolCallResult + Send + Sync>>,
    pub after_tool_call:
        Option<Arc<dyn Fn(AfterToolCallInputs) -> crate::types::AfterToolCallResult + Send + Sync>>,
}

#[derive(Debug, Clone)]
pub struct AfterToolCallInputs {
    pub assistant_message: AssistantMessage,
    pub tool_call: ToolCall,
    pub args: serde_json::Value,
    pub result: AgentToolResult,
    pub is_error: bool,
}

#[derive(Clone)]
pub struct ToolExecContext {
    pub assistant_message: AssistantMessage,
    pub registry: Arc<ToolRegistry>,
    pub hooks: ToolExecHooks,
}

/// Result of dispatching a batch.
#[derive(Debug, Clone, Default)]
pub struct DispatchOutcome {
    pub tool_results: Vec<ToolResultMessage>,
    pub all_terminated: bool,
    pub events: Vec<crate::types::AgentEvent>,
}

pub async fn execute_sequential(calls: Vec<ToolCall>, ctx: ToolExecContext) -> DispatchOutcome {
    let mut outcome = DispatchOutcome::default();

    for call in calls {
        let start = crate::types::AgentEvent::ToolExecutionStart {
            tool_call_id: call.id.clone(),
            tool_name: call.name.clone(),
            args: call.arguments.clone(),
        };
        outcome.events.push(start);

        let (result, is_error) = match dispatch_one(&call, &ctx).await {
            Ok(r) => (r, false),
            Err(msg) => (synthetic_error_result(&msg), true),
        };

        let end = crate::types::AgentEvent::ToolExecutionEnd {
            tool_call_id: call.id.clone(),
            tool_name: call.name.clone(),
            result: serde_json::to_value(&result).unwrap_or_default(),
            is_error,
        };
        outcome.events.push(end);

        let tr = ToolResultMessage {
            tool_call_id: call.id.clone(),
            tool_name: call.name.clone(),
            content: result.content.clone(),
            is_error,
            timestamp: 0,
        };
        outcome.tool_results.push(tr);
        if !result.terminate {
            outcome.all_terminated = false;
        }
    }

    outcome.all_terminated = !outcome.tool_results.is_empty()
        && outcome.tool_results.iter().all(|_tr| {
            // Re-fetch terminate from the original result via the events sink.
            // Simpler: re-derive from the events we recorded.
            outcome.events.iter().any(|e| {
                matches!(e, crate::types::AgentEvent::ToolExecutionEnd { tool_call_id, result, .. }
                    if tool_call_id == &_tr.tool_call_id && result.get("terminate").and_then(|v| v.as_bool()).unwrap_or(false))
            })
        });

    outcome
}

pub async fn execute_parallel(calls: Vec<ToolCall>, ctx: ToolExecContext) -> DispatchOutcome {
    // Preflight sequentially (validate args + before_tool_call).
    let mut preflighted: Vec<ToolCall> = Vec::with_capacity(calls.len());
    for call in calls {
        if !validate_args(&call, &ctx) {
            continue;
        }
        preflighted.push(call);
    }

    let total = preflighted.len();
    let mut outcome = DispatchOutcome::default();

    if preflighted.is_empty() {
        return outcome;
    }

    // Emit all start events up front (in source order).
    for call in &preflighted {
        outcome
            .events
            .push(crate::types::AgentEvent::ToolExecutionStart {
                tool_call_id: call.id.clone(),
                tool_name: call.name.clone(),
                args: call.arguments.clone(),
            });
    }

    // Execute concurrently. The completion-order list is built first; we
    // then emit toolResult messages in source order to match the README.
    let ctx_for_exec = ctx.clone();
    let results: Vec<(String, AgentToolResult, bool, Vec<crate::types::AgentEvent>)> =
        futures::future::join_all(preflighted.iter().cloned().map(|call| {
            let ctx = ctx_for_exec.clone();
            async move {
                let mut events = vec![crate::types::AgentEvent::ToolExecutionUpdate {
                    tool_call_id: call.id.clone(),
                    tool_name: call.name.clone(),
                    args: call.arguments.clone(),
                    partial_result: serde_json::json!({"status": "running"}),
                }];
                match dispatch_one(&call, &ctx).await {
                    Ok(r) => {
                        let end = crate::types::AgentEvent::ToolExecutionEnd {
                            tool_call_id: call.id.clone(),
                            tool_name: call.name.clone(),
                            result: serde_json::to_value(&r).unwrap_or_default(),
                            is_error: false,
                        };
                        events.push(end);
                        (call.id, r, false, events)
                    }
                    Err(msg) => {
                        let r = synthetic_error_result(&msg);
                        let end = crate::types::AgentEvent::ToolExecutionEnd {
                            tool_call_id: call.id.clone(),
                            tool_name: call.name.clone(),
                            result: serde_json::to_value(&r).unwrap_or_default(),
                            is_error: true,
                        };
                        events.push(end);
                        (call.id, r, true, events)
                    }
                }
            }
        }))
        .await;

    let _ = total;

    // Emit toolResult messages in source order.
    let mut by_id: std::collections::HashMap<String, (AgentToolResult, bool)> = results
        .iter()
        .map(|(id, r, e, _)| (id.clone(), (r.clone(), *e)))
        .collect();
    for (_, _, _, events) in results {
        outcome.events.extend(events);
    }

    let mut all_terminated = true;
    for call in &preflighted {
        if let Some((r, is_error)) = by_id.remove(&call.id) {
            let tr = ToolResultMessage {
                tool_call_id: call.id.clone(),
                tool_name: call.name.clone(),
                content: r.content.clone(),
                is_error,
                timestamp: 0,
            };
            outcome.tool_results.push(tr);
            if !r.terminate {
                all_terminated = false;
            }
        }
    }

    outcome.all_terminated = all_terminated;
    outcome
}

fn validate_args(call: &ToolCall, ctx: &ToolExecContext) -> bool {
    if let Some(tool) = ctx.registry.lookup(&call.name) {
        if tool.validate_arguments(&call.arguments).is_err() {
            return false;
        }
    }
    true
}

async fn dispatch_one(call: &ToolCall, ctx: &ToolExecContext) -> Result<AgentToolResult, String> {
    let tool = ctx
        .registry
        .lookup(&call.name)
        .ok_or_else(|| format!("tool not found: {}", call.name))?;

    if let Some(ref hook) = ctx.hooks.before_tool_call {
        let decision = hook(BeforeToolCallContext {
            assistant_message: ctx.assistant_message.clone(),
            tool_call: call.clone(),
            args: call.arguments.clone(),
        });
        if decision.block {
            return Err(decision.reason.unwrap_or_else(|| "blocked".into()));
        }
    }

    let mut result = tool
        .execute(&call.id, call.arguments.clone(), None, None)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(ref hook) = ctx.hooks.after_tool_call {
        let override_ = hook(AfterToolCallInputs {
            assistant_message: ctx.assistant_message.clone(),
            tool_call: call.clone(),
            args: call.arguments.clone(),
            result: result.clone(),
            is_error: false,
        });
        if let Some(content) = override_.content {
            result.content = content;
        }
        if let Some(details) = override_.details {
            result.details = details;
        }
        if let Some(t) = override_.terminate {
            result.terminate = t;
        }
    }

    Ok(result)
}

fn synthetic_error_result(reason: &str) -> AgentToolResult {
    AgentToolResult {
        content: vec![ToolResultContent::Text {
            text: reason.to_string(),
        }],
        details: serde_json::Value::Null,
        usage: None,
        added_tool_names: vec![],
        terminate: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AfterToolCallContext, AfterToolCallResult};

    fn dummy_call(id: &str, name: &str) -> ToolCall {
        ToolCall {
            id: id.into(),
            name: name.into(),
            arguments: serde_json::json!({}),
        }
    }

    #[test]
    fn empty_sequential_produces_no_results() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let registry = Arc::new(ToolRegistry::new());
            let ctx = ToolExecContext {
                assistant_message: AssistantMessage {
                    content: vec![],
                    stop_reason: None,
                    model: "test".into(),
                    timestamp: 0,
                },
                registry,
                hooks: ToolExecHooks::default(),
            };
            let outcome = execute_sequential(vec![], ctx).await;
            assert!(outcome.tool_results.is_empty());
            assert!(!outcome.all_terminated);
        });
    }

    #[allow(dead_code)]
    fn _after_ctx_marker(_c: AfterToolCallContext) -> AfterToolCallResult {
        AfterToolCallResult::default()
    }
}
