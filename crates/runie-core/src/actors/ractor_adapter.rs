//! Ractor adapter types: RPC reply channels and spawn helpers.
//!
//! All actors use `ractor` as the runtime. This module provides:
//! - `RpcReply<T>` / `rpc_channel()` — oneshot-based RPC reply channels
//! - `spawn_ractor()` — ergonomic actor spawn helper that returns `ActorRef` directly

use ractor::concurrency::JoinHandle;
use ractor::SpawnErr as RactorSpawnErr;
use ractor::{Actor, ActorRef};
use tokio::sync::oneshot;

#[cfg(test)]
use crate::bus::EventBus;

// Re-export ractor types for convenience
pub use ractor::{async_trait, ActorName};

/// Spawn a ractor actor and return an `ActorRef`.
pub async fn spawn_ractor<A>(
    name: Option<ActorName>,
    actor: A,
    args: A::Arguments,
) -> Result<(ActorRef<A::Msg>, JoinHandle<()>, ractor::ActorCell), RactorSpawnErr>
where
    A: Actor,
{
    let (actor_ref, handle) = Actor::spawn(name, actor, args).await?;
    Ok((actor_ref.clone(), handle, actor_ref.get_cell()))
}



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
