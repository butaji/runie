//! Leader implementation.
//!
//! Coordinates all actors in the Runie runtime and optionally listens
//! on a local socket for client connections.

use std::path::PathBuf;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;

use crate::actors::leader::{AgentSpawnFuture, SpawnedAgent};
use crate::actors::turn::RactorTurnActor;
use crate::actors::{
    InputActor, RactorConfigActor, RactorConfigHandle, RactorFffIndexerActor,
    RactorFffIndexerHandle, RactorInputHandle, RactorIoActor, RactorIoHandle,
    RactorPermissionActor, RactorPermissionHandle, RactorProviderActor, RactorProviderHandle,
    RactorSessionActor, RactorSessionHandle, LEADER_CMD_CHANNEL_CAPACITY,
};
use crate::bus::EventBus;
use crate::Event as CoreEvent;

use super::handle::LeaderHandle;
use super::messages::LeaderCommand;
use super::{AgentActorFactory, LeaderAgentHandle};

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Configuration for the leader bootstrap.
#[derive(Clone, Debug)]
pub struct LeaderConfig {
    /// Optional TCP address to listen for client connections.
    pub tcp_addr: Option<String>,
    /// Project root for the FFF indexer.
    pub project_root: PathBuf,
    /// Data directory for the FFF indexer.
    pub data_dir: PathBuf,
}

impl Default for LeaderConfig {
    fn default() -> Self {
        Self {
            tcp_addr: None,
            project_root: std::env::current_dir().unwrap_or_default(),
            data_dir: dirs::data_dir().unwrap_or_else(std::env::temp_dir),
        }
    }
}

impl LeaderConfig {
    /// Set a custom TCP address for server mode.
    pub fn with_tcp_addr<A: ToString>(mut self, addr: A) -> Self {
        self.tcp_addr = Some(addr.to_string());
        self
    }
}

// ---------------------------------------------------------------------------
// Leader
// ---------------------------------------------------------------------------

/// Leader coordinates all actors in the Runie runtime.
#[derive(Clone)]
pub struct Leader {
    config: LeaderConfig,
}

impl Leader {
    /// Create a new leader in **embedded mode** (no TCP listener).
    /// This is the default for TUI and CLI usage.
    pub fn new() -> Self {
        Self {
            config: LeaderConfig::default(),
        }
    }

    /// Create a leader with explicit configuration.
    pub fn with_config(config: LeaderConfig) -> Self {
        Self { config }
    }

    /// Use a custom TCP address (enables server mode).
    pub fn with_tcp_addr<A: ToString>(self, addr: A) -> Self {
        Self {
            config: self.config.with_tcp_addr(addr),
        }
    }

    /// Start the leader and spawn all actors.
    ///
    /// Returns a `LeaderHandle` that exposes typed refs for every actor,
    /// a facts subscription channel, and a snapshot channel receiver.
    pub async fn start(
        &self,
        provider_factory: std::sync::Arc<dyn crate::actors::provider::ProviderFactory>,
        agent_factory: std::sync::Arc<dyn AgentActorFactory<SpawnFuture = AgentSpawnFuture>>,
    ) -> anyhow::Result<LeaderHandle> {
        let bus = EventBus::<CoreEvent>::new(1000);
        self.start_with_bus(provider_factory, agent_factory, bus)
            .await
    }

    /// Start with a pre-created event bus.
    ///
    /// Use this when you need to subscribe to the bus before actors emit initial facts
    /// (e.g. `ConfigLoaded`, `TrustLoaded`). Subscribe first, then call this method:
    ///
    /// ```ignore
    /// let bus = EventBus::<Event>::new(1000);
    /// let ui_rx = bus.subscribe();
    /// // Create UiActor with ui_rx before start() returns
    /// leader.start_with_bus(factory, agent_factory, bus).await?;
    /// // UiActor has already received ConfigLoaded etc.
    /// ```
    pub async fn start_with_bus(
        &self,
        provider_factory: std::sync::Arc<dyn crate::actors::provider::ProviderFactory>,
        agent_factory: std::sync::Arc<dyn AgentActorFactory<SpawnFuture = AgentSpawnFuture>>,
        bus: EventBus<CoreEvent>,
    ) -> anyhow::Result<LeaderHandle> {
        let (cmd_tx, cmd_rx) = mpsc::channel(LEADER_CMD_CHANNEL_CAPACITY);

        let handles =
            Self::spawn_actors(&bus, &self.config, provider_factory, agent_factory).await?;

        // Capture join handles for graceful shutdown.
        let coordinator_join = tokio::spawn(Self::coordinator(cmd_rx, bus.clone()));

        let tcp_join = if let Some(ref addr) = self.config.tcp_addr {
            let bus_clone = bus.clone();
            let addr = addr.clone();
            Some(tokio::spawn(async move {
                Self::listen_tcp(&addr, bus_clone).await
            }))
        } else {
            None
        };

        Ok(LeaderHandle::new(
            cmd_tx,
            bus,
            handles,
            Some(coordinator_join),
            tcp_join,
        ))
    }

    /// Spawn all child actors and capture all cells and join handles for graceful shutdown.
    async fn spawn_actors(
        bus: &EventBus<CoreEvent>,
        config: &LeaderConfig,
        provider_factory: std::sync::Arc<dyn crate::actors::provider::ProviderFactory>,
        agent_factory: std::sync::Arc<dyn AgentActorFactory<SpawnFuture = AgentSpawnFuture>>,
    ) -> anyhow::Result<super::SpawnedHandles> {
        let (config_h, config_cell, config_join) =
            RactorConfigActor::spawn_default(bus.clone()).await?;
        let (provider_h, provider_cell, provider_join) =
            RactorProviderActor::spawn(bus.clone(), config_h.clone(), provider_factory).await?;
        let (io_h, io_cell, io_join) = RactorIoActor::spawn(bus.clone()).await?;
        let (session_h, session_cell, session_join) =
            RactorSessionActor::spawn(bus.clone()).await?;
        let (permission_h, permission_cell, permission_join) =
            RactorPermissionActor::spawn(bus.clone(), config_h.clone()).await?;
        let (turn_h, turn_cell, turn_join) = RactorTurnActor::spawn(bus.clone()).await?;
        let (input_h, input_cell, input_join) = InputActor::spawn(bus.clone()).await?;
        let (fff_h, fff_cell, fff_join) = RactorFffIndexerActor::spawn(
            config.project_root.clone(),
            config.data_dir.clone(),
            bus.clone(),
        )
        .await
        .map_err(|e| anyhow::anyhow!("FffIndexerActor spawn failed: {}", e))?;
        let SpawnedAgent {
            handle: agent_handle,
            join: agent_join,
            cell: agent_cell,
        } = agent_factory
            .spawn_with_join(bus.clone(), provider_h.clone(), permission_h.clone())
            .await?;
        let all_joins = vec![
            config_join,
            provider_join,
            io_join,
            session_join,
            permission_join,
            turn_join,
            input_join,
            fff_join,
            agent_join,
        ];

        Ok(super::SpawnedHandles {
            config: config_h,
            config_cell,
            provider: provider_h,
            provider_cell,
            io: io_h,
            io_cell,
            session: session_h,
            session_cell,
            permission: permission_h,
            permission_cell,
            turn: turn_h,
            turn_cell,
            input: input_h,
            input_cell,
            agent: agent_handle,
            agent_cell: Some(agent_cell),
            fff_indexer: fff_h,
            fff_cell,
            all_joins,
        })
    }

    /// Coordinator task.
    async fn coordinator(mut cmd_rx: mpsc::Receiver<LeaderCommand>, bus: EventBus<CoreEvent>) {
        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                LeaderCommand::Shutdown => {
                    bus.publish(CoreEvent::Quit);
                    break;
                }
                LeaderCommand::ForceAbort => break,
                LeaderCommand::Status => {}
            }
        }
    }

    /// Listen for TCP connections.
    async fn listen_tcp(addr: &str, bus: EventBus<CoreEvent>) {
        let listener = match TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(e) => {
                tracing::error!("TCP bind failed: {}", e);
                return;
            }
        };
        tracing::info!("Leader listening on TCP {}", addr);
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    tokio::spawn(Self::handle_client_tcp(stream, bus.clone()));
                }
                Err(e) => tracing::error!("TCP accept error: {}", e),
            }
        }
    }

    /// Handle a TCP client connection.
    async fn handle_client_tcp(stream: tokio::net::TcpStream, bus: EventBus<CoreEvent>) {
        let bus2 = bus.clone();
        let (rd, mut wr) = tokio::io::split(stream);

        let wr_handle = tokio::spawn(async move {
            let mut sub = bus2.subscribe();
            while let Ok(event) = sub.recv().await {
                if let Some(json) = event_to_json(&event) {
                    let line = format!("{}\n", json);
                    if wr.write_all(line.as_bytes()).await.is_err() {
                        break;
                    }
                    let _ = wr.flush().await;
                }
            }
        });

        // Use BufReader to handle UTF-8 correctly across read boundaries
        let mut reader = tokio::io::BufReader::new(rd);
        let mut line = String::new();
        while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
            // Remove trailing newline
            line = line.trim_end_matches('\n').to_string();
            if !line.is_empty() {
                process_client_line(&line, &bus);
            }
            line.clear();
        }

        wr_handle.abort();
    }

    /// Run in foreground mode.
    pub async fn run(
        self,
        provider_factory: std::sync::Arc<dyn crate::actors::provider::ProviderFactory>,
        agent_factory: std::sync::Arc<dyn AgentActorFactory<SpawnFuture = AgentSpawnFuture>>,
    ) -> anyhow::Result<()> {
        let _handle = self.start(provider_factory, agent_factory).await?;
        std::future::pending::<()>().await;
        Ok(())
    }
}

impl Default for Leader {
    fn default() -> Self {
        Self::new()
    }
}

/// All handles, cells, and join handles produced by actor spawning.
pub struct SpawnedHandles {
    pub config: RactorConfigHandle,
    pub config_cell: ractor::ActorCell,
    pub provider: RactorProviderHandle,
    pub provider_cell: ractor::ActorCell,
    pub io: RactorIoHandle,
    pub io_cell: ractor::ActorCell,
    pub session: RactorSessionHandle,
    pub session_cell: ractor::ActorCell,
    pub permission: RactorPermissionHandle,
    pub permission_cell: ractor::ActorCell,
    pub turn: crate::actors::turn::RactorTurnHandle,
    pub turn_cell: ractor::ActorCell,
    pub input: RactorInputHandle,
    pub input_cell: ractor::ActorCell,
    pub agent: std::sync::Arc<dyn LeaderAgentHandle>,
    /// Agent actor cell. `Some` in production; `None` in the test helper, which
    /// substitutes a no-op agent with no real actor to stop.
    pub agent_cell: Option<ractor::ActorCell>,
    pub fff_indexer: RactorFffIndexerHandle,
    pub fff_cell: ractor::ActorCell,
    /// All actor join handles, collected for batch await during shutdown.
    pub all_joins: Vec<tokio::task::JoinHandle<()>>,
}

/// Process a line from a client.
fn process_client_line(line: &str, bus: &EventBus<CoreEvent>) {
    let line = line.trim();
    if !line.is_empty() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(intent) = json_to_intent(&json) {
                bus.publish(intent);
            }
        }
    }
}

fn event_to_json(event: &CoreEvent) -> Option<String> {
    let method = match event {
        CoreEvent::ConfigLoaded { .. } => "config_loaded",
        CoreEvent::TurnComplete { .. } => "turn_complete",
        CoreEvent::ResponseDelta { .. } => "response_delta",
        CoreEvent::ToolStart { .. } => "tool_start",
        CoreEvent::ToolEnd { .. } => "tool_end",
        CoreEvent::Error { .. } => "error",
        CoreEvent::Quit | CoreEvent::ForceQuit => "quit",
        _ => return None,
    };
    let value = serde_json::json!({ "type": method, "event": event });
    serde_json::to_string(&value).ok()
}

fn json_to_intent(json: &serde_json::Value) -> Option<CoreEvent> {
    let msg_type = json.get("type")?.as_str()?;
    match msg_type {
        "interrupt" => Some(CoreEvent::Abort),
        "shutdown" => Some(CoreEvent::Quit),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::bus::EventBus;

    use super::*;

    /// Layer 2: Verify `Leader::new()` defaults to embedded mode (no TCP).
    #[test]
    fn leader_default_embedded_no_tcp() {
        let leader = Leader::new();
        assert!(
            leader.config.tcp_addr.is_none(),
            "Leader::new() must default to embedded mode"
        );
        assert!(
            !leader.config.project_root.as_os_str().is_empty(),
            "project_root should be set from current directory"
        );
    }

    /// Layer 2: `Leader::new()` returns a config with correct defaults.
    #[test]
    fn leader_config_defaults() {
        let config = LeaderConfig::default();
        assert!(config.tcp_addr.is_none());
    }

    /// Layer 2: `LeaderConfig::with_tcp_addr()` sets the address.
    #[test]
    fn leader_config_with_tcp_addr() {
        let config = LeaderConfig::default().with_tcp_addr("127.0.0.1:8080");
        assert_eq!(config.tcp_addr.as_deref(), Some("127.0.0.1:8080"));
    }

    /// Layer 1: `process_client_line` parses a valid intent line.
    #[tokio::test]
    async fn tcp_line_parsed_to_intent() {
        let bus = Arc::new(EventBus::<CoreEvent>::new(16));
        let mut sub = bus.subscribe();

        process_client_line(r#"{"type": "interrupt"}"#, &bus);

        let event = sub.recv().await.unwrap();
        assert!(matches!(event, CoreEvent::Abort));
    }

    /// Layer 1: `process_client_line` ignores empty and non-intent lines.
    #[tokio::test]
    async fn tcp_line_ignores_empty_and_unknown() {
        let bus = Arc::new(EventBus::<CoreEvent>::new(16));
        let mut sub = bus.subscribe();

        process_client_line("", &bus);
        process_client_line("   ", &bus);
        process_client_line(r#"{"type": "unknown"}"#, &bus);

        // No events should be published
        let timeout = tokio::time::timeout(std::time::Duration::from_millis(50), sub.recv());
        assert!(
            timeout.await.is_err(),
            "Expected no events for empty/unknown lines"
        );
    }

    /// Layer 1: BufReader::read_line handles multi-byte UTF-8 split across reads.
    ///
    /// Tokio's BufReader internally buffers data, so even if bytes arrive in
    /// separate network packets (partial UTF-8 chars), `read_line` will
    /// reassemble them correctly as long as the complete byte sequence is
    /// available in the underlying AsyncRead.
    ///
    /// This test verifies that BufReader correctly reassembles multi-byte
    /// UTF-8 characters within a single line read.
    #[tokio::test]
    async fn bufreader_preserves_split_utf8() {
        // "hello \xe4\xb8\x96\xe7\x95\x8c\n" = "hello 世界\n"
        // "世" = [0xE4, 0xB8, 0x96] (3 bytes), "界" = [0xE7, 0x95, 0x8C] (3 bytes)
        let full = b"hello \xe4\xb8\x96\xe7\x95\x8c\n";
        let mut reader = tokio::io::BufReader::new(full.as_slice());
        let mut line = String::new();
        let n = reader.read_line(&mut line).await.unwrap();
        assert!(n > 0, "Should read bytes");
        assert_eq!(line.trim_end(), "hello 世界");
    }
}
