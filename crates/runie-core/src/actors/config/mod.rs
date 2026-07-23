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
