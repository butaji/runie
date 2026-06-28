//! `ConfigActor` — the single owner of `~/.runie/config.toml`.

mod actor;
mod messages;
pub mod ractor_config;
#[cfg(test)]
mod tests;

// Ractor-based ConfigActor (recommended).
pub use ractor_config::RactorConfigActor;

// Legacy ConfigActor using custom trait (deprecated).
#[deprecated(since = "0.3.0", note = "Use RactorConfigActor instead")]
pub use actor::ConfigActor;
pub use messages::{ConfigActorHandle, ConfigMsg};
