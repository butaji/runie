//! Ractor-based `ConfigActor` handle.
//!
//! Migrated from custom Actor trait to ractor for consistency with the rest
//! of the actor system.

use std::path::PathBuf;

use ractor::ActorRef;

use crate::bus::EventBus;
use crate::config::{Config, McpServer, TruncationSection};
use crate::event::Event;
use crate::model::ThinkingLevel;

use super::messages::ConfigMsg;
use crate::config::ConfigScope;

/// Ractor-based ConfigActor handle.
/// API-compatible with `ConfigActorHandle` for drop-in replacement.
#[derive(Clone, Debug)]
pub struct RactorConfigHandle {
    inner: ActorRef<ConfigMsg>,
}

impl RactorConfigHandle {
    pub fn new(inner: ActorRef<ConfigMsg>) -> Self {
        Self { inner }
    }

    /// Send a message to the actor (fire-and-forget).
    pub async fn send(&self, msg: ConfigMsg) {
        let _ = self.inner.send_message(msg);
    }

    /// Try to send a message (non-blocking).
    pub fn try_send(&self, msg: ConfigMsg) -> Result<(), Box<ractor::MessagingErr<ConfigMsg>>> {
        self.inner.send_message(msg).map_err(Box::new)
    }

    /// Request the current in-memory config.
    pub async fn get_config(&self) -> Option<Config> {
        match self.inner.call(ConfigMsg::GetConfig, None).await {
            Ok(ractor::rpc::CallResult::Success(config)) => Some(config),
            _ => None,
        }
    }

    /// Request the list of configured providers.
    pub async fn get_configured_providers(&self) -> Option<Vec<(String, String, Vec<String>)>> {
        match self.inner.call(ConfigMsg::GetConfiguredProviders, None).await {
            Ok(ractor::rpc::CallResult::Success(v)) => Some(v),
            _ => None,
        }
    }

    /// Ask the actor to load config from disk.
    pub async fn load(&self) {
        let _ = self.inner.send_message(ConfigMsg::Load);
    }

    /// Ask the actor to reload config from disk.
    pub async fn reload(&self) {
        let _ = self.inner.send_message(ConfigMsg::Reload);
    }

    /// Save a provider configuration.
    pub async fn save_provider(
        &self,
        name: String,
        base_url: String,
        api_key: String,
        models: Vec<String>,
    ) {
        let _ = self
            .inner
            .send_message(ConfigMsg::SaveProvider {
                name,
                base_url,
                api_key,
                models,
            });
    }

    /// Remove a provider configuration.
    pub async fn remove_provider(&self, name: String) {
        let _ = self.inner.send_message(ConfigMsg::RemoveProvider { name });
    }

    /// Persist the active provider/model as the default.
    pub async fn set_default_model(&self, provider: String, model: String) {
        let _ = self
            .inner
            .send_message(ConfigMsg::SetDefaultModel { provider, model });
    }

    /// Update the saved model list for a provider.
    pub async fn set_provider_models(&self, name: String, models: Vec<String>) {
        let _ = self
            .inner
            .send_message(ConfigMsg::SetProviderModels { name, models });
    }

    /// Set the theme name.
    pub async fn set_theme(&self, name: String) {
        let _ = self.inner.send_message(ConfigMsg::SetTheme { name });
    }

    /// Set vim mode.
    pub async fn set_vim_mode(&self, enabled: bool) {
        let _ = self.inner.send_message(ConfigMsg::SetVimMode { enabled });
    }

    /// Set telemetry enabled.
    pub async fn set_telemetry(&self, enabled: bool) {
        let _ = self.inner.send_message(ConfigMsg::SetTelemetry { enabled });
    }

    /// Set truncation limits.
    pub async fn set_truncation(&self, limits: TruncationSection) {
        let _ = self.inner.send_message(ConfigMsg::SetTruncation { limits });
    }

    /// Set thinking level.
    pub async fn set_thinking_level(&self, level: ThinkingLevel) {
        let _ = self.inner.send_message(ConfigMsg::SetThinkingLevel { level });
    }

    /// Load layered config (global + project) and return the effective config.
    pub async fn load_layers(&self) -> Option<Config> {
        match self.inner.call(ConfigMsg::LoadLayers, None).await {
            Ok(ractor::rpc::CallResult::Success(config)) => Some(config),
            _ => None,
        }
    }

    /// Add or update an MCP server in the specified scope.
    ///
    /// Waits for the operation to complete before returning.
    pub async fn add_mcp_server(&self, scope: ConfigScope, name: String, server: McpServer) {
        let _ = self
            .inner
            .call(
                |tx| ConfigMsg::AddMcpServer {
                    scope,
                    name,
                    server,
                    reply: tx,
                },
                None,
            )
            .await;
    }

    /// Remove an MCP server from the specified scope.
    ///
    /// Waits for the operation to complete before returning.
    pub async fn remove_mcp_server(&self, scope: ConfigScope, name: String) {
        let _ = self
            .inner
            .call(
                |tx| ConfigMsg::RemoveMcpServer {
                    scope,
                    name,
                    reply: tx,
                },
                None,
            )
            .await;
    }

    /// List MCP servers in the specified scope.
    pub async fn list_mcp_servers(&self, scope: ConfigScope) -> Vec<(String, McpServer)> {
        match self.inner.call(|tx| ConfigMsg::ListMcpServers { scope, reply: tx }, None).await {
            Ok(ractor::rpc::CallResult::Success(v)) => v,
            _ => Vec::new(),
        }
    }
}

/// Ractor State for ConfigActor — holds all mutable state.
/// Mutated through `&mut state` in handlers.
pub struct ConfigActorState {
    pub cfg: Config,
    pub path: PathBuf,
    pub project_path: Option<PathBuf>,
    pub bus: EventBus<Event>,
}

impl ConfigActorState {
    pub(crate) fn emit(&self, event: Event) {
        self.bus.publish(event);
    }
}
