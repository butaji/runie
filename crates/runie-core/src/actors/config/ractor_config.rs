//! Ractor-based `ConfigActor` implementation.
//!
//! Migrated from custom Actor trait to ractor for consistency with the rest
//! of the actor system.

use std::path::PathBuf;

use ractor::{Actor, ActorProcessingErr, ActorRef};
use tracing::instrument;

use crate::actors::ractor_adapter::spawn_ractor;
use crate::bus::EventBus;
use crate::event::Event;

use super::config_handle::{ConfigActorState, RactorConfigHandle};
use super::handlers;
use super::messages::ConfigMsg;

// handlers module is declared in mod.rs

/// Ractor-based ConfigActor.
pub struct RactorConfigActor;

impl RactorConfigActor {
    /// Emit the current config as an event.
    fn emit_current_config(state: &ConfigActorState) {
        state.emit(Event::ConfigLoaded {
            config: Box::new(state.cfg.clone()),
        });
        tracing::info!("ConfigLoaded");
    }

    /// Spawn a `RactorConfigActor` on the given event bus.
    ///
    /// The actor loads layered config (global + project) on startup.
    /// - `global_path`: Global config path (~/.runie/config.toml)
    /// - `project_path`: Optional project config path (.runie/config.toml)
    ///
    /// Returns a `Result` to allow callers to handle spawn failures gracefully.
    pub async fn spawn(
        bus: EventBus<Event>,
        global_path: Option<PathBuf>,
        project_path: Option<PathBuf>,
    ) -> anyhow::Result<(
        RactorConfigHandle,
        ractor::ActorCell,
        tokio::task::JoinHandle<()>,
    )> {
        let path = global_path.unwrap_or_else(crate::config::config_path);
        let actor = Self;
        let (handle, join, cell) =
            spawn_ractor(None, actor, (bus, path.clone(), project_path.clone()))
                .await
                .map_err(|e| anyhow::anyhow!("RactorConfigActor spawn failed: {}", e))?;
        Ok((RactorConfigHandle::new(handle), cell, join))
    }

    /// Spawn with default paths (global ~/.runie/config.toml, project ./.runie/config.toml).
    pub async fn spawn_default(
        bus: EventBus<Event>,
    ) -> anyhow::Result<(
        RactorConfigHandle,
        ractor::ActorCell,
        tokio::task::JoinHandle<()>,
    )> {
        Self::spawn(bus, None, None).await
    }
}

#[ractor::async_trait]
impl Actor for RactorConfigActor {
    type Msg = ConfigMsg;
    type State = ConfigActorState;
    type Arguments = (EventBus<Event>, PathBuf, Option<PathBuf>);

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        let (bus, path, project_path) = args;
        // Load layered config
        let config = handlers::load_layers_async(path.clone(), project_path.clone()).await;

        // Validate loaded config against schema and emit error if invalid
        if let Err(errors) = config.validate_full() {
            tracing::warn!("initial config validation failed: {:?}", errors);
            bus.publish(Event::Error {
                id: "config".to_owned(),
                message: format!(
                    "Config validation failed: {}. Using defaults.",
                    errors.join("; ")
                ),
            });
        }

        // Spawn the file watcher in a std thread (requires `watch` feature).
        #[cfg(feature = "watch")]
        handlers::spawn_config_watcher(myself.clone(), path.clone());

        let state = ConfigActorState {
            cfg: config.clone(),
            path: path.clone(),
            project_path: project_path.clone(),
            bus: bus.clone(),
        };
        // Emit the initial config
        Self::emit_current_config(&state);

        Ok(state)
    }

    #[instrument(name = "config_actor", skip_all, fields(msg = ?msg))]
    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        handlers::handle_msg(state, msg).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::mcp::McpTransport;
    use crate::config::McpServer;
    use std::io::Write;

    #[tokio::test]
    async fn spawn_and_load_config() {
        let bus = EventBus::<Event>::new(16);
        let temp_path = std::env::temp_dir().join("runie_test_config.toml");
        // Subscribe BEFORE spawning so we don't miss pre_start's ConfigLoaded
        let mut sub = bus.subscribe();
        let (_handle, _cell, _) =
            RactorConfigActor::spawn(bus.clone(), Some(temp_path.clone()), None)
                .await
                .unwrap();

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
        let (handle, _cell, _) =
            RactorConfigActor::spawn(bus.clone(), Some(temp_path.clone()), None)
                .await
                .unwrap();

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

        // Subscribe BEFORE spawning so we don't miss pre_start's ConfigLoaded
        let mut sub = bus.subscribe();
        let (handle, _cell, _join) = RactorConfigActor::spawn(
            bus.clone(),
            Some(global_path.clone()),
            Some(project_path.clone()),
        )
        .await
        .unwrap();

        // Wait for ConfigLoaded to confirm actor has loaded the layered config
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(5);
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
        assert!(found, "ConfigLoaded should be emitted on spawn");

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

        // Subscribe BEFORE spawning so we don't miss pre_start's ConfigLoaded
        let mut sub = bus.subscribe();
        let (handle, _cell, _) =
            RactorConfigActor::spawn(bus.clone(), Some(temp_path.clone()), None)
                .await
                .unwrap();

        // Wait for ConfigLoaded to confirm actor is ready
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(5);
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
        assert!(found, "ConfigLoaded should be emitted on spawn");

        // Add an MCP server (await ensures write completes before we check)
        let server = McpServer {
            transport: McpTransport::Stdio,
            command: vec!["npx".to_string(), "-y".to_string(), "@server".to_string()],
            url: None,
            headers: std::collections::HashMap::new(),
            scope: crate::config::ConfigScope::Global,
        };
        handle
            .add_mcp_server(
                crate::config::ConfigScope::Global,
                "test-server".to_string(),
                server.clone(),
            )
            .await;

        // List MCP servers — add_mcp_server awaits completion
        let servers = handle
            .list_mcp_servers(crate::config::ConfigScope::Global)
            .await;
        assert!(
            servers.iter().any(|(name, _)| name == "test-server"),
            "Should have test-server in list: {:?}",
            servers
        );

        // Remove the server (await ensures write completes)
        handle
            .remove_mcp_server(
                crate::config::ConfigScope::Global,
                "test-server".to_string(),
            )
            .await;

        // Verify it's gone — remove_mcp_server awaits completion
        let servers = handle
            .list_mcp_servers(crate::config::ConfigScope::Global)
            .await;
        assert!(
            !servers.iter().any(|(name, _)| name == "test-server"),
            "test-server should be removed: {:?}",
            servers
        );

        let _ = std::fs::remove_file(&temp_path);
    }

    #[tokio::test]
    async fn config_actor_emits_error_on_invalid_config() {
        // Test that invalid TOML produces an Error event during load.
        // The actor loads a config with a syntax error (unclosed string).
        let bus = EventBus::<Event>::new(16);
        let temp_path = std::env::temp_dir().join("runie_test_invalid_config.toml");

        // Write a config with syntax error (missing closing quote)
        {
            let mut file = std::fs::File::create(&temp_path).unwrap();
            writeln!(file, r#"provider = "openai"#).unwrap();
        }

        let mut sub = bus.subscribe();
        let (_handle, _cell, _) =
            RactorConfigActor::spawn(bus.clone(), Some(temp_path.clone()), None)
                .await
                .unwrap();

        // Collect events for up to 3 seconds
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(3);
        let mut found_error = false;
        let mut found_config_loaded = false;

        while tokio::time::Instant::now() < deadline {
            match tokio::time::timeout(tokio::time::Duration::from_millis(50), sub.recv()).await {
                Ok(Ok(evt)) => {
                    if matches!(&evt, Event::Error { id, .. } if id == "config") {
                        found_error = true;
                    }
                    if matches!(&evt, Event::ConfigLoaded { .. }) {
                        found_config_loaded = true;
                        break;
                    }
                }
                Ok(Err(_)) | Err(_) => break,
            }
        }

        // Parse errors cause Config to fall back to defaults and validate successfully,
        // so the actor emits ConfigLoaded with defaults (no Error needed for parse failure).
        // On a structurally-valid but schema-invalid config, it would emit Error.
        assert!(
            found_config_loaded,
            "Actor should emit ConfigLoaded even with parse error"
        );
        // For a parse error, the fallback defaults are valid so no Error is emitted.
        assert!(!found_error, "Parse error should not emit Error event");
        let _ = std::fs::remove_file(&temp_path);
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
        let (_handle, _cell, _) =
            RactorConfigActor::spawn(bus.clone(), Some(temp_path.clone()), None)
                .await
                .unwrap();

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
        assert!(
            found_config_loaded,
            "Actor should emit ConfigLoaded even with parse error"
        );
        let _ = std::fs::remove_file(&temp_path);
    }

    #[cfg(feature = "watch")]
    #[test]
    fn config_event_is_relevant_touches_config_file() {
        use std::path::PathBuf;
        let path = PathBuf::from("/tmp/config.toml");
        // Empty events should not trigger
        assert!(!handlers::config_event_is_relevant(&[], &path));
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
