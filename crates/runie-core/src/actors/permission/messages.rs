//! Messages and handles for `PermissionActor`.

use tokio::sync::mpsc;

use crate::permissions::PermissionAction;

/// Messages accepted by `PermissionActor`.
#[derive(Debug)]
pub enum PermissionMsg {
    /// Agent requests permission to run a tool.
    /// The `reply` channel receives the permission action when resolved.
    AskPermission {
        request_id: String,
        tool: String,
        input: serde_json::Value,
        reply: Option<tokio::sync::oneshot::Sender<PermissionAction>>,
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
}

impl Clone for PermissionMsg {
    fn clone(&self) -> Self {
        // AskPermission cannot be cloned due to oneshot::Sender, so we drop the reply.
        // This is only used for internal routing, not for cloning user messages.
        match self {
            PermissionMsg::AskPermission { request_id, tool, input, .. } => {
                PermissionMsg::AskPermission {
                    request_id: request_id.clone(),
                    tool: tool.clone(),
                    input: input.clone(),
                    reply: None,
                }
            }
            PermissionMsg::ResolvePermission { request_id, action } => {
                PermissionMsg::ResolvePermission {
                    request_id: request_id.clone(),
                    action: action.clone(),
                }
            }
            PermissionMsg::CancelPermission { request_id } => {
                PermissionMsg::CancelPermission { request_id: request_id.clone() }
            }
            PermissionMsg::DismissRequest => PermissionMsg::DismissRequest,
        }
    }
}

/// Handle for sending commands to `PermissionActor`.
#[derive(Clone, Debug)]
pub struct PermissionActorHandle {
    tx: mpsc::Sender<PermissionMsg>,
}

impl PermissionActorHandle {
    /// Wrap an existing sender.
    pub fn new(tx: mpsc::Sender<PermissionMsg>) -> Self {
        Self { tx }
    }

    /// Request permission for a tool call. Returns a receiver for the response.
    pub async fn ask_permission(
        &self,
        request_id: String,
        tool: String,
        input: serde_json::Value,
    ) -> tokio::sync::oneshot::Receiver<PermissionAction> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let msg = PermissionMsg::AskPermission {
            request_id,
            tool,
            input,
            reply: Some(tx),
        };
        let _ = self.tx.send(msg).await;
        rx
    }

    /// Resolve a pending permission request.
    pub async fn resolve_permission(&self, request_id: String, action: PermissionAction) {
        let _ = self
            .tx
            .send(PermissionMsg::ResolvePermission {
                request_id,
                action,
            })
            .await;
    }

    /// Cancel a pending permission request.
    pub async fn cancel_permission(&self, request_id: String) {
        let _ = self
            .tx
            .send(PermissionMsg::CancelPermission { request_id })
            .await;
    }

    /// Dismiss the permission request UI.
    pub async fn dismiss(&self) {
        let _ = self.tx.send(PermissionMsg::DismissRequest).await;
    }

    /// Resolve a pending permission request (sync fire-and-forget).
    pub fn try_resolve_permission(&self, request_id: String, action: PermissionAction) {
        let _ = self.tx.try_send(PermissionMsg::ResolvePermission {
            request_id,
            action,
        });
    }

    /// Cancel a pending permission request (sync fire-and-forget).
    pub fn try_cancel_permission(&self, request_id: String) {
        let _ = self
            .tx
            .try_send(PermissionMsg::CancelPermission { request_id });
    }

    /// Dismiss the permission request UI (sync fire-and-forget).
    pub fn try_dismiss(&self) {
        let _ = self.tx.try_send(PermissionMsg::DismissRequest);
    }
}
