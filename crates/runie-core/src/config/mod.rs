#![allow(clippy::too_many_lines)]

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
pub mod routing;
pub mod schema;
pub mod scope;
#[cfg(test)]
mod tests;

// Re-export routing types for use in runie-provider and other crates.
pub use routing::{ModelId, ModelRoutingConfig, RouterStrategy, RoutingStrategy};

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

/// Strategy for routing between multiple model deployments.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ModelRoutingStrategy {
    /// Round-robin through available deployments.
    #[default]
    SimpleShuffle,
    /// Route to the lowest-latency deployment.
    LatencyBased,
    /// Route to the cheapest deployment for the task.
    CostBased,
}

/// Models configuration section.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize, JsonSchema)]
pub struct ModelsSection {
    /// The default model to use when no model is specified.
    pub default: Option<String>,
    /// Scoped models list (for model selector UI).
    #[serde(default)]
    pub scoped: Option<Vec<String>>,
    /// Per-model thinking (reasoning) level overrides, keyed
    /// `"provider/model"`. Models without an entry inherit the global
    /// `thinking_level`.
    #[serde(default)]
    pub thinking: HashMap<String, crate::model::ThinkingLevel>,
    /// Routing strategy for multi-deployment providers.
    #[serde(default)]
    pub routing_strategy: ModelRoutingStrategy,
    /// Fallback models to use when context window is exceeded.
    /// Listed in order of preference (first available is used).
    #[serde(default)]
    pub context_window_fallback: Vec<String>,
}

// ============================================================================
// Model Provider
// ============================================================================

/// A provider's configuration entry.
/// API keys are resolved from environment variables or OS keyring, not stored here.
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize, JsonSchema)]
pub struct ModelProvider {
    #[serde(rename = "type")]
    pub provider_type: Option<String>,
    pub base_url: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub models: Vec<String>,
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub headers: std::collections::HashMap<String, String>,
    /// Fallback model IDs to try when the primary model cannot handle a request
    /// (e.g., context window exceeded). Ordered by priority.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context_window_fallbacks: Vec<ModelId>,
}

// ============================================================================
// UI Section
// ============================================================================

/// UI configuration section.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[derive(JsonSchema)]
pub struct UiSection {
    /// Enable vim-style navigation mode.
    pub vim_mode: bool,
    /// Maximum number of input history entries to retain.
    pub history_max_entries: usize,
    /// Number of items per page in list dialogs.
    pub page_size: usize,
}

impl Default for UiSection {
    fn default() -> Self {
        Self { vim_mode: true, history_max_entries: 1000, page_size: 5 }
    }
}

impl UiSection {
    /// Maximum number of input history entries to retain.
    pub fn history_max(&self) -> usize {
        self.history_max_entries
    }

    /// Number of items per page in list dialogs.
    pub fn page_size(&self) -> usize {
        self.page_size
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
// Mode Section
// ============================================================================

/// Agent orchestration pattern configuration (`[mode]` in config.toml).
///
/// See PATTERNS.md: `active` selects the orchestration pattern
/// (`single` | `swarm` | `improve`); the remaining fields bound
/// worker concurrency, iterations, per-task timeout, retries, and the
/// consecutive-failure circuit breaker.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[derive(JsonSchema)]
pub struct ModeSection {
    /// Active orchestration pattern: "single" | "swarm" | "improve".
    pub active: String,
    /// Max parallel workers.
    pub workers: usize,
    /// Max iterations (improve, swarm).
    pub max_rounds: usize,
    /// Per-task timeout in milliseconds.
    pub timeout_ms: u64,
    /// Retries per task on failure.
    pub max_retries: u32,
    /// Consecutive failures before fail-fast.
    pub circuit_breaker: u32,
    /// Lead (coordinator) model for swarm/delegation patterns.
    /// Format: "provider/model". Uses current model if None.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lead_model: Option<String>,
    /// Worker (task-executor) model for swarm/delegation patterns.
    /// Format: "provider/model". Uses current model if None.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worker_model: Option<String>,
}

impl Default for ModeSection {
    fn default() -> Self {
        Self {
            active: "single".into(),
            workers: 3,
            max_rounds: 5,
            timeout_ms: 120_000,
            max_retries: 2,
            circuit_breaker: 3,
            lead_model: None,
            worker_model: None,
        }
    }
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
        Self { max_lines: 2000, max_bytes: 50 * 1024 }
    }
}

// ── Tool Cache Section ────────────────────────────────────────────────────────

/// Tool result cache configuration.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[derive(JsonSchema)]
pub struct ToolCacheSection {
    /// TTL in seconds for cached tool results. Zero disables the cache.
    pub ttl_secs: u64,
}

impl Default for ToolCacheSection {
    fn default() -> Self {
        Self { ttl_secs: 300 } // 5 minutes
    }
}

// ── HTTP / Retry Section ────────────────────────────────────────────────────────

/// HTTP and retry configuration for provider network calls.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[derive(JsonSchema)]
pub struct HttpSection {
    /// Request timeout in seconds (default: 120).
    #[serde(default = "http_request_timeout_default")]
    pub request_timeout_secs: u64,
    /// Connection timeout in seconds (default: 10).
    #[serde(default = "http_connect_timeout_default")]
    pub connect_timeout_secs: u64,
}

fn http_request_timeout_default() -> u64 {
    120
}
fn http_connect_timeout_default() -> u64 {
    10
}

impl Default for HttpSection {
    fn default() -> Self {
        Self { request_timeout_secs: 120, connect_timeout_secs: 10 }
    }
}

/// Retry configuration for transient provider errors.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[derive(JsonSchema)]
pub struct RetrySection {
    /// Maximum number of retry attempts (default: 5).
    #[serde(default = "retry_max_attempts_default")]
    pub max_attempts: u32,
    /// Initial delay in milliseconds (default: 100).
    #[serde(default = "retry_initial_delay_default")]
    pub initial_delay_ms: u64,
    /// Maximum delay in milliseconds (default: 30000).
    #[serde(default = "retry_max_delay_default")]
    pub max_delay_ms: u64,
    /// Exponential backoff multiplier (default: 2.0).
    #[serde(default = "retry_multiplier_default")]
    pub multiplier: f64,
}

fn retry_max_attempts_default() -> u32 {
    5
}
fn retry_initial_delay_default() -> u64 {
    100
}
fn retry_max_delay_default() -> u64 {
    30_000
}
fn retry_multiplier_default() -> f64 {
    2.0
}

impl Default for RetrySection {
    fn default() -> Self {
        Self { max_attempts: 5, initial_delay_ms: 100, max_delay_ms: 30_000, multiplier: 2.0 }
    }
}

// ── FFF Search Section ────────────────────────────────────────────────────────

/// FFF full-text search configuration.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[derive(JsonSchema)]
pub struct FffSection {
    /// Scan timeout in seconds (default: 30).
    #[serde(default = "fff_scan_timeout_default")]
    pub scan_timeout_secs: u64,
    /// Default maximum number of results to return (default: 50).
    #[serde(default = "fff_default_limit_default")]
    pub default_limit: usize,
    /// Maximum file size in bytes to index (default: 2 MiB).
    #[serde(default = "fff_max_file_size_default")]
    pub max_file_size_bytes: usize,
}

fn fff_scan_timeout_default() -> u64 {
    30
}
fn fff_default_limit_default() -> usize {
    50
}
fn fff_max_file_size_default() -> usize {
    2 * 1024 * 1024
}

impl Default for FffSection {
    fn default() -> Self {
        Self { scan_timeout_secs: 30, default_limit: 50, max_file_size_bytes: 2 * 1024 * 1024 }
    }
}

// ============================================================================
// Sandbox Section
// ============================================================================

/// Sandbox configuration for OS-level command sandboxing.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[derive(JsonSchema)]
pub struct SandboxSection {
    /// Enable OS-level sandboxing for bash tool execution.
    /// When enabled, bash commands are wrapped in platform-specific sandbox.
    /// Defaults to false.
    pub enabled: bool,
}

// ============================================================================
// Skills Section
// ============================================================================

/// Skills configuration for skill discovery and management.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[derive(JsonSchema)]
pub struct SkillsConfig {
    /// Additional skill locations to load. Each entry is a `SKILL.md` file or
    /// a directory walked recursively. Supports `~` expansion.
    pub paths: Vec<String>,
    /// Path prefixes to exclude from skill discovery.
    pub ignore: Vec<String>,
    /// List of disabled skill names. Disabled skills are excluded from the
    /// system prompt and skill tool invocation but remain discoverable.
    pub disabled: Vec<String>,
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
        Self { mode: crate::permissions::PermissionMode::Default, rules: Vec::new() }
    }

    /// Convert rules into a PermissionSet.
    pub fn to_permission_set(&self) -> crate::permissions::PermissionSet {
        crate::permissions::PermissionSet::new(self.rules.clone())
    }
}

// ============================================================================
// Goal Section
// ============================================================================

/// Goal mode configuration section.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[derive(JsonSchema)]
pub struct GoalSection {
    /// Enable goal mode.
    pub enabled: bool,
    /// Max verification rounds before pausing.
    pub max_rounds: usize,
    /// Auto-verify after each task completion.
    pub auto_verify: bool,
    /// Token budget for goal execution (None = unlimited).
    pub token_budget: Option<u64>,
    /// Show goal pane in UI.
    pub show_pane: bool,
    /// Max checkpoints to track.
    pub max_checkpoints: usize,
}

impl Default for GoalSection {
    fn default() -> Self {
        Self {
            enabled: true,
            max_rounds: 10,
            auto_verify: true,
            token_budget: None,
            show_pane: true,
            max_checkpoints: 20,
        }
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
    /// Agent orchestration pattern settings.
    #[serde(default)]
    pub mode: ModeSection,
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
    /// Tool result cache settings.
    #[serde(default)]
    pub tool_cache: ToolCacheSection,
    /// HTTP timeouts for provider network calls.
    #[serde(default)]
    pub http: HttpSection,
    /// Retry policy for transient provider errors.
    #[serde(default)]
    pub retry: RetrySection,
    /// FFF full-text search settings.
    #[serde(default)]
    pub fff: FffSection,
    /// OS-level sandbox settings for bash tool execution.
    #[serde(default)]
    pub sandbox: SandboxSection,
    /// Skills configuration.
    #[serde(default)]
    pub skills: SkillsConfig,
    /// Goal mode configuration.
    #[serde(default)]
    pub goal: GoalSection,
}

#[cfg(test)]
mod tracing_tests {
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
