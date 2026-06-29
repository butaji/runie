//! Typed messages for `ConfigActor`.

use crate::actors::ractor_adapter::Reply;
use crate::config::{Config, McpServer, TruncationSection};
use crate::model::ThinkingLevel;

/// Config scope for operations that can target global or project config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigScope {
    /// Global config (~/.runie/config.toml)
    Global,
    /// Project config (.runie/config.toml)
    Project,
}

/// Messages accepted by `ConfigActor`.
#[derive(Debug, Clone)]
pub enum ConfigMsg {
    /// Load config from disk and publish `Event::ConfigLoaded`.
    Load,
    /// Reload from disk, detect changes, and publish `Event::ConfigLoaded` if changed.
    Reload,
    /// Save or update a provider entry.
    SaveProvider {
        name: String,
        base_url: String,
        api_key: String,
        models: Vec<String>,
    },
    /// Remove a provider entry.
    RemoveProvider { name: String },
    /// Persist the active provider/model as the default.
    SetDefaultModel { provider: String, model: String },
    /// Update the saved model list for a provider.
    SetProviderModels { name: String, models: Vec<String> },
    /// Set the theme name.
    SetTheme { name: String },
    /// Set vim mode.
    SetVimMode { enabled: bool },
    /// Set telemetry enabled.
    SetTelemetry { enabled: bool },
    /// Set truncation limits.
    SetTruncation { limits: TruncationSection },
    /// Set thinking level.
    SetThinkingLevel { level: ThinkingLevel },
    /// Request the current in-memory config.
    GetConfig(Reply<Config>),
    /// Request the list of configured providers.
    GetConfiguredProviders(Reply<Vec<(String, String, Vec<String>)>>),
    /// Load layered config (global + project) and return the effective config.
    /// The actor merges both layers and emits ConfigLoaded with the effective config.
    LoadLayers(Reply<Config>),
    /// Add or update an MCP server in the specified scope.
    AddMcpServer { scope: ConfigScope, name: String, server: McpServer },
    /// Remove an MCP server from the specified scope.
    RemoveMcpServer { scope: ConfigScope, name: String },
    /// List MCP servers in the specified scope.
    ListMcpServers { scope: ConfigScope, reply: Reply<Vec<(String, McpServer)>> },
}
