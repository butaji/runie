//! Ractor adapter types: handle wrappers, RPC reply channels, and spawn helpers.
//!
//! All actors use `ractor` as the runtime. This module provides:
//! - `RactorHandle<Msg>` — cloneable handle to a ractor actor
//! - `RpcReply<T>` / `rpc_channel()` — oneshot-based RPC reply channels
//! - `spawn_ractor()` — ergonomic actor spawn helper

use ractor::concurrency::JoinHandle;
use ractor::SpawnErr as RactorSpawnErr;
use ractor::{Actor, ActorRef};
use tokio::sync::oneshot;

#[cfg(test)]
use crate::bus::EventBus;

// Re-export ractor types for convenience
pub use ractor::{async_trait, ActorName};

/// Handle to a ractor-based actor.
#[derive(Clone)]
pub struct RactorHandle<Msg: ractor::Message> {
    actor_ref: ActorRef<Msg>,
}

impl<Msg: ractor::Message> RactorHandle<Msg> {
    /// Send a message to the actor (fire-and-forget).
    pub async fn send(&self, msg: Msg) {
        let _ = self.actor_ref.send_message(msg);
    }

    /// Try to send a message (non-blocking).
    /// Returns Ok(()) if the message was sent, Err(MessagingErr) if the channel is closed.
    pub fn try_send(&self, msg: Msg) -> Result<(), ractor::MessagingErr<Msg>> {
        self.actor_ref.send_message(msg)
    }

    /// Get the actor cell for supervision.
    pub fn cell(&self) -> ractor::ActorCell {
        self.actor_ref.get_cell()
    }

    /// Access the underlying actor ref.
    pub fn actor_ref(&self) -> &ActorRef<Msg> {
        &self.actor_ref
    }

    /// Convert to an owned `ActorRef`.
    pub fn into_actor_ref(self) -> ActorRef<Msg> {
        self.actor_ref
    }

}

/// Construct a `RactorHandle` from an owned `ActorRef`.
impl<Msg: ractor::Message> From<ActorRef<Msg>> for RactorHandle<Msg> {
    fn from(actor_ref: ActorRef<Msg>) -> Self {
        Self { actor_ref }
    }
}

impl<Msg: ractor::Message> std::fmt::Debug for RactorHandle<Msg> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RactorHandle")
            .field("actor_id", &self.actor_ref.get_id())
            .finish()
    }
}

/// Spawn a ractor actor and return a handle.
pub async fn spawn_ractor<A>(
    name: Option<ActorName>,
    actor: A,
    args: A::Arguments,
) -> Result<(RactorHandle<A::Msg>, JoinHandle<()>, ractor::ActorCell), RactorSpawnErr>
where
    A: Actor,
{
    let (actor_ref, handle) = Actor::spawn(name, actor, args).await?;
    Ok((
        RactorHandle {
            actor_ref: actor_ref.clone(),
        },
        handle,
        actor_ref.get_cell(),
    ))
}

// ── Event bus integration ──────────────────────────────────────────────────────

// Note: EventBus<E> is already Clone, so actors can hold it directly.
// No wrapper is needed.

// ── Reply wrapper (for RPC) ───────────────────────────────────────────────────

/// Alias for `RpcReply` for backward compatibility.
pub type Reply<T> = RpcReply<T>;

/// Wrapper for RPC reply channels, compatible with both runtimes.
/// Uses Arc<Mutex<Option<oneshot::Sender<T>>>> to allow cloning.
#[derive(Debug)]
pub struct RpcReply<T>(std::sync::Arc<parking_lot::Mutex<Option<oneshot::Sender<T>>>>);

impl<T> RpcReply<T> {
    /// Create a new reply handle from a oneshot sender.
    pub fn new(sender: oneshot::Sender<T>) -> Self {
        Self(std::sync::Arc::new(parking_lot::Mutex::new(Some(sender))))
    }

    /// Send the reply value.
    pub fn send(self, value: T) {
        if let Some(sender) = self.0.lock().take() {
            let _ = sender.send(value);
        }
    }
}

impl<T> Clone for RpcReply<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

/// Create a new RPC channel.
pub fn rpc_channel<T>() -> (RpcReply<T>, oneshot::Receiver<T>) {
    let (tx, rx) = oneshot::channel();
    (RpcReply::new(tx), rx)
}

// ── Re-exports ─────────────────────────────────────────────────────────────────

pub use ractor::{ActorErr, ActorProcessingErr, MessagingErr, RactorErr};

#[cfg(test)]
mod tests {
    use super::*;

/// Test that actors can publish events directly via EventBus.
    #[tokio::test]
    async fn event_bus_actors_publish_directly() {
        let bus = EventBus::<String>::new(16);

        let mut sub = bus.subscribe();
        bus.publish("test".to_string());

        let result = sub.recv().await.unwrap();
        assert_eq!(result, "test");
    }

    /// Test RPC channel creation.
    #[test]
    fn rpc_channel_works() {
        let (reply, rx) = rpc_channel::<String>();
        reply.send("response".to_string());
        assert_eq!(rx.blocking_recv().unwrap(), "response");
    }
}
