//! Config types re-exported from the canonical config module.
//!
//! The config file watcher has moved into `ConfigActor` in `crate::actors::config`.

pub use types::{config_path, Config, ConfigChange, TruncationSection};

mod types;
