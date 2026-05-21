use async_trait::async_trait;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct RoutingContext {
    pub message_history: Vec<runie_core::Message>,
    pub estimated_tokens: usize,
    pub task_complexity: f32, // 0.0 to 1.0
    pub cost_budget: Option<f64>,
    pub latency_requirement: Option<Duration>,
    pub required_capabilities: Vec<String>,
}

#[derive(Debug, thiserror::Error, Clone, PartialEq)]
pub enum RouterError {
    #[error("routing error: {0}")]
    Failed(String),
    #[error("no suitable provider found")]
    NoSuitableProvider,
    #[error("provider not found: {0}")]
    ProviderNotFound(String),
}

#[async_trait]
pub trait Router: Send + Sync {
    /// Select the best provider for the given context.
    async fn select_provider(
        &self,
        context: &RoutingContext,
        available: &[String],
    ) -> Result<String, RouterError>;

    /// Determine if a handoff to a different provider is warranted.
    async fn should_handoff(
        &self,
        current: &str,
        context: &RoutingContext,
    ) -> Result<Option<String>, RouterError>;
}

/// Provider metadata for routing decisions.
#[derive(Debug, Clone)]
pub struct ProviderMetadata {
    pub name: String,
    pub model: String,
    pub cost_per_1k_input: f64,
    pub cost_per_1k_output: f64,
    pub max_tokens: usize,
    pub capabilities: Vec<String>,
    pub avg_latency_ms: u64,
    pub reliability_score: f32, // 0.0 to 1.0
}
