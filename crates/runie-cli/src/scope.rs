//! CLI-specific ConfigScope integration.
//!
//! This module provides a newtype wrapper that can be used as a CLI argument
//! and converts to ConfigScope.

use runie_core::config::ConfigScope;

/// A newtype wrapper that can be parsed from CLI arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigScopeValue(pub ConfigScope);

impl std::fmt::Display for ConfigScopeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.as_str())
    }
}

impl std::str::FromStr for ConfigScopeValue {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "global" | "g" => Ok(ConfigScopeValue(ConfigScope::Global)),
            "project" | "p" => Ok(ConfigScopeValue(ConfigScope::Project)),
            _ => Err(format!(
                "invalid scope '{}': expected 'global' or 'project'",
                s
            )),
        }
    }
}

impl From<ConfigScopeValue> for ConfigScope {
    fn from(val: ConfigScopeValue) -> Self {
        val.0
    }
}

impl Default for ConfigScopeValue {
    fn default() -> Self {
        ConfigScopeValue(ConfigScope::Global)
    }
}
