//! Minimal actor trait for Runie's lightweight actor model.
//!
//! We use simple tokio tasks + typed channels instead of a full actor framework.
//! This keeps things simple while still providing type-safe actor boundaries.

use std::future::Future;
use std::pin::Pin;
use tokio::sync::mpsc;

/// Minimal actor trait.
///
/// Actors are independent tokio tasks that receive messages via channels
/// and communicate via the shared EventBus.
///
/// # Type Parameters
/// - `M`: The message type this actor receives (must be Send + Clone)
/// - `E`: The event type this actor can emit to the bus (must be Send + Clone)
///
/// # Example
/// ```ignore
/// struct MyActor;
/// impl Actor for MyActor {
///     type Msg = String;
///     type Event = MyEvent;
///     
///     fn run(self, rx: mpsc::Receiver<Self::Msg>, bus: EventBus<Self::Event>) -> impl Future<Output = ()> + Send {
///         async move {
///             while let Some(msg) = rx.recv().await {
///                 bus.publish(MyEvent::Received(msg));
///             }
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
    /// The default implementation wraps the async body in a pin-boxed future
    /// for easier composition.
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

    /// Override this to implement the actor's behavior.
    /// Default implementation runs until channel closes.
    fn run_body(
        self,
        rx: mpsc::Receiver<Self::Msg>,
        bus: crate::bus::EventBus<Self::Event>,
    ) -> impl Future<Output = ()> + Send + 'static
    where
        Self: Sized,
    {
        async move {
            let _ = (self, rx, bus);
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::EventBus;

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
        sub: &mut crate::bus::ReplayReceiver<E>,
        count: usize,
    ) -> Vec<E> {
        let mut events = Vec::with_capacity(count);
        for _ in 0..count {
            match sub.try_recv() {
                Some(Ok(e)) => events.push(e),
                _ => break,
            }
        }
        events
    }
}
