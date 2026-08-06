//! Sequential and parallel tool dispatch.
//!
//! Both paths emit `ToolExecutionStart` / `ToolExecutionEnd` events through
//! the supplied sink. In parallel mode, completion-order and source-order
//! are separated per the TS README §With Tool Calls.

use std::{
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use futures::StreamExt;

use super::registry::ToolRegistry;
use crate::types::{
    AgentContext, AgentToolResult, AssistantMessage, BeforeToolCallContext, BeforeToolCallResult,
    ToolCall, ToolResultContent, ToolResultMessage,
};

/// Hooks applied during tool dispatch. `None` for any field means "use
/// default (allow / no override)".
#[derive(Default, Clone)]
pub struct ToolExecHooks {
    pub before_tool_call: Option<BeforeToolCallHook>,
    pub after_tool_call: Option<AfterToolCallHook>,
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
    pub updates: Arc<std::sync::Mutex<Vec<crate::types::AgentEvent>>>,
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
        // The lifecycle begins before the tool side effect, matching pi's
        // tool_execution_start contract. Completion and result events follow.
        outcome.events.push(tool_start(&call));
        let (result, is_error) = match dispatch_one(&call, &ctx).await {
            Ok(result) => result,
            Err(msg) => (synthetic_error_result(&msg), true),
        };
        outcome.events.extend(take_updates(&ctx));
        outcome.events.push(tool_end(&call, &result, is_error));
        outcome
            .tool_results
            .push(tool_result_message(&call, &result, is_error));
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
) -> ToolResultMessage {
    ToolResultMessage {
        tool_call_id: call.id.clone(),
        tool_name: call.name.clone(),
        content: result.content.clone(),
        details: result.details.clone(),
        usage: result.usage.clone(),
        added_tool_names: result.added_tool_names.clone(),
        is_error,
        timestamp: 0,
    }
}

pub async fn execute_parallel(calls: Vec<ToolCall>, ctx: ToolExecContext) -> DispatchOutcome {
    // Preflight: valid calls (after prepare_arguments + validation) proceed to
    // concurrent execution; invalid calls produce an immediate error result
    // (pi prepareToolCall -> createErrorToolResult).
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
            outcome
                .tool_results
                .push(tool_result_message_named(call, name, &r, is_error));
            if !r.terminate {
                all_terminated = false;
            }
        }
    }

    // An invalid (preflight-rejected) call means the batch is not fully
    // terminated.
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
                append_failed_call(&mut outcome, &call, &message);
                had_invalid = true;
            }
        }
    }
    (valid, outcome, had_invalid)
}

fn append_failed_call(outcome: &mut DispatchOutcome, call: &ToolCall, message: &str) {
    let result = synthetic_error_result(message);
    outcome.events.push(tool_start(call));
    outcome.events.push(tool_end(call, &result, true));
    outcome
        .tool_results
        .push(tool_result_message(call, &result, true));
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
) -> ToolResultMessage {
    let mut message = tool_result_message(call, result, is_error);
    message.tool_name = name;
    message
}

/// Apply `prepareArguments` (pi agent-loop.ts:586) and validate. Returns the
/// prepared call (args possibly replaced) or a pi-formatted validation error.
fn prepare_and_validate(call: &ToolCall, ctx: &ToolExecContext) -> Result<ToolCall, String> {
    if ctx.registry.lookup(&call.name).is_none() {
        return Err(format!("Tool {} not found", call.name));
    }
    let prepared = prepared_call(call, ctx);
    if let Some(tool) = ctx.registry.lookup(&prepared.name) {
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

/// Apply the tool's `prepareArguments`, replacing the args when changed.
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

    if let Some(ref hook) = ctx.hooks.before_tool_call {
        let decision = hook(BeforeToolCallContext {
            assistant_message: ctx.assistant_message.clone(),
            tool_call: call.clone(),
            args: call.arguments.clone(),
            context: ctx.context.clone(),
            signal: signal.clone(),
        })
        .await;
        if decision.block {
            return Err(decision
                .reason
                .unwrap_or_else(|| "Tool execution was blocked".into()));
        }
    }

    let (result, is_error) = match execute_tool(tool, call, ctx, signal.clone()).await {
        Ok(result) => (result, false),
        Err(reason) => (synthetic_error_result(&reason), true),
    };

    Ok(apply_after_tool_hook(call, ctx, result, signal, is_error).await)
}

async fn execute_tool(
    tool: Arc<dyn crate::types::AgentTool>,
    call: &ToolCall,
    ctx: &ToolExecContext,
    signal: tokio_util::sync::CancellationToken,
) -> Result<AgentToolResult, String> {
    let updates = ctx.updates.clone();
    let bus = ctx.bus.clone();
    let update_call = call.clone();
    let settled = Arc::new(AtomicBool::new(false));
    let callback_settled = settled.clone();
    let on_update = Box::new(move |partial_result| {
        // Pi scopes updates to the execute promise. A callback retained by a
        // tool after it settles must become a no-op, even if it fires later.
        if !tool_update_is_live(&callback_settled) {
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
        } else {
            updates.lock().expect("tool update event lock").push(event);
        }
    });
    let tool_future = tool.execute(
        &call.id,
        call.arguments.clone(),
        Some(signal.clone()),
        Some(on_update),
    );
    let result = tokio::select! {
        result = tool_future => result.map_err(|e| e.to_string()),
        aborted = wait_for_tool_abort(ctx.abort.clone()) => {
            signal.cancel();
            if aborted { Err("aborted".into()) } else { Err("tool execution aborted".into()) }
        }
    };
    settled.store(true, Ordering::Release);
    result
}

fn tool_update_is_live(settled: &AtomicBool) -> bool {
    !settled.load(Ordering::Acquire)
}

async fn wait_for_tool_abort(mut abort: Option<tokio::sync::watch::Receiver<bool>>) -> bool {
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

fn take_updates(ctx: &ToolExecContext) -> Vec<crate::types::AgentEvent> {
    std::mem::take(&mut *ctx.updates.lock().expect("tool update event lock"))
}

async fn apply_after_tool_hook(
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

fn synthetic_error_result(reason: &str) -> AgentToolResult {
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
mod tests {
    use super::*;
    use crate::types::{AfterToolCallContext, AfterToolCallResult, AgentTool};

    struct CancellationProbe {
        started: tokio::sync::watch::Sender<bool>,
    }

    #[async_trait::async_trait]
    impl AgentTool for CancellationProbe {
        fn name(&self) -> &str {
            "cancel_probe"
        }

        fn label(&self) -> &str {
            "Cancellation probe"
        }

        fn description(&self) -> &str {
            "Waits for cancellation."
        }

        async fn execute(
            &self,
            _tool_call_id: &str,
            _args: serde_json::Value,
            signal: Option<tokio_util::sync::CancellationToken>,
            on_update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
        ) -> Result<AgentToolResult, String> {
            if let Some(on_update) = on_update {
                on_update(serde_json::json!({"phase": "started"}));
            }
            let _ = self.started.send(true);
            signal
                .expect("executor must provide a cancellation token")
                .cancelled()
                .await;
            Err("cancelled by probe".into())
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
                    ..Default::default()
                },
                context: crate::types::AgentContext::default(),
                abort: None,
                registry,
                hooks: ToolExecHooks::default(),
                bus: None,
                updates: Arc::new(std::sync::Mutex::new(Vec::new())),
            };
            let outcome = execute_sequential(vec![], ctx).await;
            assert!(outcome.tool_results.is_empty());
            assert!(!outcome.all_terminated);
        });
    }

    #[test]
    fn settled_tool_update_callbacks_are_ignored() {
        let settled = AtomicBool::new(false);
        assert!(tool_update_is_live(&settled));
        settled.store(true, Ordering::Release);
        assert!(!tool_update_is_live(&settled));
    }

    #[tokio::test]
    #[allow(
        clippy::too_many_lines,
        reason = "the cancellation regression covers live bus delivery and finalization"
    )]
    async fn abort_cancels_in_flight_tool_and_emits_error_result() {
        let (started_tx, mut started_rx) = tokio::sync::watch::channel(false);
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(CancellationProbe {
            started: started_tx,
        }));
        let (abort_tx, abort_rx) = tokio::sync::watch::channel(false);
        let bus = crate::events::EventBus::new();
        let mut bus_events = bus.subscribe();
        let ctx = ToolExecContext {
            assistant_message: AssistantMessage::default(),
            context: crate::types::AgentContext::default(),
            abort: Some(abort_rx),
            bus: Some(bus),
            registry: Arc::new(registry),
            hooks: ToolExecHooks {
                after_tool_call: Some(Arc::new(|input| {
                    Box::pin(async move {
                        assert!(input.is_error);
                        crate::types::AfterToolCallResult::default()
                    })
                })),
                ..ToolExecHooks::default()
            },
            updates: Arc::new(std::sync::Mutex::new(Vec::new())),
        };
        let call = ToolCall {
            id: "cancel-1".into(),
            name: "cancel_probe".into(),
            arguments: serde_json::json!({}),
            thought_signature: None,
        };
        let abort_when_started = async {
            while !*started_rx.borrow() {
                let _ = started_rx.changed().await;
            }
            let _ = abort_tx.send(true);
        };
        let (outcome, _) = tokio::join!(execute_sequential(vec![call], ctx), abort_when_started);
        let live_update = bus_events
            .try_recv()
            .expect("tool update should publish before completion");
        assert!(matches!(
            live_update,
            crate::types::AgentEvent::ToolExecutionUpdate { .. }
        ));
        assert!(outcome.tool_results[0].is_error);
        assert_eq!(outcome.tool_results[0].details, serde_json::json!({}));
        assert!(matches!(
            outcome.tool_results[0].content.first(),
            Some(ToolResultContent::Text { text }) if text == "aborted"
        ));
    }

    #[allow(dead_code)]
    fn _after_ctx_marker(_c: AfterToolCallContext) -> AfterToolCallResult {
        AfterToolCallResult::default()
    }
}
