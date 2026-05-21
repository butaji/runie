use runie_core::{Context, Session};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Protocol for transferring context between agents.
#[async_trait::async_trait]
pub trait HandoffProtocol: Send + Sync {
    /// Serialize context for transfer.
    async fn export(&self, context: &Context) -> Result<HandoffPayload, HandoffError>;

    /// Deserialize and merge context into receiving agent.
    async fn import(&self, payload: HandoffPayload, target_context: &mut Context) -> Result<(), HandoffError>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HandoffPayload {
    pub session_snapshot: Session,
    pub working_memory_summary: String,
    pub key_files: Vec<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum HandoffError {
    #[error("handoff failed: {0}")]
    Failed(String),
    #[error("incompatible context")]
    IncompatibleContext,
}

/// Default handoff that copies session and working memory.
pub struct DefaultHandoff;

#[async_trait::async_trait]
impl HandoffProtocol for DefaultHandoff {
    async fn export(&self, context: &Context) -> Result<HandoffPayload, HandoffError> {
        Ok(HandoffPayload {
            session_snapshot: context.session.clone(),
            working_memory_summary: context.working_memory.current_task.clone(),
            key_files: context.working_memory.key_files.clone(),
            metadata: context.working_memory.custom_data.clone(),
        })
    }

    async fn import(&self, payload: HandoffPayload, target_context: &mut Context) -> Result<(), HandoffError> {
        target_context.session = payload.session_snapshot;
        target_context.working_memory.current_task = payload.working_memory_summary;
        target_context.working_memory.key_files = payload.key_files;
        target_context.working_memory.custom_data = payload.metadata;
        Ok(())
    }
}
