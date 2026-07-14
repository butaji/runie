//! Turn handling logic for AgentActor.

use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use runie_core::actors::permission::RactorPermissionHandle;
use runie_core::permissions::PermissionGate;

use crate::constants::DEFAULT_PERMISSION_TIMEOUT_SECS;
#[cfg(feature = "mcp")]
use runie_core::permissions::DefaultToolApprove;
#[cfg(feature = "git")]
use runie_core::permissions::GitTrackedWriteApprove;
use runie_core::permissions::{
    AutoApprove, FileAccessAsk, PermissionManager, PermissionMode, PermissionSet,
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
    // Session permission mode (see `/auto`): Auto prepends an auto-approve
    // policy so read, edit and shell tools skip the ask round-trip.
    let mode = permission_handle.get_mode().await;

    PermissionGate::new(
        PermissionManager::default().with_policies(policies_for_mode(mode, rules)),
        Arc::new(EmitApprovalSink::with_cancel(
            permission_handle,
            DEFAULT_PERMISSION_TIMEOUT_SECS,
            abort_tx,
        )),
    )
}

/// Build the permission policy chain for the given session mode.
///
/// In `Auto` mode an [`AutoApprove`] policy is prepended so read, edit and
/// shell tools are allowed without confirmation (sensitive paths still ask).
/// Every other mode keeps the manual chain unchanged.
pub(crate) fn policies_for_mode(
    mode: PermissionMode,
    rules: PermissionSet,
) -> Vec<Box<dyn runie_core::permissions::PermissionPolicy>> {
    let mut policies: Vec<Box<dyn runie_core::permissions::PermissionPolicy>> = Vec::new();
    if mode == PermissionMode::Auto {
        policies.push(Box::new(AutoApprove::new()));
    }
    policies.push(Box::new(FileAccessAsk::new()));
    // User declarative rules — added last so they take precedence
    // (PermissionSetPolicy.evaluate always returns Some, winning the chain).
    policies.push(Box::new(PermissionSetPolicy::new(rules)));
    #[cfg(feature = "mcp")]
    policies.push(Box::new(DefaultToolApprove::new()));
    #[cfg(feature = "git")]
    policies.push(Box::new(GitTrackedWriteApprove::new()));
    policies
}

#[cfg(test)]
mod tests {
    use super::*;
    use runie_core::permissions::{
        DenyAllSink, PermissionAction, PermissionContext, PermissionMode, PermissionSet,
    };
    use std::sync::Arc;

    /// Gate with a deny-all sink: any ask round-trip resolves to Deny, so
    /// `Allow` proves the policy chain approved without consulting the user.
    fn gate_for_mode(mode: PermissionMode) -> PermissionGate {
        PermissionGate::new(
            PermissionManager::default()
                .with_policies(policies_for_mode(mode, PermissionSet::default())),
            Arc::new(DenyAllSink),
        )
    }

    fn tool_ctx<'a>(tool: &'a str, path: Option<&'a std::path::Path>) -> PermissionContext<'a> {
        PermissionContext {
            tool,
            path,
            input: None,
            cwd: None,
            #[cfg(feature = "mcp")]
            annotations: runie_core::tool::annotations::get_tool_annotations(tool),
        }
    }

    #[tokio::test]
    async fn auto_mode_allows_read_edit_and_shell_tools() {
        let gate = gate_for_mode(PermissionMode::Auto);
        for tool in ["list_dir", "read_file", "write_file", "edit_file", "bash"] {
            let ctx = tool_ctx(tool, None);
            assert_eq!(
                gate.evaluate(&ctx).await,
                PermissionAction::Allow,
                "{tool} should be auto-approved without an ask round-trip"
            );
        }
    }

    #[tokio::test]
    async fn default_mode_still_asks_for_tools() {
        let gate = gate_for_mode(PermissionMode::Default);
        let ctx = tool_ctx("list_dir", None);
        // Manual mode: no policy allows list_dir, so the ask round-trip hits
        // the deny-all sink.
        assert_eq!(gate.evaluate(&ctx).await, PermissionAction::Deny);
    }

    #[tokio::test]
    async fn auto_mode_still_asks_for_sensitive_paths() {
        let gate = gate_for_mode(PermissionMode::Auto);
        let ctx = tool_ctx("read_file", Some(std::path::Path::new("/project/.env")));
        // Auto mode defers to the sink for sensitive paths (deny-all → Deny).
        assert_eq!(gate.evaluate(&ctx).await, PermissionAction::Deny);
    }
}
