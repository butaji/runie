//! Canonical config types for `~/.runie/config.toml`.
//!
//! This module defines the shared TOML schema that both `runie-core`
//! and `runie-provider` consume. It is the single source of truth
//! for the config file format.

use std::collections::HashMap;

use schemars::JsonSchema;

pub mod layers;
pub mod mcp;
pub mod migrate;
pub mod provider_config;
pub mod schema;
pub mod scope;
#[cfg(test)]
mod tests;

// Re-export ConfigScope for use in McpServer and CLI.
pub use scope::ConfigScope;

// Extracted Config impl to satisfy 500-line file limit.
mod config_impl;

pub use config_impl::ConfigChange;
pub use mcp::{McpSection, McpServer, McpTransport};

// Re-export config_path for convenience
pub use config_impl::config_path;

// ============================================================================
// Models Section
// ============================================================================

/// Models configuration section.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize, JsonSchema)]
pub struct ModelsSection {
    /// The default model to use when no model is specified.
    pub default: Option<String>,
    /// Scoped models list (for model selector UI).
    #[serde(default)]
    pub scoped: Option<Vec<String>>,
}

// ============================================================================
// Model Provider
// ============================================================================

/// A provider's configuration entry.
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct ModelProvider {
    #[serde(rename = "type")]
    pub provider_type: Option<String>,
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub models: Vec<String>,
}

// ============================================================================
// UI Section
// ============================================================================

/// UI configuration section.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[derive(JsonSchema)]
pub struct UiSection {
    pub vim_mode: bool,
}

impl Default for UiSection {
    fn default() -> Self {
        Self { vim_mode: true }
    }
}

// ============================================================================
// Telemetry Section
// ============================================================================

/// Telemetry configuration section.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[derive(JsonSchema)]
pub struct TelemetrySection {
    pub enabled: bool,
}

impl Default for TelemetrySection {
    fn default() -> Self {
        Self { enabled: true }
    }
}

// ============================================================================
// Prompts Section
// ============================================================================

/// Prompts configuration section.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[derive(JsonSchema)]
pub struct PromptsSection {
    pub default: Option<String>,
    pub custom: Option<String>,
}

// ============================================================================
// Truncation Section
// ============================================================================

/// Truncation limits for tool output.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[derive(JsonSchema)]
pub struct TruncationSection {
    pub max_lines: usize,
    pub max_bytes: usize,
}

impl Default for TruncationSection {
    fn default() -> Self {
        Self {
            max_lines: 2000,
            max_bytes: 50 * 1024,
        }
    }
}

// ============================================================================
// Hooks Section
// ============================================================================

/// Hook configuration.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[derive(JsonSchema)]
pub struct HooksConfig {
    /// Map of hook event name to list of shell commands to run.
    pub commands: HashMap<String, Vec<String>>,
}

// ============================================================================
// Permissions Section
// ============================================================================

/// Permissions configuration section.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[derive(JsonSchema)]
pub struct PermissionsSection {
    /// Permission mode: default, acceptEdits, auto, dontAsk, bypassPermissions, plan.
    pub mode: crate::permissions::PermissionMode,
    /// Explicit permission rules.
    pub rules: Vec<crate::permissions::PermissionRule>,
}

impl PermissionsSection {
    /// Get the default permissions section.
    pub fn default_section() -> Self {
        Self {
            mode: crate::permissions::PermissionMode::Default,
            rules: Vec::new(),
        }
    }

    /// Convert rules into a PermissionSet.
    pub fn to_permission_set(&self) -> crate::permissions::PermissionSet {
        crate::permissions::PermissionSet::new(self.rules.clone())
    }
}

// ============================================================================
// Main Config
// ============================================================================

/// Canonical config type for `~/.runie/config.toml`.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize, JsonSchema)]
pub struct Config {
    /// Default provider name.
    pub provider: Option<String>,
    /// Legacy model field (use `[models].default` instead).
    pub model: Option<String>,
    /// Theme name.
    pub theme: Option<String>,
    /// UI settings.
    #[serde(default)]
    pub ui: UiSection,
    /// Model configurations.
    #[serde(default)]
    pub models: ModelsSection,
    /// Provider configurations.
    #[serde(default)]
    pub model_providers: HashMap<String, ModelProvider>,
    /// Telemetry settings.
    #[serde(default)]
    pub telemetry: TelemetrySection,
    /// Prompt templates.
    #[serde(default)]
    pub prompts: PromptsSection,
    /// Truncation settings.
    #[serde(default)]
    pub truncation: TruncationSection,
    /// Thinking level for reasoning-intensive tasks.
    #[serde(default)]
    pub thinking_level: crate::model::ThinkingLevel,
    /// User-defined keybindings that override defaults.
    #[serde(default)]
    pub keybindings: HashMap<String, String>,
    /// Hook commands registered by event name.
    #[serde(default)]
    pub hooks: HooksConfig,
    /// Permission settings.
    #[serde(default)]
    pub permissions: PermissionsSection,
    /// MCP server configurations.
    #[serde(default)]
    pub mcp: mcp::McpSection,
}

#[cfg(test)]
mod telemetry_tests {
    use super::TelemetrySection;

    #[test]
    fn telemetry_section_default_enabled() {
        let section = TelemetrySection::default();
        assert!(section.enabled);
    }

    #[test]
    fn telemetry_can_be_disabled() {
        let section = TelemetrySection { enabled: false };
        assert!(!section.enabled);
    }
}
