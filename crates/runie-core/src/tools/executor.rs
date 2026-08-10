//! Sequential and parallel tool dispatch.
use super::{
    policy::{decide, ApprovalDecision},
    registry::ToolRegistry,
};
use crate::types::{
    AgentContext, AgentToolResult, AssistantMessage, BeforeToolCallContext, BeforeToolCallResult,
    ToolCall, ToolResultContent, ToolResultMessage,
};
use futures::StreamExt;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
#[path = "executor_hooks.rs"]
mod executor_hooks;
pub use executor_hooks::*;

#[derive(Clone)]
pub struct AfterToolCallInputs {
    pub assistant_message: AssistantMessage,
    pub tool_call: ToolCall,
    pub args: serde_json::Value,
    pub result: AgentToolResult,
    pub is_error: bool,
    pub context: crate::types::AgentContext,
    pub signal: tokio_util::sync::CancellationToken,
}

#[derive(Clone)]
pub struct ToolExecContext {
    pub assistant_message: AssistantMessage,
    pub context: AgentContext,
    pub abort: Option<tokio::sync::watch::Receiver<bool>>,
    pub registry: Arc<ToolRegistry>,
    pub hooks: ToolExecHooks,
    pub bus: Option<crate::events::EventBus>,
    pub updates: Option<Arc<std::sync::Mutex<Vec<crate::types::AgentEvent>>>>,
    pub tool_result_timestamp: i64,
}

#[derive(Debug, Clone, Default)]
pub struct DispatchOutcome {
    pub tool_results: Vec<ToolResultMessage>,
    pub all_terminated: bool,
    pub events: Vec<crate::types::AgentEvent>,
}

pub async fn execute_sequential(calls: Vec<ToolCall>, ctx: ToolExecContext) -> DispatchOutcome {
    let mut outcome = DispatchOutcome::default();

    for call in calls {
        outcome.events.push(tool_start(&call));
        let (result, is_error) = match dispatch_one(&call, &ctx).await {
            Ok(result) => result,
            Err(msg) => (synthetic_error_result(&msg), true),
        };
        outcome.events.extend(take_updates(&ctx));
        outcome.events.push(tool_end(&call, &result, is_error));
        outcome.tool_results.push(tool_result_message(
            &call,
            &result,
            is_error,
            ctx.tool_result_timestamp,
        ));
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

fn tool_start(call: &ToolCall) -> crate::types::AgentEvent {
    crate::types::AgentEvent::ToolExecutionStart {
        tool_call_id: call.id.clone(),
        tool_name: call.name.clone(),
        args: call.arguments.clone(),
    }
}

fn tool_end(call: &ToolCall, result: &AgentToolResult, is_error: bool) -> crate::types::AgentEvent {
    crate::types::AgentEvent::ToolExecutionEnd {
        tool_call_id: call.id.clone(),
        tool_name: call.name.clone(),
        result: serde_json::to_value(result).unwrap_or_default(),
        is_error,
    }
}

fn tool_result_message(
    call: &ToolCall,
    result: &AgentToolResult,
    is_error: bool,
    timestamp: i64,
) -> ToolResultMessage {
    ToolResultMessage {
        tool_call_id: call.id.clone(),
        tool_name: call.name.clone(),
        content: result.content.clone(),
        details: result.details.clone(),
        usage: result.usage.clone(),
        added_tool_names: result.added_tool_names.clone(),
        is_error,
        timestamp,
    }
}

pub async fn execute_parallel(calls: Vec<ToolCall>, ctx: ToolExecContext) -> DispatchOutcome {
    let batches = resource_batches(calls, &ctx);
    if batches.len() > 1 {
        let mut combined = DispatchOutcome {
            all_terminated: true,
            ..DispatchOutcome::default()
        };
        for batch in batches {
            let outcome = execute_parallel_batch(batch, &ctx).await;
            combined.all_terminated &= outcome.all_terminated;
            combined.tool_results.extend(outcome.tool_results);
            combined.events.extend(outcome.events);
        }
        return combined;
    }
    execute_parallel_batch(batches.into_iter().next().unwrap_or_default(), &ctx).await
}

async fn execute_parallel_batch(calls: Vec<ToolCall>, ctx: &ToolExecContext) -> DispatchOutcome {
    let (preflighted, mut outcome, had_invalid) = preflight_calls(calls, &ctx);

    if preflighted.is_empty() {
        outcome.all_terminated = !had_invalid;
        return outcome;
    }

    outcome.events.extend(preflighted.iter().map(tool_start));
    let (completion_events, mut by_id) = run_parallel_calls(&preflighted, &ctx).await;
    outcome.events.extend(completion_events);

    // Emit toolResult messages in source order.
    let mut all_terminated = !by_id.is_empty();
    for call in &preflighted {
        if let Some((name, r, is_error)) = by_id.remove(&call.id) {
            outcome.tool_results.push(tool_result_message_named(
                call,
                name,
                &r,
                is_error,
                ctx.tool_result_timestamp,
            ));
            if !r.terminate {
                all_terminated = false;
            }
        }
    }

    outcome.all_terminated = !had_invalid && all_terminated;
    outcome
}

async fn run_parallel_calls(
    calls: &[ToolCall],
    ctx: &ToolExecContext,
) -> (
    Vec<crate::types::AgentEvent>,
    std::collections::HashMap<String, (String, AgentToolResult, bool)>,
) {
    let mut running = futures::stream::FuturesUnordered::new();
    for call in calls.iter().cloned() {
        let ctx = ctx.clone();
        running.push(async move {
            let mut events = Vec::new();
            let (result, is_error) = dispatch_result(&call, &ctx).await;
            events.extend(take_updates(&ctx));
            events.push(tool_end(&call, &result, is_error));
            (call.id.clone(), call.name.clone(), result, is_error, events)
        });
    }
    let mut events = Vec::new();
    let mut by_id = std::collections::HashMap::new();
    while let Some((id, name, result, is_error, completion)) = running.next().await {
        events.extend(completion);
        by_id.insert(id, (name, result, is_error));
    }
    (events, by_id)
}

fn preflight_calls(
    calls: Vec<ToolCall>,
    ctx: &ToolExecContext,
) -> (Vec<ToolCall>, DispatchOutcome, bool) {
    let mut valid = Vec::with_capacity(calls.len());
    let mut outcome = DispatchOutcome::default();
    let mut had_invalid = false;
    for call in calls {
        match prepare_and_validate(&call, ctx) {
            Ok(prepared) => valid.push(prepared),
            Err(message) => {
                append_failed_call(&mut outcome, &call, &message, ctx.tool_result_timestamp);
                had_invalid = true;
            }
        }
    }
    (valid, outcome, had_invalid)
}

fn append_failed_call(
    outcome: &mut DispatchOutcome,
    call: &ToolCall,
    message: &str,
    timestamp: i64,
) {
    let result = synthetic_error_result(message);
    outcome.events.push(tool_start(call));
    outcome.events.push(tool_end(call, &result, true));
    outcome
        .tool_results
        .push(tool_result_message(call, &result, true, timestamp));
}

async fn dispatch_result(call: &ToolCall, ctx: &ToolExecContext) -> (AgentToolResult, bool) {
    match dispatch_prepared(call, ctx).await {
        Ok(result) => result,
        Err(message) => (synthetic_error_result(&message), true),
    }
}

fn tool_result_message_named(
    call: &ToolCall,
    name: String,
    result: &AgentToolResult,
    is_error: bool,
    timestamp: i64,
) -> ToolResultMessage {
    let mut message = tool_result_message(call, result, is_error, timestamp);
    message.tool_name = name;
    message
}

/// Apply `prepareArguments` (pi agent-loop.ts:586) and validate. Returns the
/// prepared call (args possibly replaced) or a pi-formatted validation error.
fn prepare_and_validate(call: &ToolCall, ctx: &ToolExecContext) -> Result<ToolCall, String> {
    if ctx.registry.lookup(&call.name).is_none() {
        return Err(format!("Tool {} not found", call.name));
    }
    let mut prepared = prepared_call(call, ctx);
    if let Some(tool) = ctx.registry.lookup(&prepared.name) {
        if let Some(schema) = tool.parameters() {
            let received_arguments = prepared.arguments.clone();
            prepared.arguments = coerce_json_schema(&schema, prepared.arguments).map_err(|e| {
                format!(
                    "Validation failed for tool \"{}\":\n  - {}\n\nReceived arguments:\n{}",
                    prepared.name,
                    e,
                    serde_json::to_string_pretty(&received_arguments).unwrap_or_default()
                )
            })?;
        }
        if let Err(e) = tool.validate_arguments(&prepared.arguments) {
            return Err(format!(
                "Validation failed for tool \"{}\":\n{e}\n\nReceived arguments:\n{}",
                prepared.name,
                serde_json::to_string_pretty(&prepared.arguments).unwrap_or_default()
            ));
        }
    }
    Ok(prepared)
}

#[path = "executor_schema.rs"]
mod executor_schema;
use executor_schema::*;
#[path = "executor_runtime.rs"]
mod executor_runtime;
use executor_runtime::*;
fn prepared_call(call: &ToolCall, ctx: &ToolExecContext) -> ToolCall {
    if let Some(tool) = ctx.registry.lookup(&call.name) {
        if let Some(new_args) = tool.prepare_arguments(&call.arguments) {
            if new_args != call.arguments {
                return ToolCall {
                    arguments: new_args,
                    ..call.clone()
                };
            }
        }
    }
    call.clone()
}

async fn dispatch_one(
    call: &ToolCall,
    ctx: &ToolExecContext,
) -> Result<(AgentToolResult, bool), String> {
    // Prepare + validate args (pi prepareToolCallArguments + validateToolArguments).
    let prepared = prepare_and_validate(call, ctx)?;
    dispatch_prepared(&prepared, ctx).await
}

/// Execute an already-prepared + validated call (pi prepareToolCall ->
/// executePreparedToolCall -> finalizeExecutedToolCall).
async fn dispatch_prepared(
    call: &ToolCall,
    ctx: &ToolExecContext,
) -> Result<(AgentToolResult, bool), String> {
    let tool = ctx
        .registry
        .lookup(&call.name)
        .ok_or_else(|| format!("tool not found: {}", call.name))?;
    let signal = tokio_util::sync::CancellationToken::new();
    if run_abort_requested(ctx) {
        signal.cancel();
    }

    run_before_tool_gate(call, ctx, signal.clone()).await?;

    let (result, is_error) = match execute_tool(tool, call, ctx, signal.clone()).await {
        Ok(result) => (result, false),
        Err(reason) => (synthetic_error_result(&reason), true),
    };

    Ok(apply_after_tool_hook(call, ctx, result, signal, is_error).await)
}

async fn run_before_tool_gate(
    call: &ToolCall,
    ctx: &ToolExecContext,
    signal: tokio_util::sync::CancellationToken,
) -> Result<(), String> {
    if let ApprovalDecision::Ask { reason } = decide(ctx.hooks.approval_mode, &call.name) {
        let interactive = ctx.hooks.before_tool_call.is_some();
        if !interactive {
            return Err(format!("Approval required for {}: {reason}", call.name));
        }
    }
    let Some(ref hook) = ctx.hooks.before_tool_call else {
        return Ok(());
    };
    let decision = run_before_tool_hook(
        hook,
        BeforeToolCallContext {
            assistant_message: ctx.assistant_message.clone(),
            tool_call: call.clone(),
            args: call.arguments.clone(),
            context: ctx.context.clone(),
            signal: signal.clone(),
        },
        signal.clone(),
        ctx.abort.clone(),
    )
    .await;
    if signal.is_cancelled() || run_abort_requested(ctx) {
        signal.cancel();
        return Err("Operation aborted".into());
    }
    if decision.block {
        return Err(decision
            .reason
            .unwrap_or_else(|| "Tool execution was blocked".into()));
    }
    Ok(())
}

fn run_abort_requested(ctx: &ToolExecContext) -> bool {
    ctx.abort
        .as_ref()
        .is_some_and(|receiver| *receiver.borrow())
}

async fn run_before_tool_hook(
    hook: &BeforeToolCallHook,
    input: BeforeToolCallContext,
    signal: tokio_util::sync::CancellationToken,
    mut abort: Option<tokio::sync::watch::Receiver<bool>>,
) -> BeforeToolCallResult {
    let hook_future = hook(input);
    tokio::pin!(hook_future);
    loop {
        let Some(receiver) = abort.as_mut() else {
            return hook_future.await;
        };
        tokio::select! {
            decision = &mut hook_future => return decision,
            changed = receiver.changed() => {
                if changed.is_err() {
                    abort = None;
                } else if *receiver.borrow() {
                    signal.cancel();
                }
            }
        }
    }
}

async fn execute_tool(
    tool: Arc<dyn crate::types::AgentTool>,
    call: &ToolCall,
    ctx: &ToolExecContext,
    signal: tokio_util::sync::CancellationToken,
) -> Result<AgentToolResult, String> {
    if call.name == "subagent" {
        return execute_subagent(call, ctx).await;
    }
    if call.name == "ask_user_question" {
        return execute_question(call, ctx).await;
    }
    if call.name == "web_search" {
        return execute_web_search(call, ctx).await;
    }
    if call.name == "background_bash" {
        return crate::tools::background::execute_shell(call, ctx).await;
    }
    if call.name == "background_jobs" {
        return crate::tools::background::execute_jobs(ctx).await;
    }
    if call.name == "background_cancel" {
        return crate::tools::background::execute_cancel(call, ctx).await;
    }
    if call.name == "todo_write" {
        return crate::tools::todo::execute_write(call, ctx).await;
    }
    let settled = Arc::new(AtomicBool::new(false));
    let tool_future = tool.execute(
        &call.id,
        call.arguments.clone(),
        Some(signal.clone()),
        Some(tool_update_callback(call, ctx, settled.clone())),
    );
    let result = await_tool_result(tool_future, ctx.abort.clone(), signal.clone()).await;
    settled.store(true, Ordering::Release);
    result
}
async fn execute_subagent(
    call: &ToolCall,
    ctx: &ToolExecContext,
) -> Result<AgentToolResult, String> {
    let Some(hook) = &ctx.hooks.subagent else {
        return Err("subagent requires an owning subagent hook".into());
    };
    let request: crate::tools::SubagentRequest = serde_json::from_value(call.arguments.clone())
        .map_err(|error| format!("invalid subagent request: {error}"))?;
    let output = hook(request.clone()).await?;
    Ok(crate::tools::subagent::result(&request, output))
}
fn tool_update_callback(
    call: &ToolCall,
    ctx: &ToolExecContext,
    settled: Arc<AtomicBool>,
) -> Box<dyn Fn(serde_json::Value) + Send + Sync> {
    let updates = ctx.updates.clone();
    let bus = ctx.bus.clone();
    let update_call = call.clone();
    Box::new(move |partial_result| {
        if !tool_update_is_live(&settled) {
            return;
        }
        let event = crate::types::AgentEvent::ToolExecutionUpdate {
            tool_call_id: update_call.id.clone(),
            tool_name: update_call.name.clone(),
            args: update_call.arguments.clone(),
            partial_result,
        };
        if let Some(bus) = &bus {
            bus.publish(event);
        } else if let Some(updates) = &updates {
            updates.lock().expect("tool update event lock").push(event);
        }
    })
}

async fn execute_question(
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
async fn execute_web_search(
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
