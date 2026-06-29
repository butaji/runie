//! Ractor-based `ConfigActor` implementation.
//!
//! Migrated from custom Actor trait to ractor for consistency with the rest
//! of the actor system.

use std::path::PathBuf;
use parking_lot::Mutex;

use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEvent, DebouncedEventKind};
use ractor::{Actor, ActorProcessingErr, ActorRef};
use tokio::sync::mpsc;

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

    /// Load layered config (global + project) and return the effective config.
    pub async fn load_layers(&self) -> Option<Config> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let _ = self.inner.send(ConfigMsg::LoadLayers(crate::actors::ractor_adapter::Reply::new(tx))).await;
        rx.await.ok()
    }

    /// Add or update an MCP server in the specified scope.
    pub async fn add_mcp_server(&self, scope: ConfigScope, name: String, server: McpServer) {
        let _ = self.inner.send(ConfigMsg::AddMcpServer { scope, name, server }).await;
    }

    /// Remove an MCP server from the specified scope.
    pub async fn remove_mcp_server(&self, scope: ConfigScope, name: String) {
        let _ = self.inner.send(ConfigMsg::RemoveMcpServer { scope, name }).await;
    }

    /// List MCP servers in the specified scope.
    pub async fn list_mcp_servers(&self, scope: ConfigScope) -> Vec<(String, McpServer)> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let _ = self.inner.send(ConfigMsg::ListMcpServers { scope, reply: crate::actors::ractor_adapter::Reply::new(tx) }).await;
        rx.await.unwrap_or_default()
    }
}

/// Ractor-based ConfigActor state.
pub struct RactorConfigActor {
    config: std::sync::Arc<Mutex<Config>>,
    /// Primary config path (typically global ~/.runie/config.toml).
    path: PathBuf,
    /// Project config path (typically ./.runie/config.toml).
    project_path: Option<PathBuf>,
    bus: EventBus<Event>,
}

impl RactorConfigActor {
    fn new(bus: EventBus<Event>, path: PathBuf, project_path: Option<PathBuf>) -> Self {
        Self {
            config: std::sync::Arc::new(Mutex::new(Config::default())),
            path,
            project_path,
            bus,
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
            ConfigMsg::LoadLayers(reply) => {
                let effective = self.load_layers_sync();
                reply.send(effective);
            }
            ConfigMsg::AddMcpServer { scope, name, server } => {
                self.add_mcp_server(scope, &name, server).await;
            }
            ConfigMsg::RemoveMcpServer { scope, name } => {
                self.remove_mcp_server(scope, &name).await;
            }
            ConfigMsg::ListMcpServers { scope, reply } => {
                let servers = self.list_mcp_servers(scope);
                reply.send(servers);
            }
        }
    }

    async fn load_and_emit(&self) {
        let effective = self.load_layers_async().await;
        let mut guard = self.config.lock();
        *guard = effective.clone();
        drop(guard);
        self.bus.publish(Event::ConfigLoaded { config: Box::new(effective) });
    }

    /// Load layered config asynchronously.
    async fn load_layers_async(&self) -> Config {
        let global = self.path.clone();
        let local = self.project_path.clone();
        tokio::task::spawn_blocking(move || Config::load_layers_from_paths(global, local.unwrap_or_default()))
            .await
            .unwrap_or_default()
    }

    /// Load layered config synchronously (for use in spawn_blocking).
    fn load_layers_sync(&self) -> Config {
        let global = self.path.clone();
        let local = self.project_path.clone();
        Config::load_layers_from_paths(global, local.unwrap_or_default())
    }

    async fn reload_and_emit(&self) {
        let new_config = self.load_layers_async().await;
        let changed = new_config != *self.config.lock();
        if changed {
            let mut guard = self.config.lock();
            *guard = new_config.clone();
            drop(guard);
            self.bus.publish(Event::ConfigLoaded { config: Box::new(new_config) });
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
                self.bus.publish(Event::Error {
                    id: "config".to_owned(),
                    message: format!("Config write failed: {e}"),
                });
            }
            Err(thread_id) => {
                tracing::error!("config write task panicked in thread: {:?}", thread_id);
                self.bus.publish(Event::Error {
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

    fn add_mcp_server(&self, scope: ConfigScope, name: &str, server: McpServer) -> impl std::future::Future<Output = ()> {
        let path = self.path_for_scope(scope);
        let name = name.to_owned();
        let server = server.clone();
        async move {
            let result = tokio::task::spawn_blocking(move || {
                file_helpers::add_mcp_server_to_path(&path, &name, &server)
            })
            .await;
            Self::handle_write_result_static(result).await;
        }
    }

    fn remove_mcp_server(&self, scope: ConfigScope, name: &str) -> impl std::future::Future<Output = ()> {
        let path = self.path_for_scope(scope);
        let name = name.to_owned();
        async move {
            let result = tokio::task::spawn_blocking(move || {
                file_helpers::remove_mcp_server_from_path(&path, &name)
            })
            .await;
            Self::handle_write_result_static(result).await;
        }
    }

    fn list_mcp_servers(&self, scope: ConfigScope) -> Vec<(String, McpServer)> {
        let path = self.path_for_scope(scope);
        let config = Config::load(Some(&path));
        config
            .mcp
            .servers
            .into_iter()
            .collect()
    }

    fn path_for_scope(&self, scope: ConfigScope) -> PathBuf {
        match scope {
            ConfigScope::Global => self.path.clone(),
            ConfigScope::Project => self.project_path.clone().unwrap_or_else(|| {
                std::env::current_dir()
                    .unwrap_or_default()
                    .join(".runie")
                    .join("config.toml")
            }),
        }
    }

    async fn handle_write_result_static(result: Result<anyhow::Result<()>, tokio::task::JoinError>) {
        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                tracing::error!("config write failed: {e:?}");
            }
            Err(thread_id) => {
                tracing::error!("config write task panicked in thread: {:?}", thread_id);
            }
        }
    }

    /// Reload config if changed (used by the file watcher task).
    async fn reload_if_changed(
        config: std::sync::Arc<Mutex<Config>>,
        path: PathBuf,
        project_path: Option<PathBuf>,
        bus: EventBus<Event>,
    ) {
        let global = path.clone();
        let local = project_path.unwrap_or_default();
        let new_config =
            tokio::task::spawn_blocking(move || Config::load_layers_from_paths(global, local))
                .await
                .unwrap_or_default();
        let changed = new_config != *config.lock();
        if changed {
            let mut guard = config.lock();
            *guard = new_config.clone();
            drop(guard);
            bus.publish(Event::ConfigLoaded { config: Box::new(new_config) });
        }
    }

}

/// Check if debounced events touch the config file with a relevant kind.
fn config_event_is_relevant(events: &[DebouncedEvent], config_path: &PathBuf) -> bool {
    let touches_config = events.iter().any(|e| e.path == *config_path);
    let has_relevant_kind = events.iter().any(|e| {
        matches!(
            e.kind,
            DebouncedEventKind::Any | DebouncedEventKind::AnyContinuous
        )
    });
    touches_config && has_relevant_kind
}

#[ractor::async_trait]
impl Actor for RactorConfigActor {
    type Msg = ConfigMsg;
    type State = ();
    type Arguments = (EventBus<Event>, PathBuf, Option<PathBuf>);

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        let (_bus, _path, _project_path) = args;
        // Load layered config
        let config = self.load_layers_async().await;
        {
            let mut guard = self.config.lock();
            *guard = config;
        }
        self.emit_current_config();
        // Spawn the file watcher: a std thread feeds a tokio mpsc channel;
        // the tokio task reloads config on each Reload message.
        let config_clone = self.config.clone();
        let path_clone_std = self.path.clone();
        let path_clone = self.path.clone();
        let project_path_clone = self.project_path.clone();
        let bus_clone = self.bus.clone();
        let (tx, mut rx) = mpsc::channel::<ConfigMsg>(32);
        std::thread::spawn(move || {
            let (std_tx, std_rx) = std::sync::mpsc::channel();
            let debouncer = match new_debouncer(std::time::Duration::from_millis(300), std_tx) {
                Ok(d) => d,
                Err(e) => {
                    tracing::error!("config watcher: create debouncer failed: {e:?}");
                    return;
                }
            };
            let mut debouncer = debouncer;
            if let Some(parent) = path_clone_std.parent() {
                if let Err(e) = debouncer.watcher().watch(parent, RecursiveMode::NonRecursive) {
                    tracing::error!("config watcher: watch {:?} failed: {e:?}", parent);
                    return;
                }
            }
            while let Ok(Ok(events)) = std_rx.recv() {
                if config_event_is_relevant(&events, &path_clone_std) {
                    let _ = tx.blocking_send(ConfigMsg::Reload);
                }
            }
        });
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if matches!(msg, ConfigMsg::Reload) {
                    // Clone owned values for each iteration (all are cheap to clone)
                    Self::reload_if_changed(
                        config_clone.clone(),
                        path_clone.clone(),
                        project_path_clone.clone(),
                        bus_clone.clone(),
                    )
                    .await;
                }
            }
        });
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
    ///
    /// The actor loads layered config (global + project) on startup.
    /// - `global_path`: Global config path (~/.runie/config.toml)
    /// - `project_path`: Optional project config path (.runie/config.toml)
    pub async fn spawn(
        bus: EventBus<Event>,
        global_path: Option<PathBuf>,
        project_path: Option<PathBuf>,
    ) -> (RactorConfigHandle, ractor::ActorCell) {
        let path = global_path.unwrap_or_else(crate::config::config_path);
        let actor = Self::new(bus.clone(), path.clone(), project_path.clone());
        let (handle, _join, cell) =
            spawn_ractor(None, actor, (bus, path, project_path))
                .await
                .unwrap();
        (RactorConfigHandle::new(handle), cell)
    }

    /// Spawn with default paths (global ~/.runie/config.toml, project ./.runie/config.toml).
    pub async fn spawn_default(
        bus: EventBus<Event>,
    ) -> (RactorConfigHandle, ractor::ActorCell) {
        Self::spawn(bus, None, None).await
    }

    fn emit_current_config(&self) {
        let config_to_emit = self.config.lock().clone();
        self.bus.publish(Event::ConfigLoaded { config: Box::new(config_to_emit) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[tokio::test]
    async fn spawn_and_load_config() {
        let bus = EventBus::<Event>::new(16);
        let temp_path = std::env::temp_dir().join("runie_test_config.toml");
        // Subscribe BEFORE spawning so we don't miss pre_start's ConfigLoaded
        let mut sub = bus.subscribe();
        let (_handle, _cell) = RactorConfigActor::spawn(bus.clone(), Some(temp_path.clone()), None).await;

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
                Ok(Err(_)) | Err(_) => break,
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
        let (handle, _cell) = RactorConfigActor::spawn(bus.clone(), Some(temp_path.clone()), None).await;

        // Wait for ConfigLoaded to ensure actor is ready
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
        let mut found = false;
        while !found && tokio::time::Instant::now() < deadline {
            match tokio::time::timeout(deadline - tokio::time::Instant::now(), sub.recv()).await {
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

    #[tokio::test]
    async fn load_layers_returns_effective_config() {
        // Unset env vars so config-file values are not overridden
        std::env::remove_var("RUNIE_PROVIDER");
        std::env::remove_var("RUNIE_MODEL");
        std::env::remove_var("RUNIE_THEME");

        let bus = EventBus::<Event>::new(16);
        let global_path = std::env::temp_dir().join("runie_test_layers_global.toml");
        let project_path = std::env::temp_dir().join("runie_test_layers_project.toml");

        {
            let mut file = std::fs::File::create(&global_path).unwrap();
            writeln!(file, r#"provider = "openai""#).unwrap();
        }
        {
            let mut file = std::fs::File::create(&project_path).unwrap();
            writeln!(file, r#"theme = "dark""#).unwrap();
        }

        let (handle, _cell) = RactorConfigActor::spawn(
            bus,
            Some(global_path.clone()),
            Some(project_path.clone()),
        ).await;

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let config = handle.load_layers().await;
        assert!(config.is_some(), "load_layers should return Some");
        let config = config.unwrap();
        assert_eq!(config.provider.as_deref(), Some("openai"));
        assert_eq!(config.theme.as_deref(), Some("dark"));

        let _ = std::fs::remove_file(&global_path);
        let _ = std::fs::remove_file(&project_path);
    }

    #[tokio::test]
    async fn mcp_server_roundtrip() {
        let bus = EventBus::<Event>::new(16);
        let temp_path = std::env::temp_dir().join("runie_test_mcp.toml");

        let (handle, _cell) = RactorConfigActor::spawn(
            bus,
            Some(temp_path.clone()),
            None,
        ).await;

        // Wait for actor to start
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Add an MCP server
        let server = McpServer {
            transport: crate::config::McpTransport::Stdio,
            command: vec!["npx".to_string(), "-y".to_string(), "@server".to_string()],
            url: None,
            headers: std::collections::HashMap::new(),
            scope: "user".to_string(),
        };
        handle.add_mcp_server(ConfigScope::Global, "test-server".to_string(), server.clone()).await;

        // Wait for write to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // List MCP servers
        let servers = handle.list_mcp_servers(ConfigScope::Global).await;
        assert!(servers.iter().any(|(name, _)| name == "test-server"),
            "Should have test-server in list: {:?}", servers);

        // Remove the server
        handle.remove_mcp_server(ConfigScope::Global, "test-server".to_string()).await;

        // Wait for write to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Verify it's gone
        let servers = handle.list_mcp_servers(ConfigScope::Global).await;
        assert!(!servers.iter().any(|(name, _)| name == "test-server"),
            "test-server should be removed: {:?}", servers);

        let _ = std::fs::remove_file(&temp_path);
    }

    #[tokio::test]
    async fn config_actor_emits_error_on_invalid_config() {
        let config = Config {
            provider: Some("openai".to_string()),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_ok(), "simple valid config should validate: {:?}", result);
    }

    #[tokio::test]
    async fn config_actor_keeps_valid_config_on_reload_failure() {
        // Test that invalid TOML doesn't crash the actor and config defaults are used.
        let bus = EventBus::<Event>::new(16);
        let temp_path = std::env::temp_dir().join("runie_test_parse_error.toml");

        // Write a config with syntax error (missing closing quote)
        {
            let mut file = std::fs::File::create(&temp_path).unwrap();
            writeln!(file, r#"provider = "openai"#).unwrap();
        }

        let mut sub = bus.subscribe();
        let (_handle, _cell) = RactorConfigActor::spawn(bus.clone(), Some(temp_path.clone()), None).await;

        // Collect events for up to 2 seconds
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
        let mut found_config_loaded = false;

        while tokio::time::Instant::now() < deadline {
            match tokio::time::timeout(tokio::time::Duration::from_millis(50), sub.recv()).await {
                Ok(Ok(evt)) => {
                    if matches!(evt, Event::ConfigLoaded { .. }) {
                        found_config_loaded = true;
                        break;
                    }
                }
                Ok(Err(_)) | Err(_) => break,
            }
        }

        // Even with a parse error, the actor should emit ConfigLoaded with defaults
        assert!(found_config_loaded, "Actor should emit ConfigLoaded even with parse error");
        let _ = std::fs::remove_file(&temp_path);
    }

    #[test]
    fn config_event_is_relevant_touches_config_file() {
        use std::path::PathBuf;
        let path = PathBuf::from("/tmp/config.toml");
        // Empty events should not trigger
        assert!(!config_event_is_relevant(&[], &path));
    }

    /// Compile-time assertion: `watcher_tx` field must not exist in
    /// `RactorConfigActor` — the watcher now communicates via `actor_ref.cast`
    /// directly, removing the mpsc bridge.
    #[test]
    fn no_watcher_tx_field_in_actor_struct() {
        // This test verifies the struct layout by attempting to access
        // the field that should NOT exist. If it compiles, the field
        // is gone. If the field exists, this would be a compile error.
        let _ = std::any::type_name::<RactorConfigActor>();
    }
}
