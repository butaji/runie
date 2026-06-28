//! Leader implementation.
//!
//! Coordinates all actors in the Runie runtime and optionally listens
//! on a local socket for client connections.

use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc};

use crate::bus::EventBus;
use crate::event::Event;
use crate::actors::{
    ConfigActor, IoActor, SessionActor,
    ConfigActorHandle, IoActorHandle, SessionActorHandle,
    ProviderActor, ProviderActorHandle,
};
use crate::actors::permission::RactorPermissionActor;
use crate::actors::turn::RactorTurnActor;
use crate::actors::PermissionActorHandle;

use super::messages::{LeaderCommand, LeaderStatus};

/// Leader coordinates all actors in the Runie runtime.
pub struct Leader {
    /// TCP address to listen on.
    tcp_addr: Option<String>,
}

impl Leader {
    /// Create a new leader with the default TCP address.
    pub fn new() -> Self {
        Self { tcp_addr: Some("127.0.0.1:9000".to_string()) }
    }

    /// Create a leader without socket listening (embedded mode).
    pub fn embedded() -> Self {
        Self { tcp_addr: None }
    }

    /// Use a custom TCP address.
    pub fn with_tcp_addr<A: ToString>(self, addr: A) -> Self {
        Self { tcp_addr: Some(addr.to_string()) }
    }

    /// Start the leader and spawn all actors.
    pub async fn start(
        &self,
        provider_factory: Arc<dyn crate::actors::provider::ProviderFactory>,
    ) -> anyhow::Result<LeaderHandle> {
        let bus = EventBus::<Event>::new(100);
        let (cmd_tx, cmd_rx) = mpsc::channel(32);

        let handles = Self::spawn_actors(&bus, provider_factory).await?;
        tokio::spawn(Self::coordinator(cmd_rx, bus.clone()));

        if let Some(ref addr) = self.tcp_addr {
            let bus_clone = bus.clone();
            let addr = addr.clone();
            tokio::spawn(async move {
                Self::listen_tcp(&addr, bus_clone).await;
            });
        }

        Ok(LeaderHandle {
            cmd_tx,
            event_bus: bus.clone(),
            tcp_addr: self.tcp_addr.clone(),
            config: handles.config,
            provider: handles.provider,
            io: handles.io,
            session: handles.session,
            permission: handles.permission,
            turn: handles.turn,
        })
    }

    /// Spawn all child actors.
    async fn spawn_actors(
        bus: &EventBus<Event>,
        factory: Arc<dyn crate::actors::provider::ProviderFactory>,
    ) -> anyhow::Result<SpawnedHandles> {
        let (config, _) = ConfigActor::spawn(bus.clone(), None);
        let (provider, _) = ProviderActor::spawn(bus.clone(), config.clone(), factory);
        let (io, _) = IoActor::spawn(bus.clone());
        let (session, _) = SessionActor::spawn(bus.clone());
        let (permission, _) = RactorPermissionActor::spawn(bus.clone()).await;
        let (turn, _, _) = RactorTurnActor::spawn(bus.clone()).await;

        Ok(SpawnedHandles { config, provider, io, session, permission, turn })
    }

    /// Coordinator task.
    async fn coordinator(mut cmd_rx: mpsc::Receiver<LeaderCommand>, bus: EventBus<Event>) {
        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                LeaderCommand::Shutdown => { bus.publish(Event::Quit); break; }
                LeaderCommand::ForceAbort => break,
                LeaderCommand::Status => {}
            }
        }
    }

    /// Listen for TCP connections.
    async fn listen_tcp(addr: &str, bus: EventBus<Event>) {
        let listener = match TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(e) => { tracing::error!("TCP bind failed: {}", e); return; }
        };
        tracing::info!("Leader listening on TCP {}", addr);
        loop {
            match listener.accept().await {
                Ok((stream, _)) => { tokio::spawn(Self::handle_client_tcp(stream, bus.clone())); }
                Err(e) => tracing::error!("TCP accept error: {}", e),
            }
        }
    }

    /// Handle a TCP client connection.
    async fn handle_client_tcp(stream: tokio::net::TcpStream, bus: EventBus<Event>) {
        let bus2 = bus.clone();
        let (mut rd, mut wr) = tokio::io::split(stream);
        let mut buf = vec![0u8; 1024].into_boxed_slice();

        let wr_handle = tokio::spawn(async move {
            let mut sub = bus2.subscribe();
            while let Ok(event) = sub.recv().await {
                if let Some(json) = event_to_json(&event) {
                    let line = format!("{}\n", json);
                    if wr.write_all(line.as_bytes()).await.is_err() { break; }
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
    pub async fn run(self, factory: Arc<dyn crate::actors::provider::ProviderFactory>) -> anyhow::Result<()> {
        let _ = self.start(factory).await?;
        // Wait forever - the leader runs until killed
        std::future::pending::<()>().await;
        Ok(())
    }
}

impl Default for Leader {
    fn default() -> Self { Self::new() }
}

struct SpawnedHandles {
    config: ConfigActorHandle,
    provider: ProviderActorHandle,
    io: IoActorHandle,
    session: SessionActorHandle,
    permission: PermissionActorHandle,
    turn: crate::actors::turn::RactorTurnHandle,
}

/// Process a line from a client.
fn process_client_line(line: &str, bus: &EventBus<Event>) {
    let line = line.trim();
    if !line.is_empty() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(intent) = json_to_intent(&json) {
                bus.publish(intent);
            }
        }
    }
}

fn event_to_json(event: &Event) -> Option<String> {
    let method = match event {
        Event::ConfigLoaded { .. } => "config_loaded",
        Event::TurnComplete { .. } => "turn_complete",
        Event::ResponseDelta { .. } => "response_delta",
        Event::ToolStart { .. } => "tool_start",
        Event::ToolEnd { .. } => "tool_end",
        Event::Error { .. } => "error",
        Event::Quit | Event::ForceQuit => "quit",
        _ => return None,
    };
    let value = serde_json::json!({ "type": method, "event": event });
    serde_json::to_string(&value).ok()
}

fn json_to_intent(json: &serde_json::Value) -> Option<Event> {
    let msg_type = json.get("type")?.as_str()?;
    match msg_type {
        "interrupt" => Some(Event::Abort),
        "shutdown" => Some(Event::Quit),
        _ => None,
    }
}

/// Handle to the running leader.
#[derive(Clone)]
pub struct LeaderHandle {
    cmd_tx: mpsc::Sender<LeaderCommand>,
    event_bus: EventBus<Event>,
    tcp_addr: Option<String>,
    pub config: ConfigActorHandle,
    pub provider: ProviderActorHandle,
    pub io: IoActorHandle,
    pub session: SessionActorHandle,
    pub permission: PermissionActorHandle,
    pub turn: crate::actors::turn::RactorTurnHandle,
}

impl LeaderHandle {
    /// Get the event bus.
    pub fn event_bus(&self) -> &EventBus<Event> { &self.event_bus }
    /// Subscribe to facts.
    pub fn subscribe(&self) -> broadcast::Receiver<Event> { self.event_bus.subscribe() }
    /// Get TCP address.
    pub fn tcp_addr(&self) -> Option<&str> { self.tcp_addr.as_deref() }
    /// Send shutdown command.
    pub async fn shutdown(&self) { let _ = self.cmd_tx.send(LeaderCommand::Shutdown).await; }
    /// Get status.
    pub fn status(&self) -> LeaderStatus {
        LeaderStatus { running: true, actor_count: 6, bus_subscribers: self.event_bus.subscriber_count() }
    }
}

impl AsRef<EventBus<Event>> for LeaderHandle {
    fn as_ref(&self) -> &EventBus<Event> { &self.event_bus }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leader_status_default() {
        let status = LeaderStatus::default();
        assert!(!status.running);
        assert_eq!(status.actor_count, 0);
    }

    #[test]
    fn leader_command_debug() {
        format!("{:?}", LeaderCommand::Status);
        format!("{:?}", LeaderCommand::Shutdown);
        format!("{:?}", LeaderCommand::ForceAbort);
    }

    #[test]
    fn leader_new() {
        let leader = Leader::new();
        assert!(leader.tcp_addr.is_some());
    }

    #[test]
    fn leader_embedded() {
        let leader = Leader::embedded();
        assert!(leader.tcp_addr.is_none());
    }

    #[test]
    fn leader_with_tcp_addr() {
        let leader = Leader::new().with_tcp_addr("0.0.0.0:9001");
        assert_eq!(leader.tcp_addr.as_deref(), Some("0.0.0.0:9001"));
    }
}
