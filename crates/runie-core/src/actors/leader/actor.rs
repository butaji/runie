//! Leader implementation.
//!
//! Coordinates all actors in the Runie runtime and optionally listens
//! on a local socket for client connections.

use std::path::PathBuf;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc};

use crate::actors::leader::AgentSpawnFuture;
use crate::actors::turn::RactorTurnActor;
use crate::actors::{
    InputActor, RactorConfigActor, RactorConfigHandle, RactorFffIndexerActor,
    RactorFffIndexerHandle, RactorInputHandle, RactorIoActor, RactorIoHandle,
    RactorPermissionActor, RactorPermissionHandle, RactorProviderActor, RactorProviderHandle,
    RactorSessionActor, RactorSessionHandle,
};
use crate::bus::EventBus;
use crate::Event as CoreEvent;

use super::messages::{LeaderCommand, LeaderStatus};
use super::{AgentActorFactory, LeaderAgentHandle};

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
        let bus = EventBus::<CoreEvent>::new(100);
        let (cmd_tx, cmd_rx) = mpsc::channel(32);

        let handles =
            Self::spawn_actors(&bus, &self.config, provider_factory, agent_factory).await?;
        tokio::spawn(Self::coordinator(cmd_rx, bus.clone()));

        if let Some(ref addr) = self.config.tcp_addr {
            let bus_clone = bus.clone();
            let addr = addr.clone();
            tokio::spawn(async move { Self::listen_tcp(&addr, bus_clone).await });
        }

        Ok(LeaderHandle::new(cmd_tx, bus, handles))
    }

    /// Spawn all child actors.
    async fn spawn_actors(
        bus: &EventBus<CoreEvent>,
        config: &LeaderConfig,
        provider_factory: std::sync::Arc<dyn crate::actors::provider::ProviderFactory>,
        agent_factory: std::sync::Arc<dyn AgentActorFactory<SpawnFuture = AgentSpawnFuture>>,
    ) -> anyhow::Result<SpawnedHandles> {
        let (config_h, _) = RactorConfigActor::spawn_default(bus.clone()).await;
        let (provider_h, _) =
            RactorProviderActor::spawn(bus.clone(), config_h.clone(), provider_factory).await?;
        let (io_h, _) = RactorIoActor::spawn(bus.clone()).await?;
        let (session_h, _) = RactorSessionActor::spawn(bus.clone()).await?;
        let (permission_h, _) = RactorPermissionActor::spawn(bus.clone()).await;
        let (turn_h, _, turn_join) = RactorTurnActor::spawn(bus.clone()).await;
        let (input_h, _) = InputActor::spawn(bus.clone()).await;
        let (fff_h, _) = RactorFffIndexerActor::spawn(
            config.project_root.clone(),
            config.data_dir.clone(),
            bus.clone(),
        )
        .await
        .map_err(|e| anyhow::anyhow!("FffIndexerActor spawn failed: {}", e))?;
        let agent_handle = agent_factory
            .spawn(bus.clone(), provider_h.clone(), permission_h.clone())
            .await?;

        Ok(SpawnedHandles {
            config: config_h,
            provider: provider_h,
            io: io_h,
            session: session_h,
            permission: permission_h,
            turn: turn_h,
            turn_join: Some(std::sync::Arc::new(turn_join)),
            input: input_h,
            agent: agent_handle,
            fff_indexer: fff_h,
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
        let (mut rd, mut wr) = tokio::io::split(stream);
        let mut buf = vec![0u8; 1024].into_boxed_slice();

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

        loop {
            match rd.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    if let Ok(line) = std::str::from_utf8(&buf[..n]) {
                        process_client_line(line, &bus);
                    }
                }
                Err(_) => break,
            }
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

struct SpawnedHandles {
    config: RactorConfigHandle,
    provider: RactorProviderHandle,
    io: RactorIoHandle,
    session: RactorSessionHandle,
    permission: RactorPermissionHandle,
    turn: crate::actors::turn::RactorTurnHandle,
    turn_join: Option<std::sync::Arc<tokio::task::JoinHandle<()>>>,
    input: RactorInputHandle,
    agent: std::sync::Arc<dyn LeaderAgentHandle>,
    fff_indexer: RactorFffIndexerHandle,
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



/// Handle to the running leader.
///
/// Cloneable so it can be shared across tasks. All actor refs are also cloneable.
#[derive(Clone)]
pub struct LeaderHandle {
    cmd_tx: mpsc::Sender<LeaderCommand>,
    event_bus: EventBus<CoreEvent>,
    tcp_addr: Option<String>,
    /// Config actor handle.
    pub config: RactorConfigHandle,
    /// Provider actor handle.
    pub provider: RactorProviderHandle,
    /// IO actor handle.
    pub io: RactorIoHandle,
    /// Session actor handle.
    pub session: RactorSessionHandle,
    /// Permission actor handle.
    pub permission: RactorPermissionHandle,
    /// Turn actor handle.
    pub turn: crate::actors::turn::RactorTurnHandle,
    /// Input actor handle.
    pub input: RactorInputHandle,
    /// Agent actor handle.
    pub agent: std::sync::Arc<dyn LeaderAgentHandle>,
    /// FFF indexer handle.
    pub fff_indexer: RactorFffIndexerHandle,
    /// Turn actor join handle (for graceful shutdown).
    #[allow(dead_code)]
    turn_join: Option<std::sync::Arc<tokio::task::JoinHandle<()>>>,
    /// Snapshot channel receiver placeholder. The TUI manages its own snapshot
    /// channel via UiActor::take_render_rx(); this field exists so that callers
    /// that only hold a LeaderHandle can still verify snapshot-channel delivery.
    #[allow(dead_code)]
    pub snapshot_rx: tokio::sync::watch::Receiver<crate::Snapshot>,
}

impl LeaderHandle {
    fn new(
        cmd_tx: mpsc::Sender<LeaderCommand>,
        event_bus: EventBus<CoreEvent>,
        handles: SpawnedHandles,
    ) -> Self {
        Self {
            cmd_tx,
            event_bus,
            tcp_addr: None,
            config: handles.config,
            provider: handles.provider,
            io: handles.io,
            session: handles.session,
            permission: handles.permission,
            turn: handles.turn,
            turn_join: handles.turn_join,
            input: handles.input,
            agent: handles.agent,
            fff_indexer: handles.fff_indexer,
            snapshot_rx: tokio::sync::watch::channel(crate::Snapshot::default()).1,
        }
    }

    /// Subscribe to facts published on the event bus.
    pub fn subscribe(&self) -> broadcast::Receiver<CoreEvent> {
        self.event_bus.subscribe()
    }

    /// Get the underlying event bus.
    pub fn event_bus(&self) -> &EventBus<CoreEvent> {
        &self.event_bus
    }

    /// Get TCP address (if server mode).
    pub fn tcp_addr(&self) -> Option<&str> {
        self.tcp_addr.as_deref()
    }

    /// Send shutdown command to stop all actors gracefully.
    pub async fn shutdown(&self) {
        let _ = self.cmd_tx.send(LeaderCommand::Shutdown).await;
    }

    /// Get runtime status.
    pub fn status(&self) -> LeaderStatus {
        LeaderStatus {
            running: true,
            actor_count: 9,
            bus_subscribers: self.event_bus.subscriber_count(),
        }
    }
}

impl AsRef<EventBus<CoreEvent>> for LeaderHandle {
    fn as_ref(&self) -> &EventBus<CoreEvent> {
        &self.event_bus
    }
}

/// Test helpers for constructing a `LeaderHandle` with all actors spawned.
pub mod test_helpers {
    use super::*;
    use crate::actors::leader::LeaderAgentCmd;

    /// Construct a minimal `LeaderHandle` for unit tests.
    ///
    /// Spawns all production actors with a shared event bus and returns
    /// a `LeaderHandle` with default bus/command channels.
    /// The caller takes ownership and must eventually call `shutdown()`.
    pub async fn test_leader_handle() -> LeaderHandle {
        use crate::actors::provider::{BuiltProvider, ProviderFactory};
        use crate::provider::{Provider, ProviderError};
        use crate::provider_event::ProviderEvent;
        use std::future::Future;
        use std::pin::Pin;
        use std::sync::Arc;

        struct NoOpAgentHandle;
        impl LeaderAgentHandle for NoOpAgentHandle {
            fn run(&self, _cmd: LeaderAgentCmd) -> Pin<Box<dyn Future<Output = ()> + Send>> {
                Box::pin(std::future::pending())
            }
        }

        struct NoOpProvider;
        impl Provider for NoOpProvider {
            fn generate(
                &self,
                _: Vec<crate::message::ChatMessage>,
            ) -> Pin<Box<dyn futures::Stream<Item = anyhow::Result<ProviderEvent>> + Send + '_>>
            {
                Box::pin(futures::stream::empty())
            }
        }

        struct TestProviderFactory;
        impl ProviderFactory for TestProviderFactory {
            fn build(
                &self,
                provider: &str,
                model: &str,
                _config: &crate::Config,
            ) -> Result<BuiltProvider, ProviderError> {
                Ok(BuiltProvider::new(
                    Box::new(NoOpProvider),
                    provider.into(),
                    model.into(),
                ))
            }
            fn validate_key(
                &self,
                _: &str,
                _: &str,
            ) -> Pin<Box<dyn Future<Output = anyhow::Result<Vec<String>>> + Send + '_>>
            {
                Box::pin(async { Ok(vec![]) })
            }
            fn resolve_credentials(&self, _: &str, _: &crate::Config) -> (String, String) {
                ("http://localhost".into(), "sk-test".into())
            }
        }

        let bus = EventBus::<CoreEvent>::new(16);
        let (config_h, _) = RactorConfigActor::spawn_default(bus.clone()).await;
        let factory: Arc<dyn ProviderFactory> = Arc::new(TestProviderFactory);
        let (provider_h, _) = RactorProviderActor::spawn(bus.clone(), config_h.clone(), factory)
            .await
            .expect("provider spawn");
        let (io_h, _) = RactorIoActor::spawn(bus.clone()).await.expect("io spawn");
        let (session_h, _) = RactorSessionActor::spawn(bus.clone())
            .await
            .expect("session spawn");
        let (permission_h, _) = RactorPermissionActor::spawn(bus.clone()).await;
        let (turn_h, _, turn_join) = RactorTurnActor::spawn(bus.clone()).await;
        let (input_h, _) = InputActor::spawn(bus.clone()).await;
        let (fff_h, _) = RactorFffIndexerActor::spawn(
            std::env::current_dir().unwrap_or_default(),
            std::env::temp_dir(),
            bus.clone(),
        )
        .await
        .expect("fff indexer spawn");

        let (cmd_tx, _cmd_rx) = mpsc::channel(4);
        let agent: Arc<dyn LeaderAgentHandle> = Arc::new(NoOpAgentHandle);

        LeaderHandle {
            cmd_tx,
            event_bus: bus,
            tcp_addr: None,
            config: config_h,
            provider: provider_h,
            io: io_h,
            session: session_h,
            permission: permission_h,
            turn: turn_h,
            turn_join: Some(std::sync::Arc::new(turn_join)),
            input: input_h,
            agent,
            fff_indexer: fff_h,
            snapshot_rx: tokio::sync::watch::channel(crate::Snapshot::default()).1,
        }
    }
}
