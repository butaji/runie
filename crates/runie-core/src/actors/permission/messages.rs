//! Messages for `PermissionActor`.

use crate::actors::ractor_adapter::{Reply, RpcReply};
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
        reply: Reply<PermissionAction>,
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
    GetCurrentRequest(RpcReply<Option<String>>),
    /// Load permission rules from the config actor.
    LoadRules,
    /// Query the current permission rule set.
    GetRules(RpcReply<PermissionSet>),
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
        // AskPermission's Reply is Clone via Arc, so we can clone the whole message.
        match self {
            PermissionMsg::AskPermission {
                request_id,
                tool,
                input,
                reply,
            } => PermissionMsg::AskPermission {
                request_id: request_id.clone(),
                tool: tool.clone(),
                input: input.clone(),
                reply: reply.clone(),
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
            PermissionMsg::GetCurrentRequest(reply) => PermissionMsg::GetCurrentRequest(reply.clone()),
            PermissionMsg::LoadRules => PermissionMsg::LoadRules,
            PermissionMsg::GetRules(reply) => PermissionMsg::GetRules(reply.clone()),
            PermissionMsg::TrustProject => PermissionMsg::TrustProject,
            PermissionMsg::UntrustProject => PermissionMsg::UntrustProject,
            PermissionMsg::UpsertRule { tool, action } => PermissionMsg::UpsertRule {
                tool: tool.clone(),
                action: *action,
            },
        }
    }
}
