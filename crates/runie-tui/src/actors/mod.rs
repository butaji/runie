//! Lightweight actor system for runie-tui.
//!
//! Phase 4: Replace dead actor framework with lightweight, concurrent I/O actors.
//!
//! Design principles:
//! - No JSON serialization, no type erasure
//! - Actors own I/O scope, not UI state
//! - Actors emit messages TO the StatePipe
//! - Scope encapsulation — each actor is self-contained

use std::future::Future;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// Lightweight actor trait — actors run concurrent I/O operations
/// and emit messages to a channel.
pub trait Actor: Send + 'static {
    /// Message type this actor emits
    type Msg: Send + Clone;

    /// Actor name for logging
    fn name(&self) -> &'static str;

    /// Run the actor. Consumes self.
    /// Should loop until cancelled, emitting messages via channel.
    fn run(self, msg_tx: mpsc::Sender<Self::Msg>, cancel: CancellationToken) -> impl Future<Output = ()> + Send;
}

/// ActorHandle — Spawn an actor and get a handle to control it
pub struct ActorHandle {
    cancel: CancellationToken,
}

impl ActorHandle {
    /// Create a new ActorHandle and associated CancellationToken
    pub fn new() -> (Self, CancellationToken) {
        let cancel = CancellationToken::new();
        (Self { cancel: cancel.clone() }, cancel)
    }

    /// Signal the actor to shutdown
    pub fn shutdown(&self) {
        self.cancel.cancel();
    }
}

impl Default for ActorHandle {
    fn default() -> Self {
        let (_, cancel) = Self::new();
        Self { cancel }
    }
}

/// Spawn an actor and return a handle to control it
pub fn spawn_actor<A>(actor: A) -> (ActorHandle, mpsc::Receiver<A::Msg>)
where
    A: Actor,
{
    let (handle, cancel) = ActorHandle::new();
    let (msg_tx, msg_rx) = mpsc::channel(32);
    let name = actor.name();

    tokio::spawn(async move {
        tracing::info!(target: "runie", "[ACTOR:{}] Starting actor", name);
        actor.run(msg_tx, cancel).await;
        tracing::info!(target: "runie", "[ACTOR:{}] Actor stopped", name);
    });

    (handle, msg_rx)
}

// ─── Actor implementations ─────────────────────────────────────────────────────

pub mod input;
pub mod timer;
pub mod agent;

#[cfg(test)]
mod tests;