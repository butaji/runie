#![allow(clippy::too_many_lines)]

//! Turn handling logic for AgentActor.

use tokio_util::sync::CancellationToken;

use runie_core::actors::permission::RactorPermissionHandle;
use runie_core::permissions::PermissionGate;

use crate::emit_approval_sink::EmitApprovalSink;

use super::AgentActorState;

/// Abort the currently running turn.
/// Takes and cancels the current turn token and gate, then aborts the handle.
pub(crate) async fn abort_turn(state: &mut AgentActorState) {
    // Take and cancel the current turn token and gate.
    if let Some(token) = state.current_turn_token.take() {
        token.cancel();
    }
    if let Some(gate) = state.current_gate.take() {
        gate.cancel_pending();
    }
    // Abort and await the old turn handle.
    if let Some(handle) = state.current_turn_handle.take() {
        handle.abort();
        let _ = handle.await;
    }
}

/// Complete the turn normally.
/// Clears all in-flight state so a subsequent Run is accepted.
/// Do NOT cancel the token/gate — the turn finished on its own.
pub(crate) fn complete_turn(state: &mut AgentActorState) {
    state.current_turn_token = None;
    state.current_gate = None;
    state.current_turn_handle = None;
}

/// Create a permission gate — always bypasses (delegates to sink).
pub(crate) fn create_permission_gate(
    permission_handle: RactorPermissionHandle,
    _abort_tx: CancellationToken,
) -> PermissionGate {
    // Permission engine removed: all tools are allowed immediately.
    // EmitApprovalSink emits TUI events for UX but returns Allow.
    PermissionGate::new(std::sync::Arc::new(EmitApprovalSink::new(permission_handle)))
}
