#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used)]

pub mod provider;
pub mod unified_api;
pub mod token_usage;
pub mod providers;
pub mod model_registry;
pub mod tests;
pub mod helpers;
pub mod session_adapter;
pub mod model_fetcher;

pub use provider::Provider;
pub use unified_api::UnifiedApi;
pub use token_usage::TokenUsage;
pub use providers::GenAiProvider;
pub use providers::RigProvider;
pub use model_fetcher::{ModelFetcher, FetchError, create_fetcher, get_provider_models, ModelInfo};
pub use model_registry::ModelRegistry;
