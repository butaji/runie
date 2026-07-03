//! Messages for `PermissionActor`.

use ractor::RpcReplyPort;
use tokio::sync::oneshot;
use crate::permissions::{PermissionAction, PermissionSet};

/// Messages accepted by `PermissionActor`.
#[derive(Debug)]
pub enum PermissionMsg {
    /// Agent requests permission to run a tool.
    /// The `reply` channel receives the permission action when resolved.
    AskPermission {
        request_id: String,
        tool: String,
        input: serde_json::Value,
        /// Optional reply channel. `Some(sender)` for RPC callers; `None` for fire-and-forget.
        reply: Option<oneshot::Sender<PermissionAction>>,
    },
    /// User resolves a pending permission request.
    ResolvePermission {
        request_id: String,
        action: PermissionAction,
    },
    /// Cancel a pending request (e.g., when starting new session).
    CancelPermission { request_id: String },
    /// Dismiss the permission request UI without resolving.
    DismissRequest,
    /// Query the current pending request ID (returns Option<String>).
    GetCurrentRequest(Option<RpcReplyPort<Option<String>>>),
    /// Load permission rules from the config actor.
    LoadRules,
    /// Query the current permission rule set.
    GetRules(Option<RpcReplyPort<PermissionSet>>),
    /// Mark the current project as trusted.
    TrustProject,
    /// Mark the current project as untrusted.
    UntrustProject,
    /// Add or update a permission rule.
    UpsertRule {
        tool: String,
        action: PermissionAction,
    },
}

impl Clone for PermissionMsg {
    fn clone(&self) -> Self {
        match self {
            PermissionMsg::AskPermission {
                request_id,
                tool,
                input,
                reply: _,
            } => PermissionMsg::AskPermission {
                request_id: request_id.clone(),
                tool: tool.clone(),
                input: input.clone(),
                reply: None, // Fire-and-forget; the original sender is not usable after move.
            },
            PermissionMsg::ResolvePermission { request_id, action } => {
                PermissionMsg::ResolvePermission {
                    request_id: request_id.clone(),
                    action: *action,
                }
            }
            PermissionMsg::CancelPermission { request_id } => PermissionMsg::CancelPermission {
                request_id: request_id.clone(),
            },
            PermissionMsg::DismissRequest => PermissionMsg::DismissRequest,
            PermissionMsg::GetCurrentRequest(_reply) => {
                PermissionMsg::GetCurrentRequest(None) // Fire-and-forget.
            }
            PermissionMsg::LoadRules => PermissionMsg::LoadRules,
            PermissionMsg::GetRules(_reply) => PermissionMsg::GetRules(None), // Fire-and-forget.
            PermissionMsg::TrustProject => PermissionMsg::TrustProject,
            PermissionMsg::UntrustProject => PermissionMsg::UntrustProject,
            PermissionMsg::UpsertRule { tool, action } => PermissionMsg::UpsertRule {
                tool: tool.clone(),
                action: *action,
            },
        }
    }
}
