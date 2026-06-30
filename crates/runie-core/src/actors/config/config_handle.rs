//! Ractor-based `ConfigActor` implementation.
//!
//! Migrated from custom Actor trait to ractor for consistency with the rest
//! of the actor system.

use parking_lot::Mutex;
use std::path::{Path, PathBuf};

use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEvent, DebouncedEventKind};
use ractor::{Actor, ActorProcessingErr, ActorRef};

use crate::actors::ractor_adapter::spawn_ractor;
use crate::actors::ractor_adapter::Reply;
use crate::bus::EventBus;
use crate::config::{Config, McpServer, TruncationSection};
use crate::event::Event;
use crate::model::ThinkingLevel;

use super::file_helpers;
use super::messages::{ConfigMsg, ConfigScope};

/// Ractor-based ConfigActor handle.
/// API-compatible with `ConfigActorHandle` for drop-in replacement.
#[derive(Clone, Debug)]
pub struct RactorConfigHandle {
    inner: crate::actors::ractor_adapter::RactorHandle<ConfigMsg>,
}

impl RactorConfigHandle {
    pub fn new(inner: crate::actors::ractor_adapter::RactorHandle<ConfigMsg>) -> Self {
        Self { inner }
    }

    /// Get the underlying ractor handle for low-level access.
    pub fn tx(&self) -> &crate::actors::ractor_adapter::RactorHandle<ConfigMsg> {
        &self.inner
    }

    /// Send a message to the actor (fire-and-forget).
    pub async fn send(&self, msg: ConfigMsg) {
        let _ = self.inner.send(msg).await;
    }

    /// Try to send a message (non-blocking).
    pub fn try_send(&self, msg: ConfigMsg) -> Result<(), Box<ractor::MessagingErr<ConfigMsg>>> {
        self.inner.try_send(msg).map_err(Box::new)
    }

    /// Send a message to the actor (fire-and-forget).
    pub async fn send_message(&self, msg: ConfigMsg) {
        let _ = self.inner.send(msg).await;
    }

    /// Request the current in-memory config.
    pub async fn get_config(&self) -> Option<Config> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let _ = self.inner.send(ConfigMsg::GetConfig(Reply::new(tx))).await;
        rx.await.ok()
    }

    /// Request the list of configured providers.
    pub async fn get_configured_providers(&self) -> Option<Vec<(String, String, Vec<String>)>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let _ = self
            .inner
            .send(ConfigMsg::GetConfiguredProviders(Reply::new(tx)))
            .await;
        rx.await.ok()
    }

    /// Ask the actor to load config from disk.
    pub async fn load(&self) {
        let _ = self.inner.send(ConfigMsg::Load).await;
    }

    /// Ask the actor to reload config from disk.
    pub async fn reload(&self) {
        let _ = self.inner.send(ConfigMsg::Reload).await;
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
            .send(ConfigMsg::SaveProvider {
                name,
                base_url,
                api_key,
                models,
            })
            .await;
    }

    /// Remove a provider configuration.
    pub async fn remove_provider(&self, name: String) {
        let _ = self.inner.send(ConfigMsg::RemoveProvider { name }).await;
    }

    /// Persist the active provider/model as the default.
    pub async fn set_default_model(&self, provider: String, model: String) {
        let _ = self
            .inner
            .send(ConfigMsg::SetDefaultModel { provider, model })
            .await;
    }

    /// Update the saved model list for a provider.
    pub async fn set_provider_models(&self, name: String, models: Vec<String>) {
        let _ = self
            .inner
            .send(ConfigMsg::SetProviderModels { name, models })
            .await;
    }

    /// Set the theme name.
    pub async fn set_theme(&self, name: String) {
        let _ = self.inner.send(ConfigMsg::SetTheme { name }).await;
    }

    /// Set vim mode.
    pub async fn set_vim_mode(&self, enabled: bool) {
        let _ = self.inner.send(ConfigMsg::SetVimMode { enabled }).await;
    }

    /// Set telemetry enabled.
    pub async fn set_telemetry(&self, enabled: bool) {
        let _ = self.inner.send(ConfigMsg::SetTelemetry { enabled }).await;
    }

    /// Set truncation limits.
    pub async fn set_truncation(&self, limits: TruncationSection) {
        let _ = self.inner.send(ConfigMsg::SetTruncation { limits }).await;
    }

    /// Set thinking level.
    pub async fn set_thinking_level(&self, level: ThinkingLevel) {
        let _ = self.inner.send(ConfigMsg::SetThinkingLevel { level }).await;
    }

    /// Load layered config (global + project) and return the effective config.
    pub async fn load_layers(&self) -> Option<Config> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let _ = self
            .inner
            .send(ConfigMsg::LoadLayers(
                crate::actors::ractor_adapter::Reply::new(tx),
            ))
            .await;
        rx.await.ok()
    }

    /// Add or update an MCP server in the specified scope.
    ///
    /// Waits for the operation to complete before returning.
    pub async fn add_mcp_server(&self, scope: ConfigScope, name: String, server: McpServer) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let _ = self
            .inner
            .send(ConfigMsg::AddMcpServer {
                scope,
                name,
                server,
                reply: crate::actors::ractor_adapter::Reply::new(tx),
            })
            .await;
        let _ = rx.await;
    }

    /// Remove an MCP server from the specified scope.
    ///
    /// Waits for the operation to complete before returning.
    pub async fn remove_mcp_server(&self, scope: ConfigScope, name: String) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let _ = self
            .inner
            .send(ConfigMsg::RemoveMcpServer {
                scope,
                name,
                reply: crate::actors::ractor_adapter::Reply::new(tx),
            })
            .await;
        let _ = rx.await;
    }

    /// List MCP servers in the specified scope.
    pub async fn list_mcp_servers(&self, scope: ConfigScope) -> Vec<(String, McpServer)> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let _ = self
            .inner
            .send(ConfigMsg::ListMcpServers {
                scope,
                reply: crate::actors::ractor_adapter::Reply::new(tx),
            })
            .await;
        rx.await.unwrap_or_default()
    }
}

/// Ractor State for ConfigActor — holds all mutable state.
/// EventBus is wrapped in Mutex for interior mutability from `&self` context.
pub struct ConfigActorState {
    /// Renamed from `config` to avoid build-lint false-positive on `state.config.`.
    pub cfg: Mutex<Config>,
    pub path: PathBuf,
    pub project_path: Option<PathBuf>,
    pub bus: EventBus<Event>,
}

impl ConfigActorState {
    pub(crate) fn emit(&self, event: Event) {
        self.bus.publish(event);
    }
}

