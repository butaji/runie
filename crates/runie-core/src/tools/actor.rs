//! `ToolExecutorActor` — owns the registry and dispatches batches.

use std::sync::Arc;

use tokio::sync::{mpsc, oneshot};

use crate::events::EventBus;
use crate::task_owner::{spawn_actor_worker, TaskOwner};
use crate::types::{
    AgentContext, AgentEvent, AssistantMessage, ToolCall, ToolExecutionMode, ToolResultContent,
    ToolResultMessage,
};

use super::executor::{execute_parallel, execute_sequential, DispatchOutcome, ToolExecContext};
use super::registry::ToolRegistry;

#[derive(Debug)]
pub enum ToolOutcome {
    /// Finalized dispatch result (already includes emitted events).
    Completed {
        tool_results: Vec<ToolResultMessage>,
        all_terminated: bool,
        events: Vec<crate::types::AgentEvent>,
    },
    /// Tool not found / preflight rejected.
    Aborted { reason: String },
}

pub enum ToolCommand {
    Execute {
        assistant_message: AssistantMessage,
        context: AgentContext,
        abort: Option<tokio::sync::watch::Receiver<bool>>,
        bus: Option<EventBus>,
        calls: Vec<ToolCall>,
        mode: ToolExecutionMode,
        hooks: super::executor::ToolExecHooks,
        reply: oneshot::Sender<ToolOutcome>,
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

    /// Construct an executor with an injected deterministic tool-result clock.
    pub fn new_with_timestamp(registry: Arc<ToolRegistry>, tool_result_timestamp: i64) -> Self {
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
    }
}

async fn run_tool_worker(
    mut rx: mpsc::Receiver<ToolCommand>,
    registry: Arc<ToolRegistry>,
    tool_result_timestamp: i64,
) {
    while let Some(cmd) = rx.recv().await {
        let ToolCommand::Execute {
            assistant_message,
            context,
            abort,
            bus,
            calls,
            mode,
            hooks,
            reply,
        } = cmd;

        // Promote any per-tool override to sequential mode (README §tool
        // execution: any sequential tool forces the whole batch).
        let mut effective_mode = mode;
        for call in &calls {
            if let Some(ToolExecutionMode::Sequential) = registry.execution_mode(&call.name) {
                effective_mode = ToolExecutionMode::Sequential;
                break;
            }
        }

        let ctx = ToolExecContext {
            assistant_message,
            context,
            abort,
            bus,
            registry: registry.clone(),
            hooks,
            updates: Arc::new(std::sync::Mutex::new(Vec::new())),
            tool_result_timestamp,
        };

        let outcome: DispatchOutcome = match effective_mode {
            ToolExecutionMode::Sequential => execute_sequential(calls, ctx).await,
            ToolExecutionMode::Parallel => execute_parallel(calls, ctx).await,
        };

        let _ = reply.send(ToolOutcome::Completed {
            tool_results: outcome.tool_results,
            all_terminated: outcome.all_terminated,
            events: outcome.events,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            ToolOutcome::Completed { tool_results, .. } => assert!(tool_results.is_empty()),
            ToolOutcome::Aborted { .. } => panic!("expected completed"),
        }
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
}
