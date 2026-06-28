//! Ractor adapter layer for incremental migration.
//!
//! This module provides a thin wrapper around `ractor` that maintains
//! compatibility with the existing actor interface. It allows gradual
//! migration of actors from the custom runtime to ractor.
//!
//! ## Architecture
//!
//! ```text
//! Current:                      With adapter:
//! ┌──────────────────┐          ┌──────────────────┐
//! │  Custom Actor    │          │  Custom Actor     │
//! │  trait          │          │  trait            │
//! └────────┬─────────┘          └────────┬─────────┘
//!          │                             │
//! ┌────────▼─────────┐          ┌────────▼─────────┐
//! │  spawn_actor()   │          │  RactorAdapter   │
//! └──────────────────┘          └────────┬─────────┘
//!                                         │
//!                                 ┌────────▼─────────┐
//!                                 │  ractor::Actor  │
//!                                 └──────────────────┘
//! ```
//!
//! ## Usage
//!
//! For new actors, use `RactorActor` which wraps ractor:
//!
//! ```ignore
//! use crate::actors::ractor_adapter::{RactorActor, RactorHandle};
//! use ractor::{Actor, ActorRef};
//!
//! pub struct MyActor {
//!     state: usize,
//! }
//!
//! #[ractor::async_trait]
//! impl Actor for MyActor {
//!     type Msg = MyMsg;
//!     type State = ();
//!     type Arguments = ();
//!
//!     async fn pre_start(
//!         &self,
//!         myself: ActorRef<Self::Msg>,
//!         _: (),
//!     ) -> Result<(), ractor::ActorProcessingErr> {
//!         Ok(())
//!     }
//!
//!     async fn handle(
//!         &self,
//!         _myself: ActorRef<Self::Msg>,
//!         msg: Self::Msg,
//!         _state: &mut Self::State,
//!     ) -> Result<(), ractor::ActorProcessingErr> {
//!         Ok(())
//!     }
//! }
//! ```

use std::future::Future;
use std::pin::Pin;

use ractor::{Actor, ActorRef};
use ractor::concurrency::JoinHandle;
use ractor::SpawnErr as RactorSpawnErr;
use tokio::sync::{mpsc, oneshot};

use crate::bus::EventBus;

// Re-export ractor types for convenience
pub use ractor::{async_trait, ActorName};

// ── RactorActor wrapper ────────────────────────────────────────────────────────

/// Wrapper around ractor that provides the same spawn interface as the custom runtime.
///
/// This allows gradual migration of actors to ractor without changing the spawn API.
#[allow(dead_code)]
pub struct RactorActor<A: Actor> {
    actor: A,
    _phantom: std::marker::PhantomData<A>,
}

impl<A: Actor> RactorActor<A> {
    /// Create a new RactorActor wrapper.
    #[allow(dead_code)]
    pub fn new(actor: A) -> Self {
        Self {
            actor,
            _phantom: std::marker::PhantomData,
        }
    }
}

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
    Ok((RactorHandle { actor_ref: actor_ref.clone() }, handle, actor_ref.get_cell()))
}

/// Future type returned by actor spawn.
pub type RactorFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

// ── Event bus integration ──────────────────────────────────────────────────────

/// Bridge between ractor and the EventBus.
///
/// This allows ractor actors to publish events to the shared EventBus.
pub struct EventBusBridge<E: Clone + Send + 'static> {
    bus: EventBus<E>,
}

impl<E: Clone + Send + 'static> EventBusBridge<E> {
    pub fn new(bus: EventBus<E>) -> Self {
        Self { bus }
    }

    /// Get a clone of the underlying event bus.
    pub fn bus(&self) -> &EventBus<E> {
        &self.bus
    }

    /// Publish an event to the bus.
    pub fn publish(&self, event: E) {
        self.bus.publish(event);
    }
}

impl<E: Clone + Send + 'static> Clone for EventBusBridge<E> {
    fn clone(&self) -> Self {
        Self {
            bus: self.bus.clone(),
        }
    }
}

impl<E: Clone + Send + 'static> std::fmt::Debug for EventBusBridge<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventBusBridge").finish()
    }
}

// ── GenericActorHandle ─────────────────────────────────────────────────────

/// Generic actor handle for sending typed messages.
///
/// Wraps `Arc<Sender<Msg>>` so the handle is always `Clone` regardless of `Msg`.
/// Used for fire-and-forget actors.
#[derive(Clone, Debug)]
pub struct GenericActorHandle<Msg: Clone> {
    tx: std::sync::Arc<mpsc::Sender<Msg>>,
}

impl<Msg: Clone> GenericActorHandle<Msg> {
    /// Wrap an existing sender.
    pub fn new(tx: mpsc::Sender<Msg>) -> Self {
        Self { tx: std::sync::Arc::new(tx) }
    }

    /// Access the underlying sender.
    pub fn inner(&self) -> &mpsc::Sender<Msg> {
        &self.tx
    }

    /// Send a message (async fire-and-forget).
    pub async fn send(&self, msg: Msg) {
        let _ = self.tx.send(msg).await;
    }

    /// Try to send a message (sync fire-and-forget; no-op if full).
    pub fn try_send(&self, msg: Msg) {
        let _ = self.tx.try_send(msg);
    }
}

// ── Reply wrapper (for RPC) ───────────────────────────────────────────────────

/// Alias for `RpcReply` for backward compatibility.
pub type Reply<T> = RpcReply<T>;

/// Wrapper for RPC reply channels, compatible with both runtimes.
/// Uses Arc<Mutex<Option<oneshot::Sender<T>>>> to allow cloning.
#[derive(Debug)]
pub struct RpcReply<T>(
    std::sync::Arc<std::sync::Mutex<Option<oneshot::Sender<T>>>>
);

impl<T> RpcReply<T> {
    /// Create a new reply handle from a oneshot sender.
    pub fn new(sender: oneshot::Sender<T>) -> Self {
        Self(std::sync::Arc::new(std::sync::Mutex::new(Some(sender))))
    }

    /// Send the reply value.
    pub fn send(self, value: T) {
        if let Some(sender) = self.0.lock().unwrap_or_else(|e| e.into_inner()).take() {
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

pub use ractor::{
    ActorErr, ActorProcessingErr, MessagingErr, RactorErr,
};

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that RactorActor can be spawned and messages sent.
    #[tokio::test]
    async fn ractor_actor_spawn_and_send() {
        struct TestActor;

        #[ractor::async_trait]
        impl Actor for TestActor {
            type Msg = String;
            type State = ();
            type Arguments = ();

            async fn pre_start(
                &self,
                myself: ActorRef<Self::Msg>,
                _: (),
            ) -> Result<Self::State, ractor::ActorProcessingErr> {
                Ok(())
            }

            async fn handle(
                &self,
                myself: ActorRef<Self::Msg>,
                msg: Self::Msg,
                _state: &mut Self::State,
            ) -> Result<(), ractor::ActorProcessingErr> {
                // Stop after receiving first message to end the test quickly
                myself.stop(None);
                assert_eq!(msg, "hello");
                Ok(())
            }
        }

        let (handle, join, cell) = spawn_ractor(None, TestActor, ()).await.unwrap();
        handle.send("hello".to_string()).await;
        drop(handle);
        
        // Wait for actor to stop
        tokio::time::timeout(std::time::Duration::from_secs(2), join).await.ok();
        
        // Actor should be stopped now
        assert!(matches!(
            cell.get_status(),
            ractor::ActorStatus::Stopped | ractor::ActorStatus::Unstarted
        ));
    }

    /// Test that EventBusBridge can publish events.
    #[tokio::test]
    async fn event_bus_bridge_publishes() {
        let bus = EventBus::<String>::new(16);
        let bridge = EventBusBridge::new(bus.clone());

        let mut sub = bus.subscribe();
        bridge.publish("test".to_string());

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
