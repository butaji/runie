//! `ConfigActor` — the single owner of `~/.runie/config.toml`.

pub mod config_handle;
pub mod file_helpers;
mod handlers;
pub mod messages;
pub mod ractor_config;
#[cfg(test)]
mod tests;

// Ractor-based ConfigActor.
pub use config_handle::RactorConfigHandle;
pub use messages::ConfigMsg;
pub use ractor_config::RactorConfigActor;

/// Trait for config actor handles.
pub trait ConfigHandle: Send + Sync + Clone {
    fn get_config(&self) -> impl std::future::Future<Output = Option<crate::config::Config>> + Send;
    fn get_configured_providers(
        &self,
    ) -> impl std::future::Future<Output = Option<Vec<(String, String, Vec<String>)>>> + Send;
}

impl ConfigHandle for RactorConfigHandle {
    async fn get_config(&self) -> Option<crate::config::Config> {
        self.get_config().await
    }
    async fn get_configured_providers(&self) -> Option<Vec<(String, String, Vec<String>)>> {
        self.get_configured_providers().await
    }
}
