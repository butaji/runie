//! Config types re-exported from canonical config.
//!
//! This module re-exports types from `crate::config` for backward compatibility
//! with code that imports from this module path.

pub use crate::config::{
    config_path, Config, ConfigChange, ModelsSection, ModelProvider, TruncationSection,
};

/// TelemetrySection is duplicated here for backward compatibility.
/// The canonical definition is in `crate::config::TelemetrySection`.
pub type TelemetrySection = crate::config::TelemetrySection;

/// PromptsSection is duplicated here for backward compatibility.
/// The canonical definition is in `crate::config::PromptsSection`.
pub type PromptsSection = crate::config::PromptsSection;

/// UiSection is duplicated here for backward compatibility.
/// The canonical definition is in `crate::config::UiSection`.
pub type UiSection = crate::config::UiSection;
