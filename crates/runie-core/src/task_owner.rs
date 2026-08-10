//! Explicit ownership for actor worker tasks.

use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinHandle;

/// Thin actor DSL for the recurring owned-worker construction.
///
/// The mailbox and worker loop remain explicit at each call site; the macro
/// only makes task ownership impossible to forget when spawning the worker.
#[macro_export]
macro_rules! spawn_owned_worker {
    ($future:expr) => {
        // OWNER: the actor handle returned with this TaskOwner.
        std::sync::Arc::new($crate::task_owner::TaskOwner::new(tokio::spawn($future)))
    };
}

pub use crate::spawn_owned_worker;

/// Actor DSL for the repeated mailbox/channel plus owned-worker setup.
/// Command enums and worker loops stay visible at each call site.
#[macro_export]
macro_rules! spawn_actor_worker {
    ($capacity:expr, $worker:expr) => {{
        let (tx, rx) = tokio::sync::mpsc::channel($capacity);
        let owner = $crate::spawn_owned_worker!($worker(rx));
        (tx, owner)
    }};
}

pub use crate::spawn_actor_worker;

/// Mailbox DSL for the repeated one-shot reply pattern used by actors.
#[macro_export]
macro_rules! mailbox_call {
    ($tx:expr, $command:expr, $default:expr) => {{
        let (reply_tx, mut reply_rx) = tokio::sync::mpsc::channel(1);
        let _ = $tx.send(($command)(reply_tx)).await;
        reply_rx.recv().await.unwrap_or($default)
    }};
}

pub use crate::mailbox_call;

/// Sends a command carrying a one-shot acknowledgement and waits until the
/// actor has reduced it. The command constructor stays at the call site so
/// the macro does not conceal mailbox semantics.
#[macro_export]
macro_rules! mailbox_ack {
    ($tx:expr, $command:expr) => {{
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        if $tx.send(($command)(reply_tx)).await.is_ok() {
            let _ = reply_rx.await;
            true
        } else {
            false
        }
    }};
}

pub use crate::mailbox_ack;

/// Actor DSL for reducing a non-empty batch through one acknowledged mailbox
/// command. Empty batches are a successful no-op, which keeps event mappers
/// declarative without repeating queue guards at every actor boundary.
#[macro_export]
macro_rules! mailbox_batch_ack {
    ($tx:expr, $messages:expr, $command:expr) => {{
        let messages = $messages;
        if messages.is_empty() {
            true
        } else {
            $crate::mailbox_ack!($tx, |reply| ($command)(messages, reply))
        }
    }};
}

pub use crate::mailbox_batch_ack;

/// Declare a thin domain handle around [`ReducerActor`]. The macro generates
/// only mechanical forwarding; the reducer itself remains an ordinary pure
/// function at the call site.
#[macro_export]
macro_rules! declare_reducer_actor {
    ($name:ident, $state:ty, $event:ty) => {
        #[derive(Clone, Debug)]
        pub struct $name($crate::task_owner::ReducerActor<$state, $event>);

        impl $name {
            pub fn with_capacity(
                capacity: usize,
                initial: $state,
                reduce: impl Fn(&mut $state, $event) + Send + 'static,
            ) -> Self {
                Self($crate::task_owner::ReducerActor::new(
                    capacity, initial, reduce,
                ))
            }

            pub fn from_parts(
                capacity: usize,
                initial: $state,
                reduce: impl Fn(&mut $state, $event) + Send + 'static,
            ) -> Self {
                Self::with_capacity(capacity, initial, reduce)
            }

            pub async fn apply(&self, event: $event) -> bool {
                self.0.apply(event).await
            }

            pub fn snapshot(&self) -> $state {
                self.0.snapshot()
            }

            pub fn borrow(&self) -> tokio::sync::watch::Ref<'_, $state> {
                self.0.borrow()
            }

            pub fn subscribe(&self) -> tokio::sync::watch::Receiver<$state> {
                self.0.subscribe()
            }

            pub fn shared_snapshot(&self) -> $crate::SharedSnapshot<$state> {
                self.0.shared_snapshot()
            }

            pub fn shared_subscribe(
                &self,
            ) -> tokio::sync::watch::Receiver<$crate::SharedSnapshot<$state>> {
                self.0.shared_subscribe()
            }
        }
    };
}

pub use crate::declare_reducer_actor;

/// Generic single-owner reducer actor.
///
/// Domain code supplies only the state and pure transition function. The
/// mailbox, acknowledgement, snapshot publication, and task ownership are
/// shared by all reducer actors.
#[derive(Clone)]
pub struct ReducerActor<S, E> {
    tx: mpsc::Sender<(E, oneshot::Sender<()>)>,
    snapshot: watch::Receiver<S>,
    shared_snapshot: watch::Receiver<crate::SharedSnapshot<S>>,
    _owner: std::sync::Arc<TaskOwner>,
}

impl<S, E> ReducerActor<S, E>
where
    S: Clone + Send + Sync + 'static,
    E: Send + 'static,
{
    pub fn new(capacity: usize, initial: S, reduce: impl Fn(&mut S, E) + Send + 'static) -> Self {
        let (snapshot_tx, snapshot) = watch::channel(initial.clone());
        let (shared_tx, shared_snapshot) =
            watch::channel(crate::SharedSnapshot::new(initial.clone()));
        let (tx, mut rx) = mpsc::channel::<(E, oneshot::Sender<()>)>(capacity);
        let owner = spawn_owned_worker!(async move {
            let mut state = initial;
            while let Some((event, reply)) = rx.recv().await {
                reduce(&mut state, event);
                crate::publish_shared_snapshot(&snapshot_tx, &shared_tx, state.clone());
                let _ = reply.send(());
            }
        });
        Self {
            tx,
            snapshot,
            shared_snapshot,
            _owner: owner,
        }
    }

    pub async fn apply(&self, event: E) -> bool {
        let (reply, acknowledged) = oneshot::channel();
        if self.tx.send((event, reply)).await.is_err() {
            return false;
        }
        acknowledged.await.is_ok()
    }

    pub fn snapshot(&self) -> S {
        self.snapshot.borrow().clone()
    }

    pub fn borrow(&self) -> watch::Ref<'_, S> {
        self.snapshot.borrow()
    }

    pub fn subscribe(&self) -> watch::Receiver<S> {
        self.snapshot.clone()
    }

    pub fn shared_snapshot(&self) -> crate::SharedSnapshot<S> {
        self.shared_snapshot.borrow().clone()
    }

    pub fn shared_subscribe(&self) -> watch::Receiver<crate::SharedSnapshot<S>> {
        self.shared_snapshot.clone()
    }
}

impl<S, E> std::fmt::Debug for ReducerActor<S, E> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ReducerActor")
            .finish_non_exhaustive()
    }
}

/// Keeps an actor's worker task attached to the actor's lifetime.
///
/// The handle is shared because public actor handles are cheap clones. The
/// final clone aborts the worker instead of detaching it from the runtime.
pub struct TaskOwner {
    handle: JoinHandle<()>,
}

impl TaskOwner {
    pub fn new(handle: JoinHandle<()>) -> Self {
        Self { handle }
    }
}

impl Drop for TaskOwner {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::ReducerActor;
    use tokio::sync::mpsc;

    crate::declare_reducer_actor!(TestReducerActor, u32, u32);

    #[tokio::test]
    async fn worker_macro_returns_shared_owner() {
        let owner = spawn_owned_worker!(async {});
        let clone = owner.clone();
        drop(owner);
        drop(clone);
    }

    #[tokio::test]
    async fn actor_worker_macro_wires_mailbox_to_owned_worker() {
        let (tx, owner) = spawn_actor_worker!(1, |mut rx: mpsc::Receiver<u8>| async move {
            let _ = rx.recv().await;
        });
        tx.send(7_u8).await.expect("actor mailbox is open");
        drop(tx);
        drop(owner);
    }

    #[tokio::test]
    async fn mailbox_ack_waits_for_actor_reduction() {
        let (tx, mut rx) = mpsc::channel::<tokio::sync::oneshot::Sender<()>>(1);
        // OWNER: mailbox_ack test; the handle is awaited before the test exits.
        // OWNER: mailbox batch DSL test; the handle is awaited before exit.
        let worker = tokio::spawn(async move {
            if let Some(reply) = rx.recv().await {
                let _ = reply.send(());
            }
        });
        assert!(mailbox_ack!(tx, |reply| reply));
        worker.await.expect("ack worker completes");
    }

    #[tokio::test]
    async fn mailbox_batch_ack_skips_empty_batches() {
        let (tx, mut rx) = mpsc::channel::<tokio::sync::oneshot::Sender<()>>(1);
        // OWNER: mailbox batch DSL test; the handle is awaited before exit.
        let worker = tokio::spawn(async move {
            assert!(rx.recv().await.is_none());
        });
        assert!(mailbox_batch_ack!(tx, Vec::<u8>::new(), |_, reply| reply));
        // The empty batch must not enqueue; close the mailbox explicitly so
        // the worker can prove it observed no command.
        drop(tx);
        worker.await.expect("empty batch does not enqueue");
    }

    #[tokio::test]
    async fn reducer_actor_serializes_events_and_publishes_snapshots() {
        let actor = ReducerActor::new(4, 0_u32, |state, event| *state += event);
        assert!(actor.apply(2).await);
        assert!(actor.apply(3).await);
        assert_eq!(actor.snapshot(), 5);
    }

    #[tokio::test]
    async fn reducer_actor_macro_keeps_domain_handle_mechanical() {
        let actor = TestReducerActor::from_parts(2, 4, |state, event| *state *= event);
        assert!(actor.apply(3).await);
        assert_eq!(actor.snapshot(), 12);
        assert_eq!(*actor.borrow(), 12);
        assert_eq!(*actor.shared_snapshot(), 12);
        assert_eq!(**actor.shared_subscribe().borrow(), 12);
        let _ = actor.subscribe();
    }
}
