//! Login config persistence — backward-compatibility re-exports.
//!
//! All provider credential management has been moved to `crate::provider`.
//! This module re-exports those functions for backward compatibility.

pub use crate::provider::config::config_path;
pub use crate::provider::config::get_provider_config;
pub use crate::provider::config::list_configured_providers;
pub use crate::provider::config::remove_provider_config;
pub use crate::provider::config::save_provider_config;
pub use crate::provider::config::set_test_config_path;
pub use crate::provider::config::set_test_config_with_providers;
pub use crate::provider::config::with_read_lock;
pub use crate::provider::config::with_write_lock;
