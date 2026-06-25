//! `ConfigActor` — sole owner of `~/.runie/config.toml`.

use std::path::{Path, PathBuf};

use notify_debouncer_mini::{new_debouncer, DebouncedEvent, DebouncedEventKind};
use tokio::sync::mpsc;

use crate::actors::{Actor, ActorHandle};
use crate::bus::EventBus;
use crate::config::Config;
use crate::event::Event;

use super::messages::{ConfigActorHandle, ConfigMsg};

/// Actor that owns the canonical in-memory config and all file IO for it.
pub struct ConfigActor {
    config: Config,
    path: PathBuf,
    tx: mpsc::Sender<ConfigMsg>,
}

impl ConfigActor {
    /// Spawn a `ConfigActor` on the given event bus.
    ///
    /// `test_path` overrides the canonical config path (used by tests).
    pub fn spawn(
        bus: EventBus<Event>,
        test_path: Option<PathBuf>,
    ) -> (ConfigActorHandle, ActorHandle) {
        let (tx, rx) = mpsc::channel(32);
        let path = test_path.unwrap_or_else(crate::login_config::config_path);
        let actor = Self {
            config: Config::default(),
            path,
            tx: tx.clone(),
        };
        let handle = ActorHandle::spawn(actor, rx, bus);
        (ConfigActorHandle::new(tx), handle)
    }

    fn path(&self) -> PathBuf {
        self.path.clone()
    }

    async fn handle_write_result(
        &mut self,
        result: Result<anyhow::Result<()>, tokio::task::JoinError>,
        bus: &EventBus<Event>,
    ) {
        match result {
            Ok(Ok(())) => self.load_and_emit(bus).await,
            Ok(Err(e)) => {
                tracing::error!("config write failed: {e:?}");
                bus.publish(Event::Error {
                    id: "config".to_owned(),
                    message: format!("Config write failed: {e}"),
                });
            }
            Err(e) => {
                tracing::error!("config write task panicked: {e:?}");
                bus.publish(Event::Error {
                    id: "config".to_owned(),
                    message: format!("Config write task panicked: {e}"),
                });
            }
        }
    }

    async fn load_and_emit(&mut self, bus: &EventBus<Event>) {
        self.config = Config::load_async(Some(self.path())).await;
        bus.publish(Event::ConfigLoaded {
            config: Box::new(self.config.clone()),
        });
    }

    async fn reload_and_emit(&mut self, bus: &EventBus<Event>) {
        let new_config = Config::load_async(Some(self.path())).await;
        if new_config != self.config {
            self.config = new_config.clone();
            bus.publish(Event::ConfigLoaded {
                config: Box::new(new_config),
            });
        }
    }

    /// Generic helper that spawns a blocking task to mutate config, then reloads on success.
    async fn mutate_config<F>(&mut self, bus: &EventBus<Event>, f: F)
    where
        F: FnOnce(std::path::PathBuf) -> anyhow::Result<()> + Send + 'static,
    {
        let path = self.path();
        let result = tokio::task::spawn_blocking(move || f(path)).await;
        self.handle_write_result(result, bus).await;
    }

    async fn save_provider(
        &mut self,
        name: &str,
        base_url: &str,
        api_key: &str,
        models: &[String],
        bus: &EventBus<Event>,
    ) {
        let name = name.to_owned();
        let base_url = base_url.to_owned();
        let api_key = api_key.to_owned();
        let models = models.to_vec();
        self.mutate_config(bus, move |path| {
            save_provider_to_path(&path, &name, &base_url, &api_key, &models)
        })
        .await;
    }

    async fn remove_provider(&mut self, name: &str, bus: &EventBus<Event>) {
        let name = name.to_owned();
        self.mutate_config(bus, move |path| remove_provider_from_path(&path, &name))
            .await;
    }

    async fn set_default_model(&mut self, provider: &str, model: &str, bus: &EventBus<Event>) {
        let provider = provider.to_owned();
        let model = model.to_owned();
        self.mutate_config(bus, move |path| {
            set_default_model_at_path(&path, &provider, &model)
        })
        .await;
    }

    async fn set_provider_models(&mut self, name: &str, models: &[String], bus: &EventBus<Event>) {
        let name = name.to_owned();
        let models = models.to_vec();
        self.mutate_config(bus, move |path| {
            set_provider_models_at_path(&path, &name, &models)
        })
        .await;
    }

    async fn set_theme(&mut self, name: String, bus: &EventBus<Event>) {
        let name = name.to_owned();
        self.mutate_config(bus, move |path| set_theme_at_path(&path, &name))
            .await;
    }

    async fn set_vim_mode(&mut self, enabled: bool, bus: &EventBus<Event>) {
        self.mutate_config(bus, move |path| set_vim_mode_at_path(&path, enabled)).await;
    }

    async fn set_telemetry(&mut self, enabled: bool, bus: &EventBus<Event>) {
        self.mutate_config(bus, move |path| set_telemetry_at_path(&path, enabled)).await;
    }

    async fn set_truncation(&mut self, limits: crate::config::TruncationSection, bus: &EventBus<Event>) {
        let limits = limits;
        self.mutate_config(bus, move |path| set_truncation_at_path(&path, &limits))
            .await;
    }

    async fn set_thinking_level(&mut self, level: crate::model::ThinkingLevel, bus: &EventBus<Event>) {
        let level = level;
        self.mutate_config(bus, move |path| set_thinking_level_at_path(&path, level))
            .await;
    }

    fn list_configured_providers(&self) -> Vec<(String, String, Vec<String>)> {
        let mut result: Vec<_> = self
            .config
            .model_providers
            .iter()
            .map(|(name, p)| (name.clone(), p.base_url.clone(), p.models.clone()))
            .collect();
        result.sort_by(|a, b| a.0.cmp(&b.0));
        result
    }

    fn spawn_watcher(&self) {
        let tx = self.tx.clone();
        let path = self.path();
        std::thread::spawn(move || {
            if let Err(e) = block_watcher_loop(tx, path) {
                tracing::error!("config watcher failed: {e:?}");
            }
        });
    }
}

impl Actor for ConfigActor {
    type Msg = ConfigMsg;
    type Event = Event;

    async fn run_body(mut self, mut rx: mpsc::Receiver<Self::Msg>, bus: EventBus<Event>) {
        self.load_and_emit(&bus).await;
        self.spawn_watcher();
        while let Some(msg) = rx.recv().await {
            self.handle_msg(msg, &bus).await;
        }
    }
}

impl ConfigActor {
    async fn handle_msg(&mut self, msg: ConfigMsg, bus: &EventBus<Event>) {
        match msg {
            ConfigMsg::Load => self.load_and_emit(bus).await,
            ConfigMsg::Reload => self.reload_and_emit(bus).await,
            ConfigMsg::SaveProvider {
                name,
                base_url,
                api_key,
                models,
            } => {
                self.save_provider(&name, &base_url, &api_key, &models, bus)
                    .await
            }
            ConfigMsg::RemoveProvider { name } => self.remove_provider(&name, bus).await,
            ConfigMsg::SetDefaultModel { provider, model } => {
                self.set_default_model(&provider, &model, bus).await;
            }
            ConfigMsg::SetProviderModels { name, models } => {
                self.set_provider_models(&name, &models, bus).await;
            }
            ConfigMsg::SetTheme { name } => self.set_theme(name, bus).await,
            ConfigMsg::SetVimMode { enabled } => self.set_vim_mode(enabled, bus).await,
            ConfigMsg::SetTelemetry { enabled } => self.set_telemetry(enabled, bus).await,
            ConfigMsg::SetTruncation { limits } => self.set_truncation(limits, bus).await,
            ConfigMsg::SetThinkingLevel { level } => self.set_thinking_level(level, bus).await,
            ConfigMsg::GetConfig(reply) => reply.send(self.config.clone()),
            ConfigMsg::GetConfiguredProviders(reply) => {
                reply.send(self.list_configured_providers());
            }
        }
    }
}

fn save_provider_to_path(
    path: &Path,
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

fn remove_provider_from_path(path: &Path, name: &str) -> anyhow::Result<()> {
    let mut config = Config::load(Some(path));
    config.model_providers.remove(name);
    config.save_to(path)
}

fn set_default_model_at_path(path: &Path, provider: &str, model: &str) -> anyhow::Result<()> {
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

fn set_provider_models_at_path(path: &Path, name: &str, models: &[String]) -> anyhow::Result<()> {
    let mut config = Config::load(Some(path));
    if let Some(mp) = config.model_providers.get_mut(name) {
        mp.models = models.to_vec();
    }
    config.save_to(path)
}

fn set_theme_at_path(path: &Path, name: &str) -> anyhow::Result<()> {
    let mut config = Config::load(Some(path));
    config.theme = Some(name.to_owned());
    config.save_to(path)
}

fn set_vim_mode_at_path(path: &Path, enabled: bool) -> anyhow::Result<()> {
    let mut config = Config::load(Some(path));
    config.ui.vim_mode = enabled;
    config.save_to(path)
}

fn set_telemetry_at_path(path: &Path, enabled: bool) -> anyhow::Result<()> {
    let mut config = Config::load(Some(path));
    config.telemetry.enabled = enabled;
    config.save_to(path)
}

fn set_truncation_at_path(path: &Path, limits: &crate::config::TruncationSection) -> anyhow::Result<()> {
    let mut config = Config::load(Some(path));
    config.truncation = limits.clone();
    config.save_to(path)
}

fn set_thinking_level_at_path(path: &Path, level: crate::model::ThinkingLevel) -> anyhow::Result<()> {
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

fn should_handle_config_event(events: &[DebouncedEvent], config_path: &Path) -> bool {
    let touches_config = events.iter().any(|e| e.path == *config_path);
    let has_relevant_kind = events.iter().any(|e| {
        matches!(
            e.kind,
            DebouncedEventKind::Any | DebouncedEventKind::AnyContinuous
        )
    });
    touches_config && has_relevant_kind
}
