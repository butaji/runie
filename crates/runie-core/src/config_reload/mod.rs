//! Config file watcher for hot reload
//!
//! Watches config.toml for changes and emits SwitchModel events
//! when the provider or model configuration changes.

pub use types::{config_path, Config, TruncationSection};
pub use watcher::spawn_config_watcher;

mod types;
mod watcher;
