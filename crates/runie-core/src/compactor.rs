use async_trait::async_trait;
use thiserror::Error;
use crate::Session;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum CompactorError {
    #[error("compaction failed: {0}")]
    Failed(String),
}

#[async_trait]
pub trait Compactor: Send + Sync {
    async fn compact(&self, session: &Session, target_tokens: usize) -> Result<Session, CompactorError>;
}

/// A simple compactor that summarizes older messages.
pub struct SimpleCompactor;

#[async_trait]
impl Compactor for SimpleCompactor {
    async fn compact(&self, session: &Session, _target_tokens: usize) -> Result<Session, CompactorError> {
        // Stub: just clone the session. Real impl would summarize.
        Ok(session.clone())
    }
}
