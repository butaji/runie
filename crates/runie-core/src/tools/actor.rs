//! `ToolExecutorActor` — owns the registry and dispatches batches.

use std::sync::Arc;

use tokio::sync::{mpsc, oneshot};

use crate::events::EventBus;
use crate::task_owner::{spawn_actor_worker, TaskOwner};
use crate::types::{
    AgentContext, AgentEvent, AssistantMessage, ToolCall, ToolExecutionMode, ToolResultContent,
    ToolResultMessage,
};

fn unix_timestamp_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

use super::executor::{
    execute_parallel, execute_sequential, reduce_scheduler_event, DispatchOutcome,
    SchedulerCancellationReason, SchedulerEvent, SchedulerMetrics, ToolExecContext, ToolExecHooks,
};
use super::registry::ToolRegistry;

#[derive(Debug)]
pub enum ToolOutcome {
    /// Finalized dispatch result (already includes emitted events).
    Completed {
        tool_results: Vec<ToolResultMessage>,
        all_terminated: bool,
        events: Vec<crate::types::AgentEvent>,
        scheduler: SchedulerMetrics,
        cancelled: bool,
    },
    /// Tool not found / preflight rejected.
    Aborted { reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ToolPriority {
    Background,
    Interactive,
}

#[allow(
    clippy::large_enum_variant,
    reason = "Execute retains the complete actor-owned request while Metrics stays a small mailbox query"
)]
pub enum ToolCommand {
    Execute {
        assistant_message: AssistantMessage,
        context: AgentContext,
        abort: Option<tokio::sync::watch::Receiver<bool>>,
        bus: Option<EventBus>,
        calls: Vec<ToolCall>,
        mode: ToolExecutionMode,
        priority: ToolPriority,
        hooks: super::executor::ToolExecHooks,
        reply: oneshot::Sender<ToolOutcome>,
    },
    Metrics {
        reply: oneshot::Sender<SchedulerMetrics>,
    },
    CancelQueued {
        reply: oneshot::Sender<usize>,
    },
}

#[derive(Clone)]
pub struct ToolExecutorActor {
    tx: mpsc::Sender<ToolCommand>,
    registry: Arc<ToolRegistry>,
    _worker: Arc<TaskOwner>,
}

impl ToolExecutorActor {
    pub fn new(registry: Arc<ToolRegistry>) -> Self {
        Self::new_with_timestamp(registry, 0)
    }

    /// Construct the production executor with the runtime clock. Replay and
    /// unit callers should use `new_with_timestamp` for deterministic output.
    pub fn new_live(registry: Arc<ToolRegistry>) -> Self {
        Self::new_with_clock(registry, None)
    }

    /// Construct an executor with an injected deterministic tool-result clock.
    pub fn new_with_timestamp(registry: Arc<ToolRegistry>, tool_result_timestamp: i64) -> Self {
        Self::new_with_clock(registry, Some(tool_result_timestamp))
    }

    fn new_with_clock(registry: Arc<ToolRegistry>, tool_result_timestamp: Option<i64>) -> Self {
        let reg = registry.clone();

        // OWNER: ToolExecutorActor
        let (tx, worker) = spawn_actor_worker!(64, move |rx| async move {
            run_tool_worker(rx, reg, tool_result_timestamp).await;
        });

        Self {
            tx,
            registry,
            _worker: worker,
        }
    }

    pub fn registry(&self) -> Arc<ToolRegistry> {
        self.registry.clone()
    }

    pub fn tools(&self) -> Vec<Arc<dyn crate::types::AgentTool>> {
        self.registry.tools()
    }

    /// Read the actor-owned scheduler projection without sharing mutable state.
    pub async fn scheduler_metrics(&self) -> SchedulerMetrics {
        let (reply, response) = oneshot::channel();
        if self.tx.send(ToolCommand::Metrics { reply }).await.is_err() {
            return SchedulerMetrics::default();
        }
        response.await.unwrap_or_default()
    }

    pub async fn cancel_queued(&self) -> usize {
        let (reply, response) = oneshot::channel();
        if self
            .tx
            .send(ToolCommand::CancelQueued { reply })
            .await
            .is_err()
        {
            return 0;
        }
        response.await.unwrap_or_default()
    }

    pub fn mcp_stdio_statuses(&self) -> Vec<crate::tools::McpStdioStatus> {
        self.registry.mcp_stdio_statuses()
    }

    pub fn mcp_http_statuses(&self) -> Vec<crate::tools::McpHttpStatus> {
        self.registry.mcp_http_statuses()
    }

    pub fn mcp_status_rows(&self) -> Vec<crate::tools::McpStatusRow> {
        self.registry.mcp_status_rows()
    }

    pub async fn close_mcps(&self) -> usize {
        self.registry.close_mcps().await
    }

    pub async fn reconnect_mcps(&self) -> usize {
        self.registry.reconnect_mcps().await
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "actor command mirrors the explicit async tool execution contract"
    )]
    pub async fn execute(
        &self,
        assistant_message: AssistantMessage,
        context: AgentContext,
        abort: Option<tokio::sync::watch::Receiver<bool>>,
        bus: Option<EventBus>,
        calls: Vec<ToolCall>,
        mode: ToolExecutionMode,
        hooks: super::executor::ToolExecHooks,
    ) -> ToolOutcome {
        self.execute_with_priority(
            assistant_message,
            context,
            abort,
            bus,
            calls,
            mode,
            ToolPriority::Interactive,
            hooks,
        )
        .await
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "priority is an explicit part of the queued execution contract"
    )]
    pub async fn execute_with_priority(
        &self,
        assistant_message: AssistantMessage,
        context: AgentContext,
        abort: Option<tokio::sync::watch::Receiver<bool>>,
        bus: Option<EventBus>,
        calls: Vec<ToolCall>,
        mode: ToolExecutionMode,
        priority: ToolPriority,
        hooks: super::executor::ToolExecHooks,
    ) -> ToolOutcome {
        let fallback_calls = calls.clone();
        let (reply_tx, reply_rx) = oneshot::channel();
        let _ = self
            .tx
            .send(ToolCommand::Execute {
                assistant_message,
                context,
                abort,
                bus,
                calls,
                mode,
                priority,
                hooks,
                reply: reply_tx,
            })
            .await;
        reply_rx
            .await
            .unwrap_or_else(|_| aborted_outcome(&fallback_calls, "executor dropped"))
    }
}

fn aborted_outcome(calls: &[ToolCall], reason: &str) -> ToolOutcome {
    let mut events = Vec::with_capacity(calls.len() * 2);
    let mut tool_results = Vec::with_capacity(calls.len());
    for call in calls {
        let result = serde_json::json!({
            "content": [{"type": "text", "text": reason}],
            "details": {},
            "terminate": false,
        });
        events.push(AgentEvent::ToolExecutionStart {
            tool_call_id: call.id.clone(),
            tool_name: call.name.clone(),
            args: call.arguments.clone(),
        });
        events.push(AgentEvent::ToolExecutionEnd {
            tool_call_id: call.id.clone(),
            tool_name: call.name.clone(),
            result: result.clone(),
            is_error: true,
        });
        tool_results.push(ToolResultMessage {
            tool_call_id: call.id.clone(),
            tool_name: call.name.clone(),
            content: vec![ToolResultContent::Text {
                text: reason.to_string(),
            }],
            details: serde_json::json!({}),
            is_error: true,
            ..Default::default()
        });
    }
    ToolOutcome::Completed {
        tool_results,
        all_terminated: false,
        events,
        scheduler: SchedulerMetrics::default(),
        cancelled: false,
    }
}

async fn run_tool_worker(
    mut rx: mpsc::Receiver<ToolCommand>,
    registry: Arc<ToolRegistry>,
    tool_result_timestamp: Option<i64>,
) {
    let mut scheduler = SchedulerMetrics::default();
    let mut pending = Vec::new();
    while let Some(cmd) = next_prioritized_command(&mut rx, &mut pending).await {
        if let ToolCommand::Metrics { reply } = cmd {
            let _ = reply.send(scheduler.clone());
            continue;
        }
        if let ToolCommand::CancelQueued { reply } = cmd {
            let cancelled = cancel_pending(&mut pending, &mut scheduler);
            let _ = reply.send(cancelled);
            continue;
        }
        let interactive = matches!(
            &cmd,
            ToolCommand::Execute {
                priority: ToolPriority::Interactive,
                ..
            }
        );
        apply_scheduler(&mut scheduler, SchedulerEvent::Enqueued { interactive });
        apply_scheduler(&mut scheduler, SchedulerEvent::Started);
        let (reply, outcome) = run_tool_command(cmd, &registry, tool_result_timestamp).await;
        settle_scheduler(&mut scheduler, &outcome);
        let _ = reply.send(attach_scheduler(outcome, &scheduler));
    }
}

fn settle_scheduler(scheduler: &mut SchedulerMetrics, outcome: &ToolOutcome) {
    if matches!(
        outcome,
        ToolOutcome::Completed {
            cancelled: true,
            ..
        }
    ) {
        apply_scheduler(
            scheduler,
            SchedulerEvent::CancelledWithReason {
                reason: super::executor::SchedulerCancellationReason::Abort,
            },
        );
    } else {
        let success = matches!(outcome, ToolOutcome::Completed { tool_results, .. } if tool_results.iter().all(|result| !result.is_error));
        apply_scheduler(scheduler, SchedulerEvent::Finished { success });
    }
}

fn attach_scheduler(outcome: ToolOutcome, scheduler: &SchedulerMetrics) -> ToolOutcome {
    match outcome {
        ToolOutcome::Completed {
            tool_results,
            all_terminated,
            events,
            cancelled,
            ..
        } => ToolOutcome::Completed {
            tool_results,
            all_terminated,
            events,
            scheduler: scheduler.clone(),
            cancelled,
        },
        aborted => aborted,
    }
}

fn apply_scheduler(metrics: &mut SchedulerMetrics, event: SchedulerEvent) {
    reduce_scheduler_event(metrics, event).expect("executor scheduler lifecycle must be valid");
}

async fn run_tool_command(
    command: ToolCommand,
    registry: &Arc<ToolRegistry>,
    tool_result_timestamp: Option<i64>,
) -> (oneshot::Sender<ToolOutcome>, ToolOutcome) {
    let ToolCommand::Execute {
        assistant_message,
        context,
        abort,
        bus,
        calls,
        mode,
        priority: _,
        hooks,
        reply,
    } = command
    else {
        unreachable!("metrics commands are handled by the actor worker");
    };
    let effective_mode = effective_tool_mode(registry, &calls, mode);
    let ctx = tool_exec_context(
        assistant_message,
        context,
        abort,
        bus,
        hooks,
        registry.clone(),
        tool_result_timestamp,
    );
    let outcome = execute_tool_calls(calls, ctx, effective_mode).await;
    (reply, completed_tool_outcome(outcome))
}

async fn execute_tool_calls(
    calls: Vec<ToolCall>,
    ctx: ToolExecContext,
    mode: ToolExecutionMode,
) -> DispatchOutcome {
    match mode {
        ToolExecutionMode::Sequential => execute_sequential(calls, ctx).await,
        ToolExecutionMode::Parallel => execute_parallel(calls, ctx).await,
    }
}

fn completed_tool_outcome(outcome: DispatchOutcome) -> ToolOutcome {
    ToolOutcome::Completed {
        tool_results: outcome.tool_results,
        all_terminated: outcome.all_terminated,
        events: outcome.events,
        scheduler: SchedulerMetrics::default(),
        cancelled: outcome.cancelled,
    }
}

async fn next_prioritized_command(
    rx: &mut mpsc::Receiver<ToolCommand>,
    pending: &mut Vec<ToolCommand>,
) -> Option<ToolCommand> {
    if pending.is_empty() {
        pending.push(rx.recv().await?);
    }
    while let Ok(command) = rx.try_recv() {
        pending.push(command);
    }
    let selected = pending
        .iter()
        .enumerate()
        .max_by_key(|(_, command)| match command {
            ToolCommand::Execute { priority, .. } => *priority,
            ToolCommand::Metrics { .. } | ToolCommand::CancelQueued { .. } => {
                ToolPriority::Background
            }
        })
        .map(|(index, _)| index)
        .expect("pending command is non-empty");
    Some(pending.swap_remove(selected))
}

fn cancel_pending(pending: &mut Vec<ToolCommand>, scheduler: &mut SchedulerMetrics) -> usize {
    let mut cancelled = 0;
    let mut retained = Vec::with_capacity(pending.len());
    for command in pending.drain(..) {
        if let ToolCommand::Execute {
            reply,
            calls,
            priority,
            ..
        } = command
        {
            apply_scheduler(
                scheduler,
                SchedulerEvent::Enqueued {
                    interactive: priority == ToolPriority::Interactive,
                },
            );
            apply_scheduler(
                scheduler,
                SchedulerEvent::CancelledQueued {
                    reason: SchedulerCancellationReason::User,
                },
            );
            let _ = reply.send(aborted_outcome(&calls, "queued execution cancelled"));
            cancelled += 1;
        } else {
            retained.push(command);
        }
    }
    *pending = retained;
    cancelled
}

fn effective_tool_mode(
    registry: &ToolRegistry,
    calls: &[ToolCall],
    mode: ToolExecutionMode,
) -> ToolExecutionMode {
    if calls
        .iter()
        .any(|call| registry.execution_mode(&call.name) == Some(ToolExecutionMode::Sequential))
    {
        ToolExecutionMode::Sequential
    } else {
        mode
    }
}

fn tool_exec_context(
    assistant_message: AssistantMessage,
    context: AgentContext,
    abort: Option<tokio::sync::watch::Receiver<bool>>,
    bus: Option<EventBus>,
    hooks: ToolExecHooks,
    registry: Arc<ToolRegistry>,
    timestamp: Option<i64>,
) -> ToolExecContext {
    ToolExecContext {
        assistant_message,
        context,
        abort,
        bus,
        registry,
        hooks,
        updates: None,
        tool_result_timestamp: timestamp.unwrap_or_else(unix_timestamp_millis),
    }
}

#[cfg(test)]
#[path = "actor_tests.rs"]
mod tests;
