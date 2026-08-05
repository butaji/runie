//! `ToolExecutorActor` — owns the registry and dispatches batches.

use std::sync::Arc;

use tokio::sync::{mpsc, oneshot};

use crate::types::{AssistantMessage, ToolCall, ToolExecutionMode, ToolResultMessage};

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
}

impl ToolExecutorActor {
    pub fn new(registry: Arc<ToolRegistry>) -> Self {
        let (tx, rx) = mpsc::channel(64);
        let reg = registry.clone();

        // OWNER: ToolExecutorActor
        tokio::spawn(async move {
            run_tool_worker(rx, reg).await;
        });

        Self { tx, registry }
    }

    pub fn registry(&self) -> Arc<ToolRegistry> {
        self.registry.clone()
    }

    pub async fn execute(
        &self,
        assistant_message: AssistantMessage,
        calls: Vec<ToolCall>,
        mode: ToolExecutionMode,
        hooks: super::executor::ToolExecHooks,
    ) -> ToolOutcome {
        let (reply_tx, reply_rx) = oneshot::channel();
        let _ = self
            .tx
            .send(ToolCommand::Execute {
                assistant_message,
                calls,
                mode,
                hooks,
                reply: reply_tx,
            })
            .await;
        reply_rx.await.unwrap_or(ToolOutcome::Aborted {
            reason: "executor dropped".into(),
        })
    }
}

async fn run_tool_worker(mut rx: mpsc::Receiver<ToolCommand>, registry: Arc<ToolRegistry>) {
    while let Some(cmd) = rx.recv().await {
        let ToolCommand::Execute {
            assistant_message,
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
            registry: registry.clone(),
            hooks,
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
}
