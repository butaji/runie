//! Ractor-based `PermissionActor` implementation.
//!
//! This module provides a ractor-based implementation of the PermissionActor,
//! following the same pattern as the InputActor migration.

use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::sync::Mutex;

use crate::actors::ractor_adapter::{spawn_ractor, EventBusBridge, RactorHandle};
use crate::bus::EventBus;
use crate::event::Event;
use crate::model::PermissionRequestState;
use crate::permissions::{ApprovalRegistry, PermissionAction};

use super::messages::PermissionMsg;

/// Ractor handle type for PermissionActor with convenience methods.
#[derive(Clone, Debug)]
pub struct RactorPermissionHandle {
    inner: RactorHandle<PermissionMsg>,
}

impl RactorPermissionHandle {
    /// Create a new handle wrapping the inner RactorHandle.
    pub fn new(inner: RactorHandle<PermissionMsg>) -> Self {
        Self { inner }
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
            reply: crate::actors::Reply::new(tx),
        };
        let _ = self.inner.send(msg).await;
        rx
    }

    /// Resolve a pending permission request.
    pub async fn resolve_permission(&self, request_id: String, action: PermissionAction) {
        let msg = PermissionMsg::ResolvePermission { request_id, action };
        let _ = self.inner.send(msg).await;
    }

    /// Cancel a pending permission request.
    pub async fn cancel_permission(&self, request_id: String) {
        let msg = PermissionMsg::CancelPermission { request_id };
        let _ = self.inner.send(msg).await;
    }

    /// Dismiss the permission request UI.
    pub async fn dismiss(&self) {
        let msg = PermissionMsg::DismissRequest;
        let _ = self.inner.send(msg).await;
    }

    /// Resolve a pending permission request (sync fire-and-forget).
    pub fn try_resolve_permission(&self, request_id: String, action: PermissionAction) {
        let msg = PermissionMsg::ResolvePermission { request_id, action };
        let _ = self.inner.try_send(msg);
    }

    /// Cancel a pending permission request (sync fire-and-forget).
    pub fn try_cancel_permission(&self, request_id: String) {
        let msg = PermissionMsg::CancelPermission { request_id };
        let _ = self.inner.try_send(msg);
    }

    /// Dismiss the permission request UI (sync fire-and-forget).
    pub fn try_dismiss(&self) {
        let msg = PermissionMsg::DismissRequest;
        let _ = self.inner.try_send(msg);
    }
}

/// Ractor-based PermissionActor.
///
/// Owns the approval registry and permission request UI state.
/// Uses ractor for actor supervision and message handling.
pub struct RactorPermissionActor {
    /// The authoritative approval registry.
    registry: Mutex<ApprovalRegistry>,
    /// Current permission request state.
    current_request: Mutex<Option<PermissionRequestState>>,
    /// Bridge to the event bus for publishing facts.
    bus_bridge: EventBusBridge<Event>,
}

impl Default for RactorPermissionActor {
    fn default() -> Self {
        Self {
            registry: Mutex::new(ApprovalRegistry::new()),
            current_request: Mutex::new(None),
            bus_bridge: EventBusBridge::new(EventBus::new(16)),
        }
    }
}

#[ractor::async_trait]
impl Actor for RactorPermissionActor {
    type Msg = PermissionMsg;
    type State = ();
    type Arguments = EventBus<Event>;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(())
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: PermissionMsg,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            PermissionMsg::AskPermission {
                request_id,
                tool,
                input,
                reply,
            } => {
                self.registry.lock().unwrap().register(&request_id);
                *self.current_request.lock().unwrap() = Some(PermissionRequestState {
                    request_id: request_id.clone(),
                    tool: tool.clone(),
                    input: input.clone(),
                });
                self.bus_bridge.publish(Event::PermissionRequest {
                    request_id: request_id.clone(),
                    tool: tool.clone(),
                    input,
                });
                reply.send(PermissionAction::Deny);
            }
            PermissionMsg::ResolvePermission { request_id, action } => {
                self.registry.lock().unwrap().resolve(&request_id, action);
                self.clear_request_if_matches(&request_id);
                self.bus_bridge.publish(Event::PermissionResponse { request_id, action });
            }
            PermissionMsg::CancelPermission { request_id } => {
                self.registry
                    .lock()
                    .unwrap()
                    .resolve(&request_id, PermissionAction::Deny);
                self.clear_request_if_matches(&request_id);
            }
            PermissionMsg::DismissRequest => {
                *self.current_request.lock().unwrap() = None;
                self.bus_bridge.publish(Event::PermissionRequestDismissed);
            }
        }
        Ok(())
    }
}

impl RactorPermissionActor {
    /// Spawn a `RactorPermissionActor` on the given event bus.
    pub async fn spawn(bus: EventBus<Event>) -> (RactorPermissionHandle, ractor::ActorCell) {
        let actor = Self {
            registry: Mutex::new(ApprovalRegistry::new()),
            current_request: Mutex::new(None),
            bus_bridge: EventBusBridge::new(bus.clone()),
        };
        let (handle, _join, cell) = spawn_ractor(None, actor, bus).await.unwrap();
        (RactorPermissionHandle::new(handle), cell)
    }

    fn clear_request_if_matches(&self, request_id: &str) {
        let mut current = self.current_request.lock().unwrap();
        if current
            .as_ref()
            .map(|r| r.request_id == request_id)
            .unwrap_or(false)
        {
            *current = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ask_permission_stores_request() {
        let bus = EventBus::<Event>::new(16);
        let mut sub = bus.subscribe();
        let (handle, _cell) = RactorPermissionActor::spawn(bus.clone()).await;

        handle
            .ask_permission(
                "req-1".into(),
                "bash".into(),
                serde_json::json!({}),
            )
            .await;

        // Give actor time to process
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let mut found_permission_request = false;
        for _ in 0..10 {
            if let Ok(e) = sub.try_recv() {
                if matches!(e, Event::PermissionRequest { .. }) {
                    found_permission_request = true;
                    break;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert!(found_permission_request, "Expected PermissionRequest event");
    }

    #[tokio::test]
    async fn resolve_permission_clears_request() {
        let bus = EventBus::<Event>::new(16);
        let mut sub = bus.subscribe();
        let (handle, _cell) = RactorPermissionActor::spawn(bus.clone()).await;

        // First ask permission
        handle
            .ask_permission(
                "req-2".into(),
                "read_file".into(),
                serde_json::json!({"path": "test.txt"}),
            )
            .await;

        // Then resolve it
        handle
            .resolve_permission("req-2".into(), PermissionAction::Allow)
            .await;

        // Give actor time to process
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let mut found_response = false;
        for _ in 0..10 {
            if let Ok(e) = sub.try_recv() {
                if matches!(e, Event::PermissionResponse { action: PermissionAction::Allow, .. }) {
                    found_response = true;
                    break;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert!(found_response, "Expected PermissionResponse event");
    }
}
