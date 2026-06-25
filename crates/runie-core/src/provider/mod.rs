//! Provider module — trait, registry, and dialog.
//!
//! Consolidates the former `provider.rs`, `provider_registry/`, and
//! `providers_dialog.rs` modules into a single coherent namespace.

pub mod dialog;
mod provider_trait;
pub mod registry;
mod registry_data;

// Re-exports for ergonomic access at the crate root.
pub use provider_trait::{Provider, ProviderError, ResponseChunk};
pub use registry::{
    display_name, find_model, find_provider, find_provider_by_env_var, is_known_provider,
    known_providers, ModelMeta, ProviderMeta,
};
pub use registry::{is_mock_enabled, set_mock_enabled};
