//! Provider module — trait, registry, dialog, and config.
//!
//! Consolidates the former `provider.rs`, `provider_registry/`, `providers_dialog.rs`,
//! and provider credential persistence into a single coherent namespace.

pub mod config;
pub mod detection;
pub mod dialog;
mod provider_trait;
pub mod registry;
mod registry_data;
pub mod routing;

// Re-exports for ergonomic access at the crate root.
pub use config::{
    get_provider_config, list_configured_providers, remove_provider_config, save_provider_config,
    set_test_config_path, set_test_config_with_providers,
};
pub use provider_trait::{
    Provider, ProviderError, ProviderMetadata, ResponseChunk, RetryConfig, RetryPolicy,
    CONNECT_TIMEOUT, REQUEST_TIMEOUT,
};
pub use registry::{
    display_name, find_model, find_provider, find_provider_by_env_var, is_known_provider,
    known_providers, ModelMeta, ProviderMeta,
};
pub use registry::{
    is_mock_enabled, is_mock_onboarding, mock_model, set_mock_enabled, set_mock_onboarding,
};
pub use detection::{
    detect_provider, detect_provider_from_api_base, parse_model_prefix, KNOWN_API_BASES,
};
