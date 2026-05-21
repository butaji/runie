use async_trait::async_trait;
use tidy_core::Event;
use crate::AgentLoop;

#[async_trait]
pub trait Agent: Send + Sync {
    async fn run(&mut self, request: String) -> Result<Vec<Event>, AgentError>;
    async fn stop(&mut self);
    fn is_running(&self) -> bool;
}

#[derive(Debug, thiserror::Error, Clone, PartialEq)]
pub enum AgentError {
    #[error("agent error: {0}")]
    Failed(String),
}

pub struct CodingAgent {
    loop_inner: AgentLoop,
}

impl CodingAgent {
    pub fn new(loop_inner: AgentLoop) -> Self {
        Self { loop_inner }
    }
}

#[async_trait]
impl Agent for CodingAgent {
    async fn run(&mut self, request: String) -> Result<Vec<Event>, AgentError> {
        self.loop_inner.run(request).await
            .map_err(|e| AgentError::Failed(e.to_string()))
    }

    async fn stop(&mut self) {
        self.loop_inner.state.is_running = false;
    }

    fn is_running(&self) -> bool {
        self.loop_inner.state.is_running
    }
}
