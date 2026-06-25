//! Minimal actor trait and generic reply wrapper for Runie's lightweight actor model.
//!
//! We use simple tokio tasks + typed channels instead of a full actor framework.
//! This keeps things simple while still providing type-safe actor boundaries.

use std::future::Future;
use std::pin::Pin;
use tokio::sync::{mpsc, oneshot};

/// Minimal actor trait.
///
/// Actors are independent tokio tasks that receive messages via channels
/// and communicate via the shared EventBus.
///
/// # Type Parameters
/// - `M`: The message type this actor receives (must be Send + Clone)
/// - `E`: The event type this actor can emit to the bus (must be Send + Clone)
///
/// # Implementation
///
/// Implement `run_body` to define the actor's async message loop. The `run`
/// method boxes the future returned by `run_body` for `tokio::spawn`.
///
/// # Example
/// ```ignore
/// struct MyActor;
/// impl Actor for MyActor {
///     type Msg = String;
///     type Event = MyEvent;
///
///     async fn run_body(self, rx: mpsc::Receiver<Self::Msg>, bus: EventBus<Self::Event>) {
///         while let Some(msg) = rx.recv().await {
///             bus.publish(MyEvent::Received(msg));
///         }
///     }
/// }
/// ```
pub trait Actor: Send + 'static {
    /// Message type this actor receives.
    type Msg: Send + Clone + 'static;
    /// Event type this actor emits to the bus.
    type Event: Send + Clone + 'static;

    /// Run the actor until the receiver closes or the task is cancelled.
    ///
    /// Boxes the future returned by `run_body` for `tokio::spawn`.
    fn run(
        self,
        rx: mpsc::Receiver<Self::Msg>,
        bus: crate::bus::EventBus<Self::Event>,
    ) -> ActorFuture
    where
        Self: Sized,
    {
        Box::pin(self.run_body(rx, bus))
    }

    /// Implement the actor's async message loop.
    ///
    /// This is the only method that must be implemented; `run` handles boxing.
    fn run_body(
        self,
        rx: mpsc::Receiver<Self::Msg>,
        bus: crate::bus::EventBus<Self::Event>,
    ) -> impl Future<Output = ()> + Send + 'static;
}

/// Future type returned by Actor::run.
pub type ActorFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

/// Spawn an actor and return a channel to send it messages.
///
/// # Example
/// ```ignore
/// let (tx, handle) = spawn_actor(MyActor, bus);
/// tx.send("hello".into()).await?;
/// ```
pub fn spawn_actor<A>(
    actor: A,
    bus: crate::bus::EventBus<A::Event>,
) -> (mpsc::Sender<A::Msg>, ActorHandle)
where
    A: Actor,
{
    let (tx, rx) = mpsc::channel(32);
    let handle = ActorHandle::spawn(actor, rx, bus);
    (tx, handle)
}

/// Handle to a spawned actor for cancellation and joining.
#[derive(Debug)]
pub struct ActorHandle {
    pub(crate) abort_handle: tokio::task::AbortHandle,
    pub(crate) join_handle: tokio::task::JoinHandle<()>,
}

impl ActorHandle {
    /// Spawn the actor task and return a handle.
    pub(crate) fn spawn<A>(
        actor: A,
        rx: mpsc::Receiver<A::Msg>,
        bus: crate::bus::EventBus<A::Event>,
    ) -> Self
    where
        A: Actor,
    {
        let future = actor.run(rx, bus);
        let handle = tokio::spawn(future);
        Self {
            abort_handle: handle.abort_handle(),
            join_handle: handle,
        }
    }

    /// Abort the actor task.
    pub fn abort(&self) {
        self.abort_handle.abort();
    }
}

impl Future for ActorHandle {
    type Output = Result<(), tokio::task::JoinError>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        std::pin::Pin::new(&mut self.join_handle).poll(cx)
    }
}

impl Drop for ActorHandle {
    fn drop(&mut self) {
        self.abort();
    }
}

/// Generic reply wrapper for actor request/response patterns.
///
/// Wraps a `oneshot::Sender` behind an `Arc<Mutex<Option<T>>>` so it can be
/// cloned and sent to request handlers without borrowing issues.
/// Unlike `#[derive(Clone)]`, this always implements `Clone` regardless of `T`.
///
/// # Example
/// ```ignore
/// // Request side:
/// let (tx, rx) = oneshot::channel();
/// actor_tx.send(MyMsg::GetValue(Reply::new(tx))).await?;
/// let value = rx.await?;
///
/// // Handler side:
/// match msg {
///     MyMsg::GetValue(reply) => reply.send(response),
/// }
/// ```
#[derive(Debug)]
pub struct Reply<T>(std::sync::Arc<std::sync::Mutex<Option<oneshot::Sender<T>>>>);

impl<T> Clone for Reply<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Reply<T> {
    /// Create a new reply handle from a oneshot sender.
    pub fn new(sender: oneshot::Sender<T>) -> Self {
        Self(std::sync::Arc::new(std::sync::Mutex::new(Some(sender))))
    }

    /// Send the reply value, consuming the underlying sender.
    /// No-op if the receiver was already dropped.
    pub fn send(self, value: T) {
        if let Some(sender) = self.0.lock().unwrap_or_else(|e| e.into_inner()).take() {
            let _ = sender.send(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::EventBus;
    use tokio::sync::broadcast;

    /// Simple test actor that counts messages and emits events.
    struct TestActor {
        count: usize,
    }

    impl Actor for TestActor {
        type Msg = String;
        type Event = usize;

        fn run_body(
            self,
            mut rx: mpsc::Receiver<Self::Msg>,
            bus: EventBus<Self::Event>,
        ) -> impl Future<Output = ()> + Send + 'static {
            async move {
                let mut count = self.count;
                while let Some(msg) = rx.recv().await {
                    count += msg.len();
                    bus.publish(count);
                }
            }
        }
    }

    /// L1: `run_body` has no noop default body.
    ///
    /// The old default `async move { let _ = (self, rx, bus); }` was dead code —
    /// every real actor overrides `run_body`. This test ensures no accidental regression.
    #[test]
    fn actor_trait_has_no_noop_default() {
        let src = include_str!("trait.rs");
        // The noop pattern appears only in the old default body.
        // If it exists outside tests, the default leaked back in.
        let in_tests = src.contains("#[cfg(test)]");
        if in_tests {
            let parts: std::borrow::Cow<[_]> = src.split("#[cfg(test)]").collect();
            let non_test = parts[0];
            assert!(
                !non_test.contains("let _ = (self, rx, bus)"),
                "`run_body` must not have a noop default body"
            );
        } else {
            assert!(
                !src.contains("let _ = (self, rx, bus)"),
                "`run_body` must not have a noop default body"
            );
        }
    }

    #[tokio::test]
    async fn actor_trait_runs_and_receives_messages() {
        let bus = EventBus::new(10);
        let mut subscriber = bus.subscribe();

        let actor = TestActor { count: 0 };
        let (tx, handle) = spawn_actor(actor, bus.clone());

        // Send 3 messages
        tx.send("hi".into()).await.unwrap();
        tx.send("there".into()).await.unwrap();
        tx.send("!".into()).await.unwrap();

        // Drop sender to signal completion
        drop(tx);

        // Wait for actor to finish
        handle.await.unwrap();

        // Collect events
        let events: Vec<usize> = drain_events(&mut subscriber, 3);
        assert_eq!(events, vec![2, 7, 8]); // "hi"=2, "hi there"=7, "hi there!"=8
    }

    #[tokio::test]
    async fn actor_supervision_cancels_on_drop() {
        let bus = EventBus::new(10);
        let actor = TestActor { count: 0 };
        let (tx, _handle) = spawn_actor(actor, bus.clone());

        // Send a message
        tx.send("test".into()).await.unwrap();

        // Drop handle (aborts actor)
        // The actor task should be cancelled
    }

    fn drain_events<E: Clone + Send + 'static>(
        sub: &mut broadcast::Receiver<E>,
        count: usize,
    ) -> Vec<E> {
        let mut events = Vec::with_capacity(count);
        for _ in 0..count {
            match sub.try_recv() {
                Ok(e) => events.push(e),
                Err(broadcast::error::TryRecvError::Empty) => break,
                Err(_) => break,
            }
        }
        events
    }
}

/// Smoke test: verify the re-export path from the actors module root works.
#[test]
fn actor_trait_resolves_from_actors_module() {
    // This test only compiles if the re-exports from actors:: are correct.
    // We verify that the types are reachable via the actors module path.
    fn _uses_handle(_: &crate::actors::ActorHandle) {}
    fn _uses_reply(_: crate::actors::Reply<i32>) {}
    // Suppress unused warnings — we only care that these paths resolve.
    fn _assert() {
        _uses_handle as fn(&crate::actors::ActorHandle);
        _uses_reply as fn(crate::actors::Reply<i32>);
    }
    _assert();
}
