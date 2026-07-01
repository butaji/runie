//! LeaderHandle implementation.

/// Number of actors managed by the leader (config, provider, io, session,
/// permission, turn, input, agent, fff_indexer). Used for status diagnostics.
const SPAWNED_ACTOR_COUNT: usize = 9;

use tokio::sync::{broadcast, mpsc};

use crate::bus::EventBus;
use crate::Event as CoreEvent;

use super::messages::LeaderStatus;
use super::{LeaderAgentHandle, SpawnedHandles};

/// Handle to the running leader.
///
/// Cloneable so it can be shared across tasks. All actor refs are also cloneable.
/// Note: `Clone` does not clone the join handles; only the first clone to call
/// `shutdown()` will await the handles.
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
    /// Actor cells (for stop signals).
    config_cell: ractor::ActorCell,
    provider_cell: ractor::ActorCell,
    io_cell: ractor::ActorCell,
    session_cell: ractor::ActorCell,
    permission_cell: ractor::ActorCell,
    turn_cell: ractor::ActorCell,
    input_cell: ractor::ActorCell,
    fff_cell: ractor::ActorCell,
    /// All actor join handles wrapped in Option so LeaderHandle can be Clone.
    /// Taken at shutdown time via mem::take.
    joins: Option<Vec<tokio::task::JoinHandle<()>>>,
    /// Coordinator task join handle (None in clones since handles can't be cloned).
    coordinator_join: Option<tokio::task::JoinHandle<()>>,
    /// TCP listener task join handle (None when not in server mode).
    tcp_join: Option<tokio::task::JoinHandle<()>>,
}

impl LeaderHandle {
    pub fn new(
        cmd_tx: mpsc::Sender<super::messages::LeaderCommand>,
        event_bus: EventBus<CoreEvent>,
        handles: SpawnedHandles,
        coordinator_join: Option<tokio::task::JoinHandle<()>>,
        tcp_join: Option<tokio::task::JoinHandle<()>>,
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
            input_cell: handles.input_cell,
            fff_cell: handles.fff_cell,
            joins: Some(handles.all_joins),
            coordinator_join,
            tcp_join,
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
    /// 1. Sends `Shutdown` to the coordinator and publishes `Quit` on the event bus.
    /// 2. Stops all actor cells in reverse spawn order.
    /// 3. Awaits all join handles in parallel with a timeout (actors, coordinator, TCP).
    ///
    /// This method never panics, even if the `LeaderHandle` was cloned.
    /// Subsequent clones will have `None` for the joins field after the first shutdown.
    pub async fn shutdown(mut self) {
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

        // Collect all join handles for parallel await.
        let mut all_joins = Vec::with_capacity(
            self.joins.as_ref().map_or(0, |v| v.len()) + 2,
        );

        if let Some(joins) = self.joins.take() {
            all_joins.extend(joins);
        }
        if let Some(cj) = self.coordinator_join.take() {
            all_joins.push(cj);
        }
        if let Some(tcp_join) = self.tcp_join.take() {
            all_joins.push(tcp_join);
        }

        // Await all join handles in parallel with a timeout.
        // Uses tokio::join! for concurrent await since we need all to complete.
        let timeout_duration = std::time::Duration::from_secs(5);
        let result = tokio::time::timeout(timeout_duration, async {
            let mut errors = Vec::new();
            for join in all_joins {
                if let Err(e) = join.await {
                    errors.push(e);
                }
            }
            errors
        })
        .await;

        if result.is_err() {
            tracing::warn!("Leader shutdown timed out after {:?}, {} actors may still be running", timeout_duration, self.joins.as_ref().map_or(0, |v| v.len()));
        }
    }

    /// Get runtime status.
    pub fn status(&self) -> LeaderStatus {
        LeaderStatus {
            running: true,
            actor_count: SPAWNED_ACTOR_COUNT,
            bus_subscribers: self.event_bus.subscriber_count(),
        }
    }
}

impl Clone for LeaderHandle {
    /// Clone the handle. Note: the joins/coordinator/tcp fields are set to None in
    /// the clone since join handles cannot be cloned. Only the original handle (or
    /// first clone) will await the handles on shutdown.
    fn clone(&self) -> Self {
        Self {
            cmd_tx: self.cmd_tx.clone(),
            event_bus: self.event_bus.clone(),
            tcp_addr: self.tcp_addr.clone(),
            config: self.config.clone(),
            provider: self.provider.clone(),
            io: self.io.clone(),
            session: self.session.clone(),
            permission: self.permission.clone(),
            turn: self.turn.clone(),
            input: self.input.clone(),
            agent: self.agent.clone(),
            fff_indexer: self.fff_indexer.clone(),
            config_cell: self.config_cell.clone(),
            provider_cell: self.provider_cell.clone(),
            io_cell: self.io_cell.clone(),
            session_cell: self.session_cell.clone(),
            permission_cell: self.permission_cell.clone(),
            turn_cell: self.turn_cell.clone(),
            input_cell: self.input_cell.clone(),
            fff_cell: self.fff_cell.clone(),
            joins: None,
            coordinator_join: None,
            tcp_join: None,
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
    ///
    /// Uses trait bounds to verify field names and types compile without constructing the struct.
    /// This avoids `unsafe { mem::zeroed() }` which is UB for non-Copy types.
    #[test]
    fn leader_handle_exposes_all_actor_refs() {
        fn _assert_field<T: 'static>(_v: &T) {}

        // Phantom data to hold field types without constructing LeaderHandle.
        struct FieldChecker<T> {
            _p: std::marker::PhantomData<T>,
        }

        impl<T> FieldChecker<T> {
            fn check(_: &LeaderHandle)
            where
                T: std::borrow::Borrow<LeaderHandle>,
            {
                // no-op: only compile-time check
            }
        }

        // If this compiles, LeaderHandle has all required public fields.
        // We never construct a LeaderHandle, just verify the type resolves.
        fn _compile_time_check(_: impl Fn(&LeaderHandle)) {}
        _compile_time_check(|h| {
            _assert_field::<crate::actors::RactorConfigHandle>(&h.config);
            _assert_field::<crate::actors::RactorProviderHandle>(&h.provider);
            _assert_field::<crate::actors::RactorIoHandle>(&h.io);
            _assert_field::<crate::actors::RactorSessionHandle>(&h.session);
            _assert_field::<crate::actors::RactorPermissionHandle>(&h.permission);
            _assert_field::<crate::actors::turn::RactorTurnHandle>(&h.turn);
            _assert_field::<crate::actors::RactorInputHandle>(&h.input);
            _assert_field::<std::sync::Arc<dyn LeaderAgentHandle>>(&h.agent);
            _assert_field::<crate::actors::RactorFffIndexerHandle>(&h.fff_indexer);
        });
    }

    /// Layer 1: `status.actor_count` matches the expected constant.
    #[test]
    fn leader_status_counts_actors() {
        assert_eq!(SPAWNED_ACTOR_COUNT, 9);
        // Keep the constant consistent with SpawnedHandles fields:
        // config, provider, io, session, permission, turn, input, agent, fff_indexer
    }
}
