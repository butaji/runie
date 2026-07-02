//! Turn handling logic for AgentActor.

use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use runie_core::actors::permission::RactorPermissionHandle;
use runie_core::permissions::PermissionGate;
use runie_core::permissions::{
    DefaultToolApprove, FileAccessAsk, GitTrackedWriteApprove, PermissionManager,
    PermissionSetPolicy,
};

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

/// Create a permission gate with a cancellation token for abort support.
pub(crate) async fn create_permission_gate(
    permission_handle: RactorPermissionHandle,
    abort_tx: CancellationToken,
) -> PermissionGate {
    // Load user permission rules from PermissionActor. This includes rules
    // from [[permissions]] in config.toml and any /trust decisions.
    let rules = permission_handle.get_rules().await;

    let permissions = PermissionManager::default().with_policies(vec![
        Box::new(DefaultToolApprove::new()),
        Box::new(GitTrackedWriteApprove::new()),
        Box::new(FileAccessAsk::new()),
        // User declarative rules — added last so they take precedence
        // (PermissionSetPolicy.evaluate always returns Some, winning the chain).
        Box::new(PermissionSetPolicy::new(rules)),
    ]);
    PermissionGate::new(
        permissions,
        Arc::new(EmitApprovalSink::with_cancel(
            permission_handle,
            60,
            abort_tx,
        )),
    )
}
