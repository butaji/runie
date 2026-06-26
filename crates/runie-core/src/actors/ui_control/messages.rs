//! Typed messages for `UiControlActor`.

use tokio::sync::mpsc;

use crate::commands::DialogState;
use crate::login_flow::LoginFlowState;

/// All messages accepted by `UiControlActor`.
///
/// Covers dialog lifecycle, login flow, and quit control.
#[derive(Debug, Clone)]
pub enum UiControlMsg {
    // ── Dialog lifecycle ───────────────────────────────────────────────────
    /// Open a dialog (replaces any current dialog).
    OpenDialog(DialogState),
    /// Push a dialog onto the back-stack, keeping the current one active.
    PushDialog(DialogState),
    /// Close the current dialog, restoring the one beneath it (if any).
    PopDialog,
    /// Close all open dialogs.
    CloseAllDialogs,

    // ── Login flow ─────────────────────────────────────────────────────────
    /// Start the login flow (clears any open dialogs).
    StartLoginFlow,
    /// Update login flow state from a step change.
    LoginFlowStep(LoginFlowState),
    /// Cancel and close the login flow.
    CancelLoginFlow,

    // ── Quit ───────────────────────────────────────────────────────────────
    /// Request normal quit (emit `QuitRequested` fact).
    RequestQuit,
    /// Force quit without cleanup (emit `QuitRequested` fact).
    ForceQuit,
}

/// Handle for sending messages to `UiControlActor`.
#[derive(Clone, Debug)]
pub struct UiControlActorHandle {
    tx: mpsc::Sender<UiControlMsg>,
}

impl UiControlActorHandle {
    /// Wrap an existing sender.
    pub fn new(tx: mpsc::Sender<UiControlMsg>) -> Self {
        Self { tx }
    }

    /// Send a message to the actor (async fire-and-forget).
    pub async fn send(&self, msg: UiControlMsg) {
        let _ = self.tx.send(msg).await;
    }

    /// Try to send a message (sync fire-and-forget).
    pub fn try_send(&self, msg: UiControlMsg) {
        let _ = self.tx.try_send(msg);
    }
}
