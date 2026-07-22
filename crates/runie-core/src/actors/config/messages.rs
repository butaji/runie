//! Typed messages for `ConfigActor`.

use crate::config::{Config, McpServer, ModeSection, TruncationSection};
use crate::model::ThinkingLevel;
use ractor::RpcReplyPort;

// Re-export ConfigScope from config module for backwards compatibility.
pub use crate::config::ConfigScope;

/// Messages accepted by `ConfigActor`.
#[derive(Debug)]
pub enum ConfigMsg {
    /// Load config from disk and publish `Event::ConfigLoaded`.
    Load,
    /// Reload from disk, detect changes, and publish `Event::ConfigLoaded` if changed.
    Reload,
    /// Save or update a provider entry.
    SaveProvider { name: String, base_url: String, api_key: String, models: Vec<String> },
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
    /// Set the agent orchestration pattern section.
    SetMode { section: ModeSection },
    /// Set (or clear, with `None`) the per-model thinking level override for
    /// `provider/model`.
    SetModelThinking { provider: String, model: String, level: Option<ThinkingLevel> },
    /// Request the current in-memory config.
    GetConfig(RpcReplyPort<Config>),
    /// Request the list of configured providers.
    GetConfiguredProviders(RpcReplyPort<Vec<(String, String, Vec<String>)>>),
    /// Load layered config (global + project) and return the effective config.
    /// The actor merges both layers and emits ConfigLoaded with the effective config.
    LoadLayers(RpcReplyPort<Config>),
    /// Add or update an MCP server in the specified scope.
    AddMcpServer {
        scope: ConfigScope,
        name: String,
        server: McpServer,
        /// Optional reply channel. `Some(port)` for RPC callers; `None` for fire-and-forget.
        reply: Option<RpcReplyPort<()>>,
    },
    /// Remove an MCP server from the specified scope.
    RemoveMcpServer {
        scope: ConfigScope,
        name: String,
        /// Optional reply channel. `Some(port)` for RPC callers; `None` for fire-and-forget.
        reply: Option<RpcReplyPort<()>>,
    },
    /// List MCP servers in the specified scope.
    ListMcpServers {
        scope: ConfigScope,
        /// Optional reply channel. `Some(port)` for RPC callers; `None` for fire-and-forget.
        reply: Option<RpcReplyPort<Vec<(String, McpServer)>>>,
    },
}

impl Clone for ConfigMsg {
    #[allow(clippy::too_many_lines)]
    fn clone(&self) -> Self {
        match self {
            ConfigMsg::Load => ConfigMsg::Load,
            ConfigMsg::Reload => ConfigMsg::Reload,
            ConfigMsg::SaveProvider { name, base_url, api_key, models } => ConfigMsg::SaveProvider {
                name: name.clone(),
                base_url: base_url.clone(),
                api_key: api_key.clone(),
                models: models.clone(),
            },
            ConfigMsg::RemoveProvider { name } => ConfigMsg::RemoveProvider { name: name.clone() },
            ConfigMsg::SetDefaultModel { provider, model } => {
                ConfigMsg::SetDefaultModel { provider: provider.clone(), model: model.clone() }
            }
            ConfigMsg::SetProviderModels { name, models } => {
                ConfigMsg::SetProviderModels { name: name.clone(), models: models.clone() }
            }
            ConfigMsg::SetTheme { name } => ConfigMsg::SetTheme { name: name.clone() },
            ConfigMsg::SetVimMode { enabled } => ConfigMsg::SetVimMode { enabled: *enabled },
            ConfigMsg::SetTelemetry { enabled } => ConfigMsg::SetTelemetry { enabled: *enabled },
            ConfigMsg::SetTruncation { limits } => ConfigMsg::SetTruncation { limits: limits.clone() },
            ConfigMsg::SetThinkingLevel { level } => ConfigMsg::SetThinkingLevel { level: *level },
            ConfigMsg::SetMode { section } => ConfigMsg::SetMode { section: section.clone() },
            ConfigMsg::SetModelThinking { provider, model, level } => {
                ConfigMsg::SetModelThinking { provider: provider.clone(), model: model.clone(), level: *level }
            }
            ConfigMsg::GetConfig(_) => ConfigMsg::Load,
            ConfigMsg::GetConfiguredProviders(_) => ConfigMsg::Reload,
            ConfigMsg::LoadLayers(_) => ConfigMsg::Load,
            ConfigMsg::AddMcpServer { scope, name, server, .. } => {
                ConfigMsg::AddMcpServer {
                    scope: *scope,
                    name: name.clone(),
                    server: server.clone(),
                    reply: None, // Fire-and-forget; original reply not usable after move.
                }
            }
            ConfigMsg::RemoveMcpServer { scope, name, .. } => {
                ConfigMsg::RemoveMcpServer {
                    scope: *scope,
                    name: name.clone(),
                    reply: None, // Fire-and-forget.
                }
            }
            ConfigMsg::ListMcpServers { scope, .. } => ConfigMsg::ListMcpServers {
                scope: *scope,
                reply: None, // Fire-and-forget.
            },
        }
    }
}
