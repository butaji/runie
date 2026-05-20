//! AI Layer — Model abstraction inspired by pi's @earendil-works/pi-ai
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │  Model                                                          │
//! │  - id, name, provider                                          │
//! │  - context_window, max_tokens                                   │
//! │  - input/output cost                                           │
//! │  - capabilities (vision, function_calling, streaming)           │
//! └─────────────────────────────────────────────────────────────────┘
//!     ↓
//! ┌─────────────────────────────────────────────────────────────────┐
//! │  ModelProvider (trait)                                          │
//! │  - stream() -> Stream of deltas                                │
//! │  - complete() -> Full response                                 │
//! │  - cost() -> Cost estimate                                      │
//! └─────────────────────────────────────────────────────────────────┘
//!     ↓
//! ┌─────────────────────────────────────────────────────────────────┐
//! │  Providers                                                       │
//! │  - AnthropicProvider (Claude)                                  │
//! │  - OpenAIProvider (GPT-4o, o1)                                 │
//! │  - OllamaProvider (local)                                      │
//! │  - DeepSeekProvider                                            │
//! │  - GoogleProvider (Gemini)                                     │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Cost tracking
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Cost {
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub cache_read_tokens: usize,
    pub cache_write_tokens: usize,
    pub input_cost: f64,
    pub output_cost: f64,
    pub cache_read_cost: f64,
    pub cache_write_cost: f64,
}

impl Cost {
    pub fn zero() -> Self {
        Self::default()
    }

    pub fn total_cost(&self) -> f64 {
        self.input_cost + self.output_cost + self.cache_read_cost + self.cache_write_cost
    }

    pub fn add(&mut self, other: &Cost) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.cache_read_tokens += other.cache_read_tokens;
        self.cache_write_tokens += other.cache_write_tokens;
        self.input_cost += other.input_cost;
        self.output_cost += other.output_cost;
        self.cache_read_cost += other.cache_read_cost;
        self.cache_write_cost += other.cache_write_cost;
    }
}

/// Model capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub vision: bool,
    pub function_calling: bool,
    pub streaming: bool,
    pub reasoning: bool,
}

/// Model definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub api: String,  // API type: "anthropic-messages", "openai-responses", etc.
    pub context_window: usize,
    pub max_tokens: usize,
    pub input_cost: f64,  // per million tokens
    pub output_cost: f64, // per million tokens
    pub cache_cost: Option<f64>,  // cache read cost
    pub capabilities: ModelCapabilities,
    pub base_url: Option<String>,
    pub thinking: bool,
}

impl Model {
    /// Get cost for a given token count
    pub fn estimate_cost(&self, input_tokens: usize, output_tokens: usize) -> Cost {
        let input_cost = (input_tokens as f64 / 1_000_000.0) * self.input_cost;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * self.output_cost;
        
        Cost {
            input_tokens,
            output_tokens,
            input_cost,
            output_cost,
            ..Default::default()
        }
    }

    /// Default Claude Sonnet 4
    pub fn default_claude() -> Self {
        Self {
            id: "anthropic/claude-sonnet-4".to_string(),
            name: "Claude Sonnet 4".to_string(),
            provider: "anthropic".to_string(),
            api: "anthropic-messages".to_string(),
            context_window: 200_000,
            max_tokens: 8192,
            input_cost: 3.00,
            output_cost: 15.00,
            cache_cost: Some(0.30),
            capabilities: ModelCapabilities {
                vision: true,
                function_calling: true,
                streaming: true,
                reasoning: true,
            },
            base_url: None,
            thinking: true,
        }
    }

    /// Default GPT-4o
    pub fn default_gpt4o() -> Self {
        Self {
            id: "openai/gpt-4o".to_string(),
            name: "GPT-4o".to_string(),
            provider: "openai".to_string(),
            api: "openai-responses".to_string(),
            context_window: 128_000,
            max_tokens: 16384,
            input_cost: 2.50,
            output_cost: 10.00,
            cache_cost: None,
            capabilities: ModelCapabilities {
                vision: true,
                function_calling: true,
                streaming: true,
                reasoning: false,
            },
            base_url: None,
            thinking: false,
        }
    }

    /// Default Ollama (local, free)
    pub fn default_ollama() -> Self {
        Self {
            id: "ollama/llama3.3".to_string(),
            name: "Llama 3.3".to_string(),
            provider: "ollama".to_string(),
            api: "ollama-chat".to_string(),
            context_window: 8_000,
            max_tokens: 4096,
            input_cost: 0.0,
            output_cost: 0.0,
            cache_cost: None,
            capabilities: ModelCapabilities {
                vision: false,
                function_calling: false,
                streaming: true,
                reasoning: false,
            },
            base_url: Some("http://localhost:11434".to_string()),
            thinking: false,
        }
    }
}

/// Streaming options
#[derive(Debug, Clone)]
pub struct StreamingOptions {
    pub thinking_level: Option<crate::core::agent::ThinkingLevel>,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f32>,
    pub system_prompt: Option<String>,
}

/// Streaming delta
#[derive(Debug, Clone)]
pub enum StreamingDelta {
    Content(String),
    Thinking(String),
    ToolCall {
        call_id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolCallDelta {
        call_id: String,
        delta: String,
    },
    CostUpdate(Cost),
    Done(Cost),
}

/// Error types for AI operations
#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("API error: {0}")]
    Api(String),
    #[error("Authentication error: {0}")]
    Auth(String),
    #[error("Rate limit exceeded")]
    RateLimit,
    #[error("Context length exceeded")]
    ContextLengthExceeded,
    #[error("Network error: {0}")]
    Network(String),
    #[error("Timeout")]
    Timeout,
    #[error("Model not available: {0}")]
    ModelNotAvailable(String),
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

/// ModelProvider trait — inspired by pi's provider system
#[async_trait]
pub trait ModelProvider: Send + Sync {
    /// Get the provider name
    fn name(&self) -> &str;

    /// Check if this provider handles the given model
    fn handles(&self, model: &Model) -> bool;

    /// Stream a completion
    async fn stream(
        &self,
        model: &Model,
        messages: &[(String, String)],
        options: &StreamingOptions,
    ) -> Result<Pin<Box<dyn Stream<Item = StreamingDelta> + Send>>, AiError>;

    /// Complete (non-streaming)
    async fn complete(
        &self,
        model: &Model,
        messages: &[(String, String)],
        options: &StreamingOptions,
    ) -> Result<String, AiError>;

    /// Count tokens (rough estimate if API not available)
    fn count_tokens(&self, text: &str) -> usize {
        // Rough estimate: ~4 chars per token
        text.len() / 4
    }

    /// Get cost for a response
    fn calculate_cost(&self, model: &Model, input_tokens: usize, output_tokens: usize) -> Cost {
        model.estimate_cost(input_tokens, output_tokens)
    }
}

/// Provider registry
pub struct ProviderRegistry {
    providers: RwLock<Vec<Arc<dyn ModelProvider>>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: RwLock::new(Vec::new()),
        }
    }

    pub async fn register(&self, provider: Arc<dyn ModelProvider>) {
        self.providers.write().await.push(provider);
    }

    pub async fn get_provider(&self, model: &Model) -> Option<Arc<dyn ModelProvider>> {
        let providers = self.providers.read().await;
        providers.iter()
            .find(|p| p.handles(model))
            .cloned()
    }

    pub async fn complete(
        &self,
        model: &Model,
        messages: &[(String, String)],
        options: &StreamingOptions,
    ) -> Result<String, AiError> {
        let provider = self.get_provider(model).await
            .ok_or_else(|| AiError::ModelNotAvailable(model.id.clone()))?;
        provider.complete(model, messages, options).await
    }

    pub async fn stream(
        &self,
        model: &Model,
        messages: &[(String, String)],
        options: &StreamingOptions,
    ) -> Result<Pin<Box<dyn Stream<Item = StreamingDelta> + Send>>, AiError> {
        let provider = self.get_provider(model).await
            .ok_or_else(|| AiError::ModelNotAvailable(model.id.clone()))?;
        provider.stream(model, messages, options).await
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple in-memory model database (similar to pi's ModelRegistry)
pub struct ModelDatabase {
    pub models: HashMap<String, Model>,
    pub active_model: RwLock<Option<String>>,
}

impl ModelDatabase {
    pub fn new() -> Self {
        let mut models = HashMap::new();
        
        models.insert("anthropic/claude-sonnet-4".to_string(), Model::default_claude());
        models.insert("openai/gpt-4o".to_string(), Model::default_gpt4o());
        models.insert("ollama/llama3.3".to_string(), Model::default_ollama());
        
        Self {
            models,
            active_model: RwLock::new(None),
        }
    }

    pub fn get(&self, id: &str) -> Option<&Model> {
        self.models.get(id)
    }

    pub fn list(&self) -> Vec<&Model> {
        self.models.values().collect()
    }

    pub fn set_active(&self, id: &str) {
        // Would need async context
    }

    pub async fn set_active_async(&self, id: &str) {
        if self.models.contains_key(id) {
            *self.active_model.write().await = Some(id.to_string());
        }
    }

    pub async fn active(&self) -> Option<&Model> {
        let id = self.active_model.read().await;
        id.as_ref().and_then(|id| self.models.get(id))
    }
}

impl Default for ModelDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_calculation() {
        let model = Model::default_claude();
        let cost = model.estimate_cost(100_000, 50_000);
        
        // 100K input at $3/M = $0.30
        // 50K output at $15/M = $0.75
        // Total = $1.05
        assert!((cost.input_cost - 0.30).abs() < 0.01);
        assert!((cost.output_cost - 0.75).abs() < 0.01);
        assert!((cost.total_cost() - 1.05).abs() < 0.01);
    }

    #[test]
    fn test_model_defaults() {
        let claude = Model::default_claude();
        assert!(claude.capabilities.vision);
        assert!(claude.capabilities.reasoning);
        
        let gpt = Model::default_gpt4o();
        assert!(gpt.capabilities.vision);
        assert!(!gpt.capabilities.reasoning);
        
        let ollama = Model::default_ollama();
        assert_eq!(ollama.input_cost, 0.0);
        assert!(ollama.base_url.is_some());
    }

    #[tokio::test]
    async fn test_provider_registry() {
        let registry = ProviderRegistry::new();
        let models = registry.get_provider(&Model::default_claude()).await;
        // No providers registered yet
        assert!(models.is_none());
    }

    #[tokio::test]
    async fn test_model_database() {
        let db = ModelDatabase::new();
        
        assert!(db.get("anthropic/claude-sonnet-4").is_some());
        assert!(db.get("openai/gpt-4o").is_some());
        assert!(db.get("ollama/llama3.3").is_some());
        assert!(db.get("nonexistent").is_none());
        
        assert_eq!(db.list().len(), 3);
    }
}
