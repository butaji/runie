//! `ConfigActor` — the single owner of `~/.runie/config.toml`.

mod actor;
mod file_helpers;
mod messages;
pub mod ractor_config;
#[cfg(test)]
mod tests;

// Ractor-based ConfigActor (recommended).
pub use ractor_config::{RactorConfigActor, RactorConfigHandle};

// Legacy ConfigActor using custom trait (deprecated).
#[deprecated(since = "0.3.0", note = "Use RactorConfigActor instead")]
pub use actor::ConfigActor;
pub use messages::{ConfigActorHandle, ConfigMsg};

/// Trait for config actor handles.
/// Both `ConfigActorHandle` and `RactorConfigHandle` implement this trait,
/// allowing generic code to work with either.
pub trait ConfigHandle: Send + Sync + Clone {
    /// Get the current config.
    fn get_config(&self) -> impl std::future::Future<Output = Option<crate::config::Config>> + Send;
    /// Get configured providers.
    fn get_configured_providers(
        &self,
    ) -> impl std::future::Future<Output = Option<Vec<(String, String, Vec<String>)>>> + Send;
}

impl ConfigHandle for ConfigActorHandle {
    async fn get_config(&self) -> Option<crate::config::Config> {
        self.get_config().await
    }
    async fn get_configured_providers(&self) -> Option<Vec<(String, String, Vec<String>)>> {
        self.get_configured_providers().await
    }
}

impl ConfigHandle for RactorConfigHandle {
    async fn get_config(&self) -> Option<crate::config::Config> {
        self.get_config().await
    }
    async fn get_configured_providers(&self) -> Option<Vec<(String, String, Vec<String>)>> {
        self.get_configured_providers().await
    }
}
