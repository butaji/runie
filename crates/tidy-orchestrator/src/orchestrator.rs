use async_trait::async_trait;
use tidy_core::Context;
use crate::{Task, SubagentHandle, SubagentResult, SubagentStatus, HandoffProtocol};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum OrchestratorError {
    #[error("orchestrator error: {0}")]
    Failed(String),
    #[error("subagent not found: {0}")]
    SubagentNotFound(String),
    #[error("handoff error: {0}")]
    HandoffError(String),
    #[error("max subagents exceeded")]
    MaxSubagentsExceeded,
}

#[async_trait]
pub trait Orchestrator: Send + Sync {
    /// Spawn a new subagent for a task.
    async fn spawn(
        &self,
        task: Task,
        parent_context: &Context,
    ) -> Result<SubagentHandle, OrchestratorError>;

    /// Handoff context from one agent to another.
    async fn handoff(
        &self,
        from: &str,
        to: &str,
        context: &Context,
    ) -> Result<(), OrchestratorError>;

    /// Collect results from subagents.
    async fn collect(
        &self,
        handles: Vec<SubagentHandle>,
    ) -> Result<Vec<SubagentResult>, OrchestratorError>;

    /// Cancel a running subagent.
    async fn cancel(&self, handle_id: &str) -> Result<(), OrchestratorError>;
}

/// Simple orchestrator that manages subagents in memory.
pub struct SimpleOrchestrator {
    subagents: Arc<RwLock<HashMap<String, SubagentHandle>>>,
    handoff_protocol: Arc<dyn HandoffProtocol>,
    max_subagents: usize,
}

impl SimpleOrchestrator {
    pub fn new(handoff_protocol: Arc<dyn HandoffProtocol>, max_subagents: usize) -> Self {
        Self {
            subagents: Arc::new(RwLock::new(HashMap::new())),
            handoff_protocol,
            max_subagents,
        }
    }
}

#[async_trait]
impl Orchestrator for SimpleOrchestrator {
    async fn spawn(
        &self,
        task: Task,
        _parent_context: &Context,
    ) -> Result<SubagentHandle, OrchestratorError> {
        let subagents = self.subagents.read().await;
        if subagents.len() >= self.max_subagents {
            return Err(OrchestratorError::MaxSubagentsExceeded);
        }
        drop(subagents);

        let handle = SubagentHandle {
            id: task.id.clone(),
            task: task.clone(),
            status: SubagentStatus::Pending,
            created_at: Utc::now(),
        };

        let mut subagents = self.subagents.write().await;
        subagents.insert(handle.id.clone(), handle.clone());

        Ok(handle)
    }

    async fn handoff(
        &self,
        from: &str,
        to: &str,
        context: &Context,
    ) -> Result<(), OrchestratorError> {
        let payload = self.handoff_protocol.export(context).await
            .map_err(|e| OrchestratorError::HandoffError(e.to_string()))?;

        // In real impl, we'd find the target agent and import context
        // For now, just log the handoff
        tracing::info!("Handoff from {} to {} with {} messages", from, to, payload.session_snapshot.messages.len());
        Ok(())
    }

    async fn collect(
        &self,
        handles: Vec<SubagentHandle>,
    ) -> Result<Vec<SubagentResult>, OrchestratorError> {
        let mut results = Vec::new();

        for handle in handles {
            let subagents = self.subagents.read().await;
            if let Some(existing) = subagents.get(&handle.id) {
                results.push(SubagentResult {
                    handle: existing.clone(),
                    events: Vec::new(),
                    final_output: String::new(),
                    completed_at: Utc::now(),
                });
            }
        }

        Ok(results)
    }

    async fn cancel(&self, handle_id: &str) -> Result<(), OrchestratorError> {
        let mut subagents = self.subagents.write().await;
        if let Some(handle) = subagents.get_mut(handle_id) {
            handle.status = SubagentStatus::Cancelled;
            Ok(())
        } else {
            Err(OrchestratorError::SubagentNotFound(handle_id.to_string()))
        }
    }
}
