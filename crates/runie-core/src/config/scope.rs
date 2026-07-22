//! Config scope types.
//!
//! Defines the scope for config operations that can target global or project config.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

/// Config scope for operations that can target global or project config.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Display, EnumString, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ConfigScope {
    /// Global config (~/.runie/config.toml)
    #[default]
    #[strum(serialize = "global")]
    Global,
    /// Project config (.runie/config.toml)
    #[strum(serialize = "project")]
    Project,
}

impl ConfigScope {
    /// Returns true if this is the global scope.
    pub fn is_global(self) -> bool {
        matches!(self, ConfigScope::Global)
    }

    /// Returns true if this is the project scope.
    pub fn is_project(self) -> bool {
        matches!(self, ConfigScope::Project)
    }

    /// Convert to lowercase string for TOML/display.
    pub fn as_str(&self) -> &'static str {
        match self {
            ConfigScope::Global => "global",
            ConfigScope::Project => "project",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_scope_serialization() {
        assert_eq!(
            serde_json::to_string(&ConfigScope::Global).unwrap(),
            "\"global\""
        );
        assert_eq!(
            serde_json::to_string(&ConfigScope::Project).unwrap(),
            "\"project\""
        );
    }

    #[test]
    fn config_scope_deserialization() {
        assert_eq!(
            serde_json::from_str::<ConfigScope>("\"global\"").unwrap(),
            ConfigScope::Global
        );
        assert_eq!(
            serde_json::from_str::<ConfigScope>("\"project\"").unwrap(),
            ConfigScope::Project
        );
    }

    #[test]
    fn config_scope_round_trip() {
        for scope in [ConfigScope::Global, ConfigScope::Project] {
            let s = serde_json::to_string(&scope).unwrap();
            let parsed: ConfigScope = serde_json::from_str(&s).unwrap();
            assert_eq!(parsed, scope);
        }
    }

    #[test]
    fn config_scope_from_str() {
        use std::str::FromStr;
        assert_eq!(
            ConfigScope::from_str("global").unwrap(),
            ConfigScope::Global
        );
        assert_eq!(
            ConfigScope::from_str("project").unwrap(),
            ConfigScope::Project
        );
        assert!(ConfigScope::from_str("invalid").is_err());
    }

    #[test]
    fn config_scope_display() {
        assert_eq!(ConfigScope::Global.to_string(), "global");
        assert_eq!(ConfigScope::Project.to_string(), "project");
    }

    #[test]
    fn config_scope_default() {
        assert_eq!(ConfigScope::default(), ConfigScope::Global);
    }

    #[test]
    fn config_scope_as_str() {
        assert_eq!(ConfigScope::Global.as_str(), "global");
        assert_eq!(ConfigScope::Project.as_str(), "project");
    }
}
