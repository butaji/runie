//! Shared actor types and macros for mpsc-based actors.
//!
//! All actors follow the same mpsc pattern:
//! - Handle holds `tx: UnboundedSender<Msg>`
//! - Actor loop: `while let Some(msg) = rx.recv().await { handle(msg).await; }`
//! - Reads go direct via `Arc<RwLock<State>>` on the handle
//! - Writes go via `tx.send(msg)` fire-and-forget
//! - RPC calls use `Option<oneshot::Sender<T>>` inside the message

// ─────────────────────────────────────────────────────────────────────────────
// RpcError
// ─────────────────────────────────────────────────────────────────────────────

/// Error when the actor drops before sending an RPC reply.
#[derive(Debug)]
pub enum RpcError {
    ActorDropped,
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RpcError::ActorDropped => write!(f, "actor dropped"),
        }
    }
}

impl std::error::Error for RpcError {}

// ─────────────────────────────────────────────────────────────────────────────
// `spawn_actor!` — generates handle struct + spawn fn from a simple pattern
// ─────────────────────────────────────────────────────────────────────────────

/// Spawn an actor that processes messages with an async handler closure.
///
/// The closure/function is called for each message: `async fn(Msg, &mut State)`
///
/// # Example
/// ```
/// pub fn spawn_io_actor(bus: EventBus<Event>) -> (IoActorHandle, JoinHandle<()>) {
///     spawn_actor! {
///         IoActorHandle,
///         IoMsg,
///         IoActorState { bus },
///         async |state: &mut IoActorState, msg: IoMsg| {
///             match msg {
///                 IoMsg::RunBash { command, shell } => state.run_bash(command, shell).await,
///                 // ...
///             }
///         }
///     }
/// }
/// ```
#[macro_export]
macro_rules! spawn_actor {
    // No-state variant
    (
        $Handle:ident,
        $Msg:ty,
        no_state,
        async |$msg:ident: $MsgTy:ty| $block:block
    ) => {
        #[derive(Clone, Debug)]
        pub struct $Handle(pub tokio::sync::mpsc::UnboundedSender<$Msg>);

        #[allow(dead_code)]
        pub fn spawn_actor() -> ($Handle, tokio::task::JoinHandle<()>) {
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<$Msg>();
            let handle = $Handle(tx);
            let join = tokio::spawn(async move {
                while let Some($msg) = rx.recv().await {
                    $block
                }
            });
            (handle, join)
        }
    };

    // With state struct
    (
        $Handle:ident,
        $Msg:ty,
        $State:ty,
        async |$state:ident: $StateTy:ty, $msg:ident: $MsgTy:ty| $block:block
    ) => {
        #[derive(Clone, Debug)]
        pub struct $Handle(pub tokio::sync::mpsc::UnboundedSender<$Msg>);

        #[allow(dead_code)]
        pub fn spawn_actor($state: $StateTy) -> ($Handle, tokio::task::JoinHandle<()>) {
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<$Msg>();
            let handle = $Handle(tx);
            let join = tokio::spawn(async move {
                let mut $state = $state;
                while let Some($msg) = rx.recv().await {
                    $block
                }
            });
            (handle, join)
        }
    };

    // With state init + handler fn
    (
        $Handle:ident,
        $Msg:ty,
        $State:ty,
        init: $init:expr,
        handler: $handler:path
    ) => {
        #[derive(Clone, Debug)]
        pub struct $Handle(pub tokio::sync::mpsc::UnboundedSender<$Msg>);

        #[allow(dead_code)]
        pub fn spawn_actor($init) -> ($Handle, tokio::task::JoinHandle<()>) {
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<$Msg>();
            let handle = $Handle(tx);
            let join = tokio::spawn(async move {
                let mut state = $init;
                while let Some(msg) = rx.recv().await {
                    $handler(&mut state, msg).await;
                }
            });
            (handle, join)
        }
    };
}
