//! Explicit ownership for actor worker tasks.

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
    use tokio::sync::mpsc;

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
        let worker = tokio::spawn(async move {
            if let Some(reply) = rx.recv().await {
                let _ = reply.send(());
            }
        });
        assert!(mailbox_ack!(tx, |reply| reply));
        worker.await.expect("ack worker completes");
    }
}
