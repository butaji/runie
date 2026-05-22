#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used)]

pub mod provider;
pub mod unified_api;
pub mod token_usage;
pub mod providers;
pub mod model_registry;
pub mod tests;

pub use provider::Provider;
pub use unified_api::UnifiedApi;
pub use token_usage::TokenUsage;
pub use providers::GenAiProvider;
pub use model_registry::{ModelInfo, ModelRegistry};
