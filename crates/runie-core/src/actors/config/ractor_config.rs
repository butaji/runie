//! Ractor-based `ConfigActor` implementation.
//!
//! Migrated from custom Actor trait to ractor for consistency with the rest
//! of the actor system.

use std::path::PathBuf;
use std::sync::Mutex;

use notify_debouncer_mini::{new_debouncer, DebouncedEvent, DebouncedEventKind};
use ractor::{Actor, ActorProcessingErr, ActorRef};
use tokio::sync::mpsc;

use crate::actors::ractor_adapter::{spawn_ractor, EventBusBridge};
use crate::actors::Reply;
use crate::bus::EventBus;
use crate::config::Config;
use crate::event::Event;

use super::messages::ConfigMsg;

/// Ractor-based ConfigActor handle.
#[derive(Clone, Debug)]
pub struct RactorConfigHandle {
    inner: crate::actors::ractor_adapter::RactorHandle<ConfigMsg>,
}

impl RactorConfigHandle {
    pub fn new(inner: crate::actors::ractor_adapter::RactorHandle<ConfigMsg>) -> Self {
        Self { inner }
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
}

/// Ractor-based ConfigActor state.
pub struct RactorConfigActor {
    config: std::sync::Arc<std::sync::Mutex<Config>>,
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
                let config = self.config.lock().unwrap().clone();
                reply.send(config);
            }
            ConfigMsg::GetConfiguredProviders(reply) => {
                reply.send(self.list_configured_providers());
            }
        }
    }

    async fn load_and_emit(&self) {
        let config = Config::load_async(Some(self.path.clone())).await;
        {
            let mut guard = self.config.lock().unwrap();
            *guard = config.clone();
        }
        self.bus_bridge.publish(Event::ConfigLoaded { config: Box::new(config) });
    }

    async fn reload_and_emit(&self) {
        let new_config = Config::load_async(Some(self.path.clone())).await;
        let changed = {
            let mut guard = self.config.lock().unwrap();
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
            save_provider_to_path(&path, &name, &base_url, &api_key, &models)
        }).await;
        self.handle_write_result(result).await;
    }

    async fn remove_provider(&self, name: &str) {
        let path = self.path.clone();
        let name = name.to_owned();
        let result = tokio::task::spawn_blocking(move || remove_provider_from_path(&path, &name)).await;
        self.handle_write_result(result).await;
    }

    async fn set_default_model(&self, provider: &str, model: &str) {
        let path = self.path.clone();
        let provider = provider.to_owned();
        let model = model.to_owned();
        let result = tokio::task::spawn_blocking(move || {
            set_default_model_at_path(&path, &provider, &model)
        }).await;
        self.handle_write_result(result).await;
    }

    async fn set_provider_models(&self, name: &str, models: &[String]) {
        let path = self.path.clone();
        let name = name.to_owned();
        let models = models.to_vec();
        let result = tokio::task::spawn_blocking(move || set_provider_models_at_path(&path, &name, &models)).await;
        self.handle_write_result(result).await;
    }

    async fn set_theme(&self, name: &str) {
        let path = self.path.clone();
        let name = name.to_owned();
        let result = tokio::task::spawn_blocking(move || set_theme_at_path(&path, &name)).await;
        self.handle_write_result(result).await;
    }

    async fn set_vim_mode(&self, enabled: bool) {
        let path = self.path.clone();
        let result = tokio::task::spawn_blocking(move || set_vim_mode_at_path(&path, enabled)).await;
        self.handle_write_result(result).await;
    }

    async fn set_telemetry(&self, enabled: bool) {
        let path = self.path.clone();
        let result = tokio::task::spawn_blocking(move || set_telemetry_at_path(&path, enabled)).await;
        self.handle_write_result(result).await;
    }

    async fn set_truncation(&self, limits: &crate::config::TruncationSection) {
        let path = self.path.clone();
        let limits = limits.clone();
        let result = tokio::task::spawn_blocking(move || set_truncation_at_path(&path, &limits)).await;
        self.handle_write_result(result).await;
    }

    async fn set_thinking_level(&self, level: &crate::model::ThinkingLevel) {
        let path = self.path.clone();
        let level = level.clone();
        let result = tokio::task::spawn_blocking(move || set_thinking_level_at_path(&path, level)).await;
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
        let guard = self.config.lock().unwrap();
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
            if let Err(e) = block_watcher_loop(tx, path) {
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
        // Load config on start
        let config = Config::load_async(Some(path)).await;
        {
            let mut guard = self.config.lock().unwrap();
            *guard = config;
        }
        // Create watcher channel and spawn watcher thread
        let (tx, rx) = mpsc::channel(32);
        {
            let mut guard = self.watcher_tx.lock().unwrap();
            *guard = Some(tx.clone());
        }
        self.spawn_watcher(tx.clone());
        // Emit initial config
        self.emit_current_config();
        // Start watcher receiver task
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
    pub async fn spawn(
        bus: EventBus<Event>,
        test_path: Option<PathBuf>,
    ) -> (RactorConfigHandle, ractor::ActorCell) {
        let path = test_path.unwrap_or_else(crate::config::config_path);
        let actor = Self::new(bus.clone(), path.clone());
        let (handle, _join, cell) = spawn_ractor(None, actor, (bus, path)).await.unwrap();
        (RactorConfigHandle::new(handle), cell)
    }

    fn emit_current_config(&self) {
        let config_to_emit = {
            let guard = self.config.lock().unwrap();
            guard.clone()
        };
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
        config: &std::sync::Arc<std::sync::Mutex<Config>>,
        path: &PathBuf,
        bus: &EventBusBridge<Event>,
    ) {
        let new_config = Config::load_async(Some(path.clone())).await;
        let changed = {
            let mut guard = config.lock().unwrap();
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

// ── File helpers (sync, for use in spawn_blocking) ─────────────────────────────
fn save_provider_to_path(
    path: &PathBuf,
    name: &str,
    base_url: &str,
    api_key: &str,
    models: &[String],
) -> anyhow::Result<()> {
    let mut config = Config::load(Some(path));
    let provider_type = config
        .model_providers
        .get(name)
        .and_then(|p| p.provider_type.clone());
    config.model_providers.insert(
        name.into(),
        crate::config::ModelProvider {
            provider_type,
            base_url: base_url.into(),
            api_key: api_key.into(),
            models: models.into(),
        },
    );
    config.save_to(path)
}

fn remove_provider_from_path(path: &PathBuf, name: &str) -> anyhow::Result<()> {
    let mut config = Config::load(Some(path));
    config.model_providers.remove(name);
    config.save_to(path)
}

fn set_default_model_at_path(path: &PathBuf, provider: &str, model: &str) -> anyhow::Result<()> {
    let mut config = Config::load(Some(path));
    config.provider = Some(provider.into());
    config.model = None;
    config.models.default = Some(model.into());
    let mp = config
        .model_providers
        .entry(provider.into())
        .or_insert_with(default_empty_provider);
    if !mp.models.contains(&model.into()) && !model.is_empty() {
        mp.models.push(model.into());
        mp.models.sort();
    }
    config.save_to(path)
}

fn set_provider_models_at_path(path: &PathBuf, name: &str, models: &[String]) -> anyhow::Result<()> {
    let mut config = Config::load(Some(path));
    if let Some(mp) = config.model_providers.get_mut(name) {
        mp.models = models.to_vec();
    }
    config.save_to(path)
}

fn set_theme_at_path(path: &PathBuf, name: &str) -> anyhow::Result<()> {
    let mut config = Config::load(Some(path));
    config.theme = Some(name.to_owned());
    config.save_to(path)
}

fn set_vim_mode_at_path(path: &PathBuf, enabled: bool) -> anyhow::Result<()> {
    let mut config = Config::load(Some(path));
    config.ui.vim_mode = enabled;
    config.save_to(path)
}

fn set_telemetry_at_path(path: &PathBuf, enabled: bool) -> anyhow::Result<()> {
    let mut config = Config::load(Some(path));
    config.telemetry.enabled = enabled;
    config.save_to(path)
}

fn set_truncation_at_path(path: &PathBuf, limits: &crate::config::TruncationSection) -> anyhow::Result<()> {
    let mut config = Config::load(Some(path));
    config.truncation = limits.clone();
    config.save_to(path)
}

fn set_thinking_level_at_path(path: &PathBuf, level: crate::model::ThinkingLevel) -> anyhow::Result<()> {
    let mut config = Config::load(Some(path));
    config.thinking_level = level;
    config.save_to(path)
}

fn default_empty_provider() -> crate::config::ModelProvider {
    crate::config::ModelProvider {
        provider_type: None,
        base_url: String::new(),
        api_key: String::new(),
        models: Vec::new(),
    }
}

fn block_watcher_loop(tx: mpsc::Sender<ConfigMsg>, config_path: PathBuf) -> anyhow::Result<()> {
    let (file_tx, file_rx) = std::sync::mpsc::channel();
    let mut debouncer = new_debouncer(std::time::Duration::from_millis(300), file_tx)
        .map_err(|e| anyhow::anyhow!("Failed to create watcher: {e:?}"))?;
    if let Some(parent) = config_path.parent() {
        debouncer
            .watcher()
            .watch(parent, notify::RecursiveMode::NonRecursive)
            .map_err(|e| anyhow::anyhow!("Failed to watch config dir: {e:?}"))?;
    }
    while let Ok(Ok(events)) = file_rx.recv() {
        if should_handle_config_event(&events, &config_path) {
            let _ = tx.blocking_send(ConfigMsg::Reload);
        }
    }
    Ok(())
}

fn should_handle_config_event(events: &[DebouncedEvent], config_path: &PathBuf) -> bool {
    let touches_config = events.iter().any(|e| e.path == *config_path);
    let has_relevant_kind = events.iter().any(|e| {
        matches!(
            e.kind,
            DebouncedEventKind::Any | DebouncedEventKind::AnyContinuous
        )
    });
    touches_config && has_relevant_kind
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn spawn_and_load_config() {
        let bus = EventBus::<Event>::new(16);
        let temp_path = std::env::temp_dir().join("runie_test_config.toml");
        let (handle, cell) = RactorConfigActor::spawn(bus.clone(), Some(temp_path.clone())).await;
        let mut sub = bus.subscribe();

        // Wait for ConfigLoaded event
        let mut found = false;
        while let Ok(evt) = sub.recv().await {
            if matches!(evt, Event::ConfigLoaded { .. }) {
                found = true;
                break;
            }
        }
        assert!(found, "ConfigLoaded should be emitted on spawn");

        // Cleanup
        let _ = std::fs::remove_file(&temp_path);
    }

    #[tokio::test]
    async fn get_config_returns_config() {
        let bus = EventBus::<Event>::new(16);
        let temp_path = std::env::temp_dir().join("runie_test_config2.toml");
        let (handle, _cell) = RactorConfigActor::spawn(bus.clone(), Some(temp_path.clone())).await;

        // Give it time to load
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let config = handle.get_config().await;
        assert!(config.is_some(), "get_config should return Some");

        // Cleanup
        let _ = std::fs::remove_file(&temp_path);
    }
}
