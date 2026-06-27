//! Provider registry.

pub mod registry_data;
pub mod registry;

pub use registry::{
    display_name, find_model, find_provider, find_provider_by_env_var, is_known_provider,
    known_providers, is_mock_enabled, set_mock_enabled, ModelMeta, ProviderMeta,
};
