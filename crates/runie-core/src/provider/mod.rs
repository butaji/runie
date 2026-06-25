//! Provider module — trait, registry, and dialog.
//!
//! Consolidates the former `provider.rs`, `provider_registry/`, and
//! `providers_dialog.rs` modules into a single coherent namespace.

pub mod dialog;
pub mod registry;
mod provider_trait;
mod registry_data;

// Re-exports for ergonomic access at the crate root.
pub use registry::{
    display_name, find_model, find_provider, find_provider_by_env_var, is_known_provider,
    known_providers, ModelMeta, ProviderMeta,
};
pub use registry::{is_mock_enabled, set_mock_enabled};
pub use provider_trait::{Provider, ProviderError, ResponseChunk};
