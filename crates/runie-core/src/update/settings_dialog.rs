//! Settings dialog — re-exports from settings module.
//!
//! This module is kept for backward compatibility while the settings
//! module is being consolidated.

pub use crate::settings::{
    build_setting_categories, build_setting_items, handle_settings_category, provider_model_lists,
};
