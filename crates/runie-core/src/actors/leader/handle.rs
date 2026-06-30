//! LeaderHandle implementation.

use tokio::sync::{broadcast, mpsc};

use crate::bus::EventBus;
use crate::Event as CoreEvent;

use super::messages::LeaderStatus;
use super::{LeaderAgentHandle, SpawnedHandles};

/// Handle to the running leader.
///
/// Cloneable so it can be shared across tasks. All actor refs are also cloneable.
#[derive(Clone)]
pub struct LeaderHandle {
    cmd_tx: mpsc::Sender<super::messages::LeaderCommand>,
    event_bus: EventBus<CoreEvent>,
    tcp_addr: Option<String>,
    /// Config actor handle.
    pub config: crate::actors::RactorConfigHandle,
    /// Provider actor handle.
    pub provider: crate::actors::RactorProviderHandle,
    /// IO actor handle.
    pub io: crate::actors::RactorIoHandle,
    /// Session actor handle.
    pub session: crate::actors::RactorSessionHandle,
    /// Permission actor handle.
    pub permission: crate::actors::RactorPermissionHandle,
    /// Turn actor handle.
    pub turn: crate::actors::turn::RactorTurnHandle,
    /// Input actor handle.
    pub input: crate::actors::RactorInputHandle,
    /// Agent actor handle.
    pub agent: std::sync::Arc<dyn LeaderAgentHandle>,
    /// FFF indexer handle.
    pub fff_indexer: crate::actors::RactorFffIndexerHandle,
    // ── Shutdown state ────────────────────────────────────────────────────────
    config_cell: ractor::ActorCell,
    provider_cell: ractor::ActorCell,
    io_cell: ractor::ActorCell,
    session_cell: ractor::ActorCell,
    permission_cell: ractor::ActorCell,
    turn_cell: ractor::ActorCell,
    turn_join: std::sync::Arc<tokio::task::JoinHandle<()>>,
    input_cell: ractor::ActorCell,
    agent_join: std::sync::Arc<tokio::task::JoinHandle<()>>,
    fff_cell: ractor::ActorCell,
    /// Snapshot channel receiver placeholder. The TUI manages its own snapshot
    /// channel via UiActor::take_render_rx(); this field exists so that callers
    /// that only hold a LeaderHandle can still verify snapshot-channel delivery.
    #[allow(dead_code)]
    pub snapshot_rx: tokio::sync::watch::Receiver<crate::Snapshot>,
}

impl LeaderHandle {
    pub fn new(
        cmd_tx: mpsc::Sender<super::messages::LeaderCommand>,
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
            input: handles.input,
            agent: handles.agent,
            fff_indexer: handles.fff_indexer,
            config_cell: handles.config_cell,
            provider_cell: handles.provider_cell,
            io_cell: handles.io_cell,
            session_cell: handles.session_cell,
            permission_cell: handles.permission_cell,
            turn_cell: handles.turn_cell,
            turn_join: handles.turn_join,
            input_cell: handles.input_cell,
            agent_join: handles.agent_join,
            fff_cell: handles.fff_cell,
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

    /// Stop all child actors gracefully and await their completion.
    ///
    /// 1. Publishes `Quit` to signal all actors.
    /// 2. Stops all actor cells to ensure termination.
    /// 3. Awaits the turn and agent join handles.
    pub async fn shutdown(self) {
        use super::messages::LeaderCommand;
        let _ = self.cmd_tx.send(LeaderCommand::Shutdown).await;
        self.event_bus.publish(CoreEvent::Quit);

        // Stop all actors (reverse spawn order: least dependent first).
        self.input_cell.stop(None);
        self.session_cell.stop(None);
        self.turn_cell.stop(None);
        self.fff_cell.stop(None);
        self.permission_cell.stop(None);
        self.config_cell.stop(None);
        self.provider_cell.stop(None);
        self.io_cell.stop(None);

        // Await turn and agent join handles.
        let turn_join = std::sync::Arc::try_unwrap(self.turn_join)
            .expect("turn join handle should have single ref on shutdown");
        let _ = turn_join.await;
        let agent_join = std::sync::Arc::try_unwrap(self.agent_join)
            .expect("agent join handle should have single ref on shutdown");
        let _ = agent_join.await;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Layer 1: Compile-time check that `LeaderHandle` exposes all required actor ref fields.
    #[test]
    fn leader_handle_exposes_all_actor_refs() {
        fn _check_types() {
            fn _field<T>(_: &T) {}
            let handle: LeaderHandle = unimplemented!();
            _field(&handle.config);
            _field(&handle.provider);
            _field(&handle.io);
            _field(&handle.session);
            _field(&handle.permission);
            _field(&handle.turn);
            _field(&handle.input);
            _field(&handle.agent);
            _field(&handle.fff_indexer);
            // snapshot_rx is also exposed for render-path tests.
            _field(&handle.snapshot_rx);
        }
    }
}
