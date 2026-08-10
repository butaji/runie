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
    execute_parallel, execute_sequential, reduce_scheduler_event, SchedulerEvent, SchedulerMetrics,
    ToolExecContext, ToolExecHooks,
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

    pub fn mcp_stdio_statuses(&self) -> Vec<crate::tools::McpStdioStatus> {
        self.registry.mcp_stdio_statuses()
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
    while let Some(cmd) = next_prioritized_command(&mut rx).await {
        if let ToolCommand::Metrics { reply } = cmd {
            let _ = reply.send(scheduler.clone());
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
        let cancelled = matches!(
            &outcome,
            ToolOutcome::Completed {
                cancelled: true,
                ..
            }
        );
        if cancelled {
            apply_scheduler(
                &mut scheduler,
                SchedulerEvent::CancelledWithReason {
                    reason: super::executor::SchedulerCancellationReason::Abort,
                },
            );
        } else {
            let success = matches!(
            &outcome,
            ToolOutcome::Completed { tool_results, .. }
                if tool_results.iter().all(|result| !result.is_error)
            );
            apply_scheduler(&mut scheduler, SchedulerEvent::Finished { success });
        }
        let outcome = match outcome {
            ToolOutcome::Completed {
                tool_results,
                all_terminated,
                events,
                cancelled: _cancelled,
                ..
            } => ToolOutcome::Completed {
                tool_results,
                all_terminated,
                events,
                scheduler: scheduler.clone(),
                cancelled: _cancelled,
            },
            aborted => aborted,
        };
        let _ = reply.send(outcome);
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
    let outcome = match effective_mode {
        ToolExecutionMode::Sequential => execute_sequential(calls, ctx).await,
        ToolExecutionMode::Parallel => execute_parallel(calls, ctx).await,
    };
    (
        reply,
        ToolOutcome::Completed {
            tool_results: outcome.tool_results,
            all_terminated: outcome.all_terminated,
            events: outcome.events,
            scheduler: SchedulerMetrics::default(),
            cancelled: outcome.cancelled,
        },
    )
}

async fn next_prioritized_command(rx: &mut mpsc::Receiver<ToolCommand>) -> Option<ToolCommand> {
    let first = rx.recv().await?;
    let mut pending = vec![first];
    while let Ok(command) = rx.try_recv() {
        pending.push(command);
    }
    let selected = pending
        .iter()
        .enumerate()
        .max_by_key(|(_, command)| match command {
            ToolCommand::Execute { priority, .. } => *priority,
            ToolCommand::Metrics { .. } => ToolPriority::Background,
        })
        .map(|(index, _)| index)
        .expect("pending command is non-empty");
    Some(pending.swap_remove(selected))
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
mod tests {
    use super::*;

    struct ActorCancellationProbe {
        started: tokio::sync::watch::Sender<bool>,
    }

    #[async_trait::async_trait]
    impl crate::types::AgentTool for ActorCancellationProbe {
        fn name(&self) -> &str {
            "actor_cancel_probe"
        }
        fn label(&self) -> &str {
            "actor_cancel_probe"
        }
        fn description(&self) -> &str {
            "actor cancellation test tool"
        }

        async fn execute(
            &self,
            _: &str,
            _: serde_json::Value,
            signal: Option<tokio_util::sync::CancellationToken>,
            _: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
        ) -> Result<crate::types::AgentToolResult, String> {
            let _ = self.started.send(true);
            signal
                .expect("actor supplies cancellation")
                .cancelled()
                .await;
            Err("cancelled by actor probe".into())
        }
    }

    #[tokio::test]
    async fn execute_empty_batch_completes() {
        let registry = Arc::new(ToolRegistry::new());
        let actor = ToolExecutorActor::new(registry);
        let outcome = actor
            .execute(
                crate::types::AssistantMessage {
                    content: vec![],
                    stop_reason: None,
                    model: "test".into(),
                    timestamp: 0,
                    ..Default::default()
                },
                crate::types::AgentContext::default(),
                None,
                None,
                vec![],
                ToolExecutionMode::Parallel,
                super::super::executor::ToolExecHooks::default(),
            )
            .await;
        match outcome {
            ToolOutcome::Completed {
                tool_results,
                scheduler,
                ..
            } => {
                assert!(tool_results.is_empty());
                assert_eq!(scheduler.completed, 1);
                assert_eq!(scheduler.running, 0);
                assert_eq!(scheduler.interactive_enqueued, 1);
            }
            ToolOutcome::Aborted { .. } => panic!("expected completed"),
        }
        let snapshot = actor.scheduler_metrics().await;
        assert_eq!(snapshot.completed, 1);
        assert_eq!(snapshot.running, 0);

        let _ = actor
            .execute(
                crate::types::AssistantMessage {
                    content: vec![],
                    stop_reason: None,
                    model: "test".into(),
                    timestamp: 0,
                    ..Default::default()
                },
                crate::types::AgentContext::default(),
                None,
                None,
                vec![ToolCall {
                    id: "missing".into(),
                    name: "missing_tool".into(),
                    arguments: serde_json::json!({}),
                    thought_signature: None,
                }],
                ToolExecutionMode::Sequential,
                super::super::executor::ToolExecHooks::default(),
            )
            .await;
        let snapshot = actor.scheduler_metrics().await;
        assert_eq!(snapshot.completed, 1);
        assert_eq!(snapshot.failed, 1);
    }

    #[tokio::test]
    async fn aborting_actor_execution_projects_scheduler_cancellation() {
        let (started_tx, mut started_rx) = tokio::sync::watch::channel(false);
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(ActorCancellationProbe {
            started: started_tx,
        }));
        let actor = ToolExecutorActor::new(Arc::new(registry));
        let (abort_tx, abort_rx) = tokio::sync::watch::channel(false);
        let execute = actor.execute(
            crate::types::AssistantMessage::default(),
            crate::types::AgentContext::default(),
            Some(abort_rx),
            None,
            vec![ToolCall {
                id: "cancel-1".into(),
                name: "actor_cancel_probe".into(),
                arguments: serde_json::json!({}),
                thought_signature: None,
            }],
            ToolExecutionMode::Sequential,
            super::super::executor::ToolExecHooks::default(),
        );
        let abort = async {
            while !*started_rx.borrow() {
                let _ = started_rx.changed().await;
            }
            let _ = abort_tx.send(true);
        };
        let (outcome, ()) = tokio::join!(execute, abort);
        assert!(matches!(
            outcome,
            ToolOutcome::Completed {
                cancelled: true,
                ..
            }
        ));
        let snapshot = actor.scheduler_metrics().await;
        assert_eq!(snapshot.cancelled, 1);
        assert_eq!(snapshot.running, 0);
    }

    #[test]
    fn dropped_executor_fallback_is_an_error_tool_result() {
        let calls = vec![ToolCall {
            id: "call-1".into(),
            name: "echo".into(),
            arguments: serde_json::json!({}),
            thought_signature: None,
        }];
        let ToolOutcome::Completed {
            tool_results,
            events,
            all_terminated,
            ..
        } = aborted_outcome(&calls, "executor dropped")
        else {
            panic!("fallback must complete with synthetic tool results");
        };
        assert!(!all_terminated);
        assert!(tool_results[0].is_error);
        assert_eq!(tool_results[0].tool_call_id, "call-1");
        assert!(matches!(
            events[1],
            AgentEvent::ToolExecutionEnd { is_error: true, .. }
        ));
    }

    #[tokio::test]
    async fn queued_command_selection_prefers_interactive_work() {
        let (tx, mut rx) = mpsc::channel(4);
        for priority in [ToolPriority::Background, ToolPriority::Interactive] {
            let (reply, _) = oneshot::channel();
            tx.send(ToolCommand::Execute {
                assistant_message: AssistantMessage::default(),
                context: AgentContext::default(),
                abort: None,
                bus: None,
                calls: vec![],
                mode: ToolExecutionMode::Parallel,
                priority,
                hooks: ToolExecHooks::default(),
                reply,
            })
            .await
            .unwrap();
        }
        let selected = next_prioritized_command(&mut rx).await.unwrap();
        assert!(matches!(
            selected,
            ToolCommand::Execute {
                priority: ToolPriority::Interactive,
                ..
            }
        ));
    }
}
