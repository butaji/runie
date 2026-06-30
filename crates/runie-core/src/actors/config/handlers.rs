//! Async message handlers for `RactorConfigActor`.
//!
//! Extracted from `ractor_config.rs` to satisfy the 500-line file limit.

use std::path::{Path, PathBuf};

use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEvent, DebouncedEventKind};

use crate::actors::ractor_adapter::Reply;
use crate::config::{Config, McpServer, TruncationSection};
use crate::event::Event;
use crate::model::ThinkingLevel;
use ractor::ActorRef;

use super::config_handle::ConfigActorState;
use super::file_helpers;
use super::messages::ConfigMsg;
use crate::config::ConfigScope;

/// Load layered config asynchronously.
pub(super) async fn load_layers_async(global: PathBuf, local: Option<PathBuf>) -> Config {
    tokio::task::spawn_blocking(move || {
        Config::load_layers_from_paths(global, local.unwrap_or_default())
    })
    .await
    .unwrap_or_default()
}

/// Load layered config synchronously (for use in spawn_blocking).
pub(super) fn load_layers_sync(global: &Path, local: &Option<PathBuf>) -> Config {
    Config::load_layers_from_paths(global.to_path_buf(), local.clone().unwrap_or_default())
}

/// Dispatch incoming config messages to their handlers.
pub(super) async fn handle_msg(state: &mut ConfigActorState, msg: ConfigMsg) {
    match msg {
        ConfigMsg::Load => load_and_emit(state).await,
        ConfigMsg::Reload => reload_and_emit(state).await,
        ConfigMsg::SaveProvider {
            name,
            base_url,
            api_key,
            models,
        } => {
            save_provider(state, &name, &base_url, &api_key, &models).await;
        }
        ConfigMsg::RemoveProvider { name } => remove_provider(state, &name).await,
        ConfigMsg::SetDefaultModel { provider, model } => {
            set_default_model(state, &provider, &model).await;
        }
        ConfigMsg::SetProviderModels { name, models } => {
            set_provider_models(state, &name, &models).await;
        }
        ConfigMsg::SetTheme { name } => set_theme(state, &name).await,
        ConfigMsg::SetVimMode { enabled } => set_vim_mode(state, enabled).await,
        ConfigMsg::SetTelemetry { enabled } => set_telemetry(state, enabled).await,
        ConfigMsg::SetTruncation { limits } => set_truncation(state, &limits).await,
        ConfigMsg::SetThinkingLevel { level } => set_thinking_level(state, &level).await,
        ConfigMsg::GetConfig(reply) => {
            let cfg = state.cfg.clone();
            reply.send(cfg);
        }
        ConfigMsg::GetConfiguredProviders(reply) => {
            reply.send(list_configured_providers(state));
        }
        ConfigMsg::LoadLayers(reply) => {
            let effective = load_layers_sync(&state.path, &state.project_path);
            reply.send(effective);
        }
        ConfigMsg::AddMcpServer {
            scope,
            name,
            server,
            reply,
        } => {
            add_mcp_server(state, scope, &name, server, reply).await;
        }
        ConfigMsg::RemoveMcpServer { scope, name, reply } => {
            remove_mcp_server(state, scope, &name, reply).await;
        }
        ConfigMsg::ListMcpServers { scope, reply } => {
            let servers = list_mcp_servers_from_state(state, scope);
            reply.send(servers);
        }
    }
}

/// Load config from disk and emit `ConfigLoaded`.
pub(super) async fn load_and_emit(state: &mut ConfigActorState) {
    let effective = load_layers_async(state.path.clone(), state.project_path.clone()).await;

    // Validate against the JSON schema. Emit error and keep defaults if invalid.
    if let Err(errors) = effective.validate_full() {
        tracing::warn!("initial config validation failed: {:?}", errors);
        state.emit(Event::Error {
            id: "config".to_owned(),
            message: format!(
                "Config validation failed: {}. Using defaults.",
                errors.join("; ")
            ),
        });
        // Keep the default config (already in state from pre_start), emit it.
        state.emit(Event::ConfigLoaded {
            config: Box::new(state.cfg.clone()),
        });
        return;
    }

    state.cfg = effective.clone();
    state.emit(Event::ConfigLoaded {
        config: Box::new(effective),
    });
}

/// Reload config from disk and emit `ConfigLoaded` if changed.
pub(super) async fn reload_and_emit(state: &mut ConfigActorState) {
    let new_config = load_layers_async(state.path.clone(), state.project_path.clone()).await;

    // Validate against the JSON schema. Keep previous valid config if invalid.
    if let Err(errors) = new_config.validate_full() {
        tracing::warn!("config reload validation failed: {:?}", errors);
        state.emit(Event::Error {
            id: "config".to_owned(),
            message: format!(
                "Config reload validation failed: {}. Keeping previous config.",
                errors.join("; ")
            ),
        });
        return;
    }

    let changed = new_config != state.cfg;
    if changed {
        state.cfg = new_config.clone();
        state.emit(Event::ConfigLoaded {
            config: Box::new(new_config),
        });
    }
}

/// Save a provider entry to disk.
pub(super) async fn save_provider(
    state: &mut ConfigActorState,
    name: &str,
    base_url: &str,
    api_key: &str,
    models: &[String],
) {
    let path = state.path.clone();
    let name = name.to_owned();
    let base_url = base_url.to_owned();
    let api_key = api_key.to_owned();
    let models = models.to_vec();
    let result = tokio::task::spawn_blocking(move || {
        file_helpers::save_provider_to_path(&path, &name, &base_url, &api_key, &models)
    })
    .await;
    handle_write_result(state, result).await;
}

/// Remove a provider entry from disk.
pub(super) async fn remove_provider(state: &mut ConfigActorState, name: &str) {
    let path = state.path.clone();
    let name = name.to_owned();
    let result = tokio::task::spawn_blocking(move || {
        file_helpers::remove_provider_from_path(&path, &name)
    })
    .await;
    handle_write_result(state, result).await;
}

/// Set the default model for a provider.
pub(super) async fn set_default_model(state: &mut ConfigActorState, provider: &str, model: &str) {
    let path = state.path.clone();
    let provider = provider.to_owned();
    let model = model.to_owned();
    let result = tokio::task::spawn_blocking(move || {
        file_helpers::set_default_model_at_path(&path, &provider, &model)
    })
    .await;
    handle_write_result(state, result).await;
}

/// Set the models list for a provider.
pub(super) async fn set_provider_models(state: &mut ConfigActorState, name: &str, models: &[String]) {
    let path = state.path.clone();
    let name = name.to_owned();
    let models = models.to_vec();
    let result = tokio::task::spawn_blocking(move || {
        file_helpers::set_provider_models_at_path(&path, &name, &models)
    })
    .await;
    handle_write_result(state, result).await;
}

/// Set the active theme.
pub(super) async fn set_theme(state: &mut ConfigActorState, name: &str) {
    let path = state.path.clone();
    let name = name.to_owned();
    let result =
        tokio::task::spawn_blocking(move || file_helpers::set_theme_at_path(&path, &name)).await;
    handle_write_result(state, result).await;
}

/// Set vim mode preference.
pub(super) async fn set_vim_mode(state: &mut ConfigActorState, enabled: bool) {
    let path = state.path.clone();
    let result =
        tokio::task::spawn_blocking(move || file_helpers::set_vim_mode_at_path(&path, enabled))
            .await;
    handle_write_result(state, result).await;
}

/// Set telemetry preference.
pub(super) async fn set_telemetry(state: &mut ConfigActorState, enabled: bool) {
    let path = state.path.clone();
    let result =
        tokio::task::spawn_blocking(move || file_helpers::set_telemetry_at_path(&path, enabled))
            .await;
    handle_write_result(state, result).await;
}

/// Set truncation limits.
pub(super) async fn set_truncation(state: &mut ConfigActorState, limits: &TruncationSection) {
    let path = state.path.clone();
    let limits = limits.clone();
    let result = tokio::task::spawn_blocking(move || {
        file_helpers::set_truncation_at_path(&path, &limits)
    })
    .await;
    handle_write_result(state, result).await;
}

/// Set thinking level.
pub(super) async fn set_thinking_level(state: &mut ConfigActorState, level: &ThinkingLevel) {
    let path = state.path.clone();
    let level = *level;
    let result = tokio::task::spawn_blocking(move || {
        file_helpers::set_thinking_level_at_path(&path, level)
    })
    .await;
    handle_write_result(state, result).await;
}

/// Handle the result of a blocking file write operation.
pub(super) async fn handle_write_result(
    state: &mut ConfigActorState,
    result: Result<anyhow::Result<()>, tokio::task::JoinError>,
) {
    match result {
        Ok(Ok(())) => load_and_emit(state).await,
        Ok(Err(e)) => {
            tracing::error!("config write failed: {e:?}");
            state.emit(Event::Error {
                id: "config".to_owned(),
                message: format!("Config write failed: {e}"),
            });
        }
        Err(thread_id) => {
            tracing::error!("config write task panicked in thread: {:?}", thread_id);
            state.emit(Event::Error {
                id: "config".to_owned(),
                message: "Config write task panicked".to_owned(),
            });
        }
    }
}

/// List configured providers from current state.
pub(super) fn list_configured_providers(
    state: &ConfigActorState,
) -> Vec<(String, String, Vec<String>)> {
    let mut result: Vec<_> = state
        .cfg
        .model_providers
        .iter()
        .map(|(name, p)| (name.clone(), p.base_url.clone(), p.models.clone()))
        .collect();
    result.sort_by(|a, b| a.0.cmp(&b.0));
    result
}

/// Add an MCP server configuration.
pub(super) async fn add_mcp_server(
    state: &mut ConfigActorState,
    scope: ConfigScope,
    name: &str,
    server: McpServer,
    reply: Reply<()>,
) {
    let path = path_for_scope(&state.path, &state.project_path, scope);
    let name = name.to_owned();
    let server = server.clone();
    let result = tokio::task::spawn_blocking(move || {
        file_helpers::add_mcp_server_to_path(&path, &name, &server)
    })
    .await;
    match result {
        Ok(Ok(())) => {
            reply.send(());
            load_and_emit(state).await;
        }
        Ok(Err(e)) => {
            tracing::error!("add mcp server failed: {:?}", e);
            state.emit(Event::Error {
                id: "config".to_owned(),
                message: format!("Failed to add MCP server: {e}"),
            });
            reply.send(());
        }
        Err(thread_id) => {
            tracing::error!("add mcp server task panicked: {:?}", thread_id);
            reply.send(());
        }
    }
}

/// Remove an MCP server configuration.
pub(super) async fn remove_mcp_server(
    state: &mut ConfigActorState,
    scope: ConfigScope,
    name: &str,
    reply: Reply<()>,
) {
    let path = path_for_scope(&state.path, &state.project_path, scope);
    let name = name.to_owned();
    let result = tokio::task::spawn_blocking(move || {
        file_helpers::remove_mcp_server_from_path(&path, &name)
    })
    .await;
    match result {
        Ok(Ok(())) => {
            reply.send(());
            load_and_emit(state).await;
        }
        Ok(Err(e)) => {
            tracing::error!("remove mcp server failed: {:?}", e);
            state.emit(Event::Error {
                id: "config".to_owned(),
                message: format!("Failed to remove MCP server: {e}"),
            });
            reply.send(());
        }
        Err(thread_id) => {
            tracing::error!("remove mcp server task panicked: {:?}", thread_id);
            reply.send(());
        }
    }
}

/// List MCP servers from config file for a given scope.
pub(super) fn list_mcp_servers_from_state(
    state: &ConfigActorState,
    scope: ConfigScope,
) -> Vec<(String, McpServer)> {
    let path = path_for_scope(&state.path, &state.project_path, scope);
    let config = Config::load(Some(&path));
    config.mcp.servers.into_iter().collect()
}

/// Resolve the config file path for a given scope.
pub(super) fn path_for_scope(
    global: &Path,
    project: &Option<PathBuf>,
    scope: ConfigScope,
) -> PathBuf {
    match scope {
        ConfigScope::Global => global.to_path_buf(),
        ConfigScope::Project => project.clone().unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_default()
                .join(".runie")
                .join("config.toml")
        }),
    }
}

/// Check if debounced events touch the config file with a relevant kind.
pub(super) fn config_event_is_relevant(events: &[DebouncedEvent], config_path: &PathBuf) -> bool {
    let touches_config = events.iter().any(|e| e.path == *config_path);
    let has_relevant_kind = events.iter().any(|e| {
        matches!(
            e.kind,
            DebouncedEventKind::Any | DebouncedEventKind::AnyContinuous
        )
    });
    touches_config && has_relevant_kind
}

/// Spawn the file watcher thread that watches config for changes.
pub(super) fn spawn_config_watcher(myself: ActorRef<ConfigMsg>, path: PathBuf) {
    let myself_clone = myself.clone();
    let path_clone = path.clone();

    std::thread::spawn(move || {
        let (tx, rx) = std::sync::mpsc::channel();
        let debouncer = match new_debouncer(std::time::Duration::from_millis(300), tx) {
            Ok(d) => d,
            Err(e) => {
                tracing::error!("config watcher: create debouncer failed: {e:?}");
                return;
            }
        };
        let mut debouncer = debouncer;
        if let Some(parent) = path_clone.parent() {
            if let Err(e) = debouncer
                .watcher()
                .watch(parent, RecursiveMode::NonRecursive)
            {
                tracing::error!("config watcher: watch {:?} failed: {e:?}", parent);
                return;
            }
        }
        while let Ok(Ok(events)) = rx.recv() {
            if config_event_is_relevant(&events, &path_clone) {
                let _ = myself_clone.cast(ConfigMsg::Reload);
            }
        }
    });
}
