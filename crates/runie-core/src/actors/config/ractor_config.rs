//! Ractor-based `ConfigActor` implementation.
//!
//! Migrated from custom Actor trait to ractor for consistency with the rest
//! of the actor system.

use std::path::PathBuf;
use parking_lot::Mutex;

use ractor::{Actor, ActorProcessingErr, ActorRef};
use tokio::sync::mpsc;

use crate::actors::ractor_adapter::{spawn_ractor, EventBusBridge};
use crate::actors::ractor_adapter::Reply;
use crate::bus::EventBus;
use crate::config::{Config, TruncationSection};
use crate::event::Event;
use crate::model::ThinkingLevel;

use super::file_helpers;
use super::messages::ConfigMsg;

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
    pub fn try_send(&self, msg: ConfigMsg) -> Result<(), ractor::MessagingErr<ConfigMsg>> {
        self.inner.try_send(msg)
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
        let _ = self.inner.send(ConfigMsg::GetConfiguredProviders(Reply::new(tx))).await;
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
    pub async fn save_provider(&self, name: String, base_url: String, api_key: String, models: Vec<String>) {
        let _ = self.inner.send(ConfigMsg::SaveProvider { name, base_url, api_key, models }).await;
    }

    /// Remove a provider configuration.
    pub async fn remove_provider(&self, name: String) {
        let _ = self.inner.send(ConfigMsg::RemoveProvider { name }).await;
    }

    /// Persist the active provider/model as the default.
    pub async fn set_default_model(&self, provider: String, model: String) {
        let _ = self.inner.send(ConfigMsg::SetDefaultModel { provider, model }).await;
    }

    /// Update the saved model list for a provider.
    pub async fn set_provider_models(&self, name: String, models: Vec<String>) {
        let _ = self.inner.send(ConfigMsg::SetProviderModels { name, models }).await;
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
}

/// Ractor-based ConfigActor state.
pub struct RactorConfigActor {
    config: std::sync::Arc<Mutex<Config>>,
    path: PathBuf,
    bus_bridge: EventBusBridge<Event>,
    watcher_tx: Mutex<Option<mpsc::Sender<ConfigMsg>>>,
}

impl RactorConfigActor {
    fn new(bus: EventBus<Event>, path: PathBuf) -> Self {
        Self {
            config: std::sync::Arc::new(Mutex::new(Config::default())),
            path,
            bus_bridge: EventBusBridge::new(bus),
            watcher_tx: Mutex::new(None),
        }
    }

    async fn handle_msg(&self, msg: ConfigMsg) {
        match msg {
            ConfigMsg::Load => self.load_and_emit().await,
            ConfigMsg::Reload => self.reload_and_emit().await,
            ConfigMsg::SaveProvider { name, base_url, api_key, models } => {
                self.save_provider(&name, &base_url, &api_key, &models).await;
            }
            ConfigMsg::RemoveProvider { name } => self.remove_provider(&name).await,
            ConfigMsg::SetDefaultModel { provider, model } => {
                self.set_default_model(&provider, &model).await;
            }
            ConfigMsg::SetProviderModels { name, models } => {
                self.set_provider_models(&name, &models).await;
            }
            ConfigMsg::SetTheme { name } => self.set_theme(&name).await,
            ConfigMsg::SetVimMode { enabled } => self.set_vim_mode(enabled).await,
            ConfigMsg::SetTelemetry { enabled } => self.set_telemetry(enabled).await,
            ConfigMsg::SetTruncation { limits } => self.set_truncation(&limits).await,
            ConfigMsg::SetThinkingLevel { level } => self.set_thinking_level(&level).await,
            ConfigMsg::GetConfig(reply) => {
                let cfg = self.config.lock().clone();
                reply.send(cfg);
            }
            ConfigMsg::GetConfiguredProviders(reply) => {
                reply.send(self.list_configured_providers());
            }
        }
    }

    async fn load_and_emit(&self) {
        let config = Config::load_async(Some(self.path.clone())).await;
        {
            let mut guard = self.config.lock();
            *guard = config.clone();
        }
        self.bus_bridge.publish(Event::ConfigLoaded { config: Box::new(config) });
    }

    async fn reload_and_emit(&self) {
        let new_config = Config::load_async(Some(self.path.clone())).await;
        let changed = {
            let mut guard = self.config.lock();
            if new_config != *guard {
                *guard = new_config.clone();
                true
            } else {
                false
            }
        };
        if changed {
            self.bus_bridge.publish(Event::ConfigLoaded { config: Box::new(new_config) });
        }
    }

    async fn save_provider(&self, name: &str, base_url: &str, api_key: &str, models: &[String]) {
        let path = self.path.clone();
        let name = name.to_owned();
        let base_url = base_url.to_owned();
        let api_key = api_key.to_owned();
        let models = models.to_vec();
        let result = tokio::task::spawn_blocking(move || {
            file_helpers::save_provider_to_path(&path, &name, &base_url, &api_key, &models)
        }).await;
        self.handle_write_result(result).await;
    }

    async fn remove_provider(&self, name: &str) {
        let path = self.path.clone();
        let name = name.to_owned();
        let result = tokio::task::spawn_blocking(move || file_helpers::remove_provider_from_path(&path, &name)).await;
        self.handle_write_result(result).await;
    }

    async fn set_default_model(&self, provider: &str, model: &str) {
        let path = self.path.clone();
        let provider = provider.to_owned();
        let model = model.to_owned();
        let result = tokio::task::spawn_blocking(move || file_helpers::set_default_model_at_path(&path, &provider, &model)).await;
        self.handle_write_result(result).await;
    }

    async fn set_provider_models(&self, name: &str, models: &[String]) {
        let path = self.path.clone();
        let name = name.to_owned();
        let models = models.to_vec();
        let result = tokio::task::spawn_blocking(move || file_helpers::set_provider_models_at_path(&path, &name, &models)).await;
        self.handle_write_result(result).await;
    }

    async fn set_theme(&self, name: &str) {
        let path = self.path.clone();
        let name = name.to_owned();
        let result = tokio::task::spawn_blocking(move || file_helpers::set_theme_at_path(&path, &name)).await;
        self.handle_write_result(result).await;
    }

    async fn set_vim_mode(&self, enabled: bool) {
        let path = self.path.clone();
        let result = tokio::task::spawn_blocking(move || file_helpers::set_vim_mode_at_path(&path, enabled)).await;
        self.handle_write_result(result).await;
    }

    async fn set_telemetry(&self, enabled: bool) {
        let path = self.path.clone();
        let result = tokio::task::spawn_blocking(move || file_helpers::set_telemetry_at_path(&path, enabled)).await;
        self.handle_write_result(result).await;
    }

    async fn set_truncation(&self, limits: &TruncationSection) {
        let path = self.path.clone();
        let limits = limits.clone();
        let result = tokio::task::spawn_blocking(move || file_helpers::set_truncation_at_path(&path, &limits)).await;
        self.handle_write_result(result).await;
    }

    async fn set_thinking_level(&self, level: &ThinkingLevel) {
        let path = self.path.clone();
        let level = *level;
        let result = tokio::task::spawn_blocking(move || file_helpers::set_thinking_level_at_path(&path, level)).await;
        self.handle_write_result(result).await;
    }

    async fn handle_write_result(&self, result: Result<anyhow::Result<()>, tokio::task::JoinError>) {
        match result {
            Ok(Ok(())) => self.load_and_emit().await,
            Ok(Err(e)) => {
                tracing::error!("config write failed: {e:?}");
                self.bus_bridge.publish(Event::Error {
                    id: "config".to_owned(),
                    message: format!("Config write failed: {e}"),
                });
            }
            Err(thread_id) => {
                tracing::error!("config write task panicked in thread: {:?}", thread_id);
                self.bus_bridge.publish(Event::Error {
                    id: "config".to_owned(),
                    message: "Config write task panicked".to_owned(),
                });
            }
        }
    }

    fn list_configured_providers(&self) -> Vec<(String, String, Vec<String>)> {
        let guard = self.config.lock();
        let mut result: Vec<_> = guard
            .model_providers
            .iter()
            .map(|(name, p)| (name.clone(), p.base_url.clone(), p.models.clone()))
            .collect();
        result.sort_by(|a, b| a.0.cmp(&b.0));
        result
    }

    fn spawn_watcher(&self, tx: mpsc::Sender<ConfigMsg>) {
        let path = self.path.clone();
        std::thread::spawn(move || {
            if let Err(e) = file_helpers::block_watcher_loop(tx, path) {
                tracing::error!("config watcher failed: {e:?}");
            }
        });
    }
}

#[ractor::async_trait]
impl Actor for RactorConfigActor {
    type Msg = ConfigMsg;
    type State = ();
    type Arguments = (EventBus<Event>, PathBuf);

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        let (_bus, path) = args;
        let config = Config::load_async(Some(path)).await;
        {
            let mut guard = self.config.lock();
            *guard = config;
        }
        let (tx, rx) = mpsc::channel(32);
        {
            let mut guard = self.watcher_tx.lock();
            *guard = Some(tx.clone());
        }
        self.spawn_watcher(tx.clone());
        self.emit_current_config();
        self.spawn_watcher_task(rx).await;
        Ok(())
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        self.handle_msg(msg).await;
        Ok(())
    }
}

impl RactorConfigActor {
    /// Spawn a `RactorConfigActor` on the given event bus.
    pub async fn spawn(bus: EventBus<Event>, test_path: Option<PathBuf>) -> (RactorConfigHandle, ractor::ActorCell) {
        let path = test_path.unwrap_or_else(crate::config::config_path);
        let actor = Self::new(bus.clone(), path.clone());
        let (handle, _join, cell) = spawn_ractor(None, actor, (bus, path)).await.unwrap();
        (RactorConfigHandle::new(handle), cell)
    }

    fn emit_current_config(&self) {
        let config_to_emit = self.config.lock().clone();
        self.bus_bridge.publish(Event::ConfigLoaded { config: Box::new(config_to_emit) });
    }

    async fn spawn_watcher_task(&self, mut rx: mpsc::Receiver<ConfigMsg>) {
        let config_clone = self.config.clone();
        let path_clone = self.path.clone();
        let bus_clone = self.bus_bridge.clone();
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if matches!(msg, ConfigMsg::Reload) {
                    Self::handle_watcher_reload(&config_clone, &path_clone, &bus_clone).await;
                }
            }
        });
    }

    async fn handle_watcher_reload(
        config: &std::sync::Arc<Mutex<Config>>,
        path: &PathBuf,
        bus: &EventBusBridge<Event>,
    ) {
        let new_config = Config::load_async(Some(path.clone())).await;
        let changed = {
            let mut guard = config.lock();
            if new_config != *guard {
                *guard = new_config.clone();
                true
            } else {
                false
            }
        };
        if changed {
            bus.publish(Event::ConfigLoaded { config: Box::new(new_config) });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn spawn_and_load_config() {
        let bus = EventBus::<Event>::new(16);
        let temp_path = std::env::temp_dir().join("runie_test_config.toml");
        // Subscribe BEFORE spawning so we don't miss pre_start's ConfigLoaded
        let mut sub = bus.subscribe();
        let (_handle, _cell) = RactorConfigActor::spawn(bus.clone(), Some(temp_path.clone())).await;

        // Wait for ConfigLoaded with timeout to prevent hanging forever
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(5);
        let mut found = false;
        while !found && tokio::time::Instant::now() < deadline {
            match tokio::time::timeout(tokio::time::Duration::from_millis(100), sub.recv()).await {
                Ok(Ok(evt)) => {
                    if matches!(evt, Event::ConfigLoaded { .. }) {
                        found = true;
                    }
                }
                Ok(Err(_)) | Err(_) => break, // Channel closed or timeout
            }
        }
        assert!(found, "ConfigLoaded should be emitted on spawn");
        let _ = std::fs::remove_file(&temp_path);
    }

    #[tokio::test]
    async fn get_config_returns_config() {
        let bus = EventBus::<Event>::new(16);
        let temp_path = std::env::temp_dir().join("runie_test_config2.toml");
        let mut sub = bus.subscribe();
        let (handle, _cell) = RactorConfigActor::spawn(bus.clone(), Some(temp_path.clone())).await;

        // Wait for ConfigLoaded to ensure actor is ready
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
        let mut found = false;
        while !found && tokio::time::Instant::now() < deadline {
            match tokio::time::timeout(
                deadline - tokio::time::Instant::now(),
                sub.recv(),
            ).await {
                Ok(Ok(evt)) => {
                    if matches!(evt, Event::ConfigLoaded { .. }) {
                        found = true;
                    }
                }
                Ok(Err(_)) | Err(_) => break,
            }
        }
        assert!(found, "ConfigLoaded should be emitted");

        let config = handle.get_config().await;
        assert!(config.is_some(), "get_config should return Some");
        let _ = std::fs::remove_file(&temp_path);
    }
}
