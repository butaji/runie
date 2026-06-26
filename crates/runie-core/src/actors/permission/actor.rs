//! `PermissionActor` — sole owner of approval registry and permission UI state.

use tokio::sync::mpsc;

use crate::actors::{Actor, Reply};
use crate::bus::EventBus;
use crate::event::Event;
use crate::model::PermissionRequestState;
use crate::permissions::{ApprovalRegistry, PermissionAction};

use super::messages::{PermissionActorHandle, PermissionMsg};

/// Actor that owns the approval registry and permission request UI state.
pub struct PermissionActor {
    registry: ApprovalRegistry,
    current_request: Option<PermissionRequestState>,
}

impl Default for PermissionActor {
    fn default() -> Self {
        Self {
            registry: ApprovalRegistry::new(),
            current_request: None,
        }
    }
}

impl PermissionActor {
    /// Spawn a `PermissionActor` on the given event bus.
    pub fn spawn(
        bus: EventBus<Event>,
    ) -> (PermissionActorHandle, crate::actors::ActorHandle) {
        let (tx, rx) = mpsc::channel(32);
        let actor = Self::default();
        let handle = crate::actors::ActorHandle::spawn(actor, rx, bus);
        (PermissionActorHandle::new(tx), handle)
    }

    fn handle_ask_permission(&mut self, request_id: String, tool: String, input: serde_json::Value, reply: Reply<PermissionAction>, bus: &EventBus<Event>) {
        self.registry.register(&request_id);
        self.current_request = Some(PermissionRequestState {
            request_id: request_id.clone(),
            tool: tool.clone(),
            input: input.clone(),
        });
        bus.publish(Event::PermissionRequest {
            request_id,
            tool,
            input,
        });
        // Deny on timeout for now
        reply.send(PermissionAction::Deny);
    }

    fn handle_resolve_permission(&mut self, request_id: String, action: PermissionAction, bus: &EventBus<Event>) {
        self.registry.resolve(&request_id, action);
        self.clear_request_if_matches(&request_id);
        bus.publish(Event::PermissionResponse { request_id, action });
    }

    fn handle_cancel_permission(&mut self, request_id: String) {
        self.registry.resolve(&request_id, PermissionAction::Deny);
        self.clear_request_if_matches(&request_id);
    }

    fn handle_dismiss_request(&mut self, bus: &EventBus<Event>) {
        self.current_request = None;
        bus.publish(Event::PermissionRequestDismissed);
    }

    fn clear_request_if_matches(&mut self, request_id: &str) {
        if self.current_request.as_ref().map(|r| r.request_id == request_id).unwrap_or(false) {
            self.current_request = None;
        }
    }
}

impl Actor for PermissionActor {
    type Msg = PermissionMsg;
    type Event = Event;

    async fn run_body(mut self, mut rx: mpsc::Receiver<Self::Msg>, bus: EventBus<Event>) {
        while let Some(msg) = rx.recv().await {
            match msg {
                PermissionMsg::AskPermission { request_id, tool, input, reply } => {
                    self.handle_ask_permission(request_id, tool, input, reply, &bus);
                }
                PermissionMsg::ResolvePermission { request_id, action } => {
                    self.handle_resolve_permission(request_id, action, &bus);
                }
                PermissionMsg::CancelPermission { request_id } => {
                    self.handle_cancel_permission(request_id);
                }
                PermissionMsg::DismissRequest => {
                    self.handle_dismiss_request(&bus);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_actor_is_empty() {
        let actor = PermissionActor::default();
        assert!(actor.current_request.is_none());
    }

    #[test]
    fn ask_permission_stores_request() {
        let mut actor = PermissionActor::default();
        let bus = crate::bus::EventBus::<Event>::new(16);
        let (tx, _rx) = tokio::sync::oneshot::channel();
        actor.handle_ask_permission(
            "req-1".into(),
            "bash".into(),
            serde_json::json!({}),
            Reply::new(tx),
            &bus,
        );
        assert!(actor.current_request.is_some());
    }
}
