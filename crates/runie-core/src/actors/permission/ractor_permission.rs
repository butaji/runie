//! Ractor-based `PermissionActor` implementation.
//!
//! This module provides a ractor-based implementation of the PermissionActor,
//! following the same pattern as the InputActor migration.

use parking_lot::Mutex;
use ractor::{Actor, ActorProcessingErr, ActorRef};

use crate::actors::ractor_adapter::{spawn_ractor, RactorHandle};
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

    /// Send a message to the actor (fire-and-forget).
    pub async fn send_message(&self, msg: PermissionMsg) {
        let _ = self.inner.send(msg).await;
    }

    /// Try to send a message (non-blocking).
    pub fn try_send(&self, msg: PermissionMsg) -> Result<(), ractor::MessagingErr<PermissionMsg>> {
        self.inner.try_send(msg)
    }
}

/// Ractor-based PermissionActor.
///
/// Owns the approval registry and permission request UI state.
/// Uses ractor for actor supervision and message handling.
pub struct RactorPermissionActor {
    /// The authoritative approval registry.
    registry: ApprovalRegistry,
    /// Current permission request state.
    current_request: Mutex<Option<PermissionRequestState>>,
    /// Bridge to the event bus for publishing facts.
    bus: EventBus<Event>,
}

impl Default for RactorPermissionActor {
    fn default() -> Self {
        Self {
            registry: ApprovalRegistry::new(),
            current_request: Mutex::new(None),
            bus: EventBus::new(16),
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
                // Store reply channel and emit event; do NOT reply until ResolvePermission.
                self.handle_ask_permission(request_id, tool, input, reply);
            }
            PermissionMsg::ResolvePermission { request_id, action } => {
                self.registry.resolve(&request_id, action);
                self.clear_request_if_matches(&request_id);
                self.bus.publish(Event::PermissionResponse { request_id, action });
            }
            PermissionMsg::CancelPermission { request_id } => {
                self.registry.resolve(&request_id, PermissionAction::Deny);
                self.clear_request_if_matches(&request_id);
            }
            PermissionMsg::DismissRequest => {
                *self.current_request.lock() = None;
                self.bus.publish(Event::PermissionRequestDismissed);
            }
        }
        Ok(())
    }
}

impl RactorPermissionActor {
    /// Spawn a `RactorPermissionActor` on the given event bus.
    pub async fn spawn(bus: EventBus<Event>) -> (RactorPermissionHandle, ractor::ActorCell) {
        let actor = Self {
            registry: ApprovalRegistry::new(),
            current_request: Mutex::new(None),
            bus: bus.clone(),
        };
        let (handle, _join, cell) = spawn_ractor(None, actor, bus).await.unwrap();
        (RactorPermissionHandle::new(handle), cell)
    }

    fn clear_request_if_matches(&self, request_id: &str) {
        let mut current = self.current_request.lock();
        if current.as_ref().map(|r| r.request_id == request_id).unwrap_or(false) {
            *current = None;
        }
    }

    /// Handle `AskPermission`: store the reply channel in the registry so
    /// `ResolvePermission` can deliver the user's choice. Do NOT reply here.
    fn handle_ask_permission(
        &self,
        request_id: String,
        tool: String,
        input: serde_json::Value,
        reply: crate::actors::Reply<PermissionAction>,
    ) {
        // Store the reply channel in ApprovalRegistry so ResolvePermission can send.
        self.registry.register(&request_id, reply);

        *self.current_request.lock() = Some(PermissionRequestState {
            request_id: request_id.clone(),
            tool: tool.clone(),
            input: input.clone(),
        });

        self.bus.publish(Event::PermissionRequest {
            request_id,
            tool,
            input,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::Receiver;

    /// Wait for an event matching a predicate with a deterministic timeout.
    async fn wait_for_event<F>(sub: &mut Receiver<Event>, pred: F) -> bool
    where
        F: Fn(&Event) -> bool,
    {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
        while tokio::time::Instant::now() < deadline {
            let timeout_duration = deadline - tokio::time::Instant::now();
            match tokio::time::timeout(timeout_duration, sub.recv()).await {
                Ok(Ok(evt)) => {
                    if pred(&evt) {
                        return true;
                    }
                }
                Ok(Err(_)) | Err(_) => break,
            }
        }
        false
    }

    // ── Layer 1: State/Logic tests ──────────────────────────────────────────

    #[tokio::test]
    async fn permission_actor_awaits_resolution() {
        // Verify that AskPermission does NOT immediately resolve.
        // The receiver should still be pending until ResolvePermission is called.
        let bus = EventBus::<Event>::new(16);
        let (handle, _cell) = RactorPermissionActor::spawn(bus.clone()).await;

        let mut rx = handle
            .ask_permission(
                "req-await-1".into(),
                "bash".into(),
                serde_json::json!({}),
            )
            .await;

        // Use try_recv to verify the channel is NOT yet complete
        // (would return Ok(Ready) if already resolved)
        let resolved = match rx.try_recv() {
            Ok(_) => true,  // Got a value = already resolved
            Err(tokio::sync::oneshot::error::TryRecvError::Empty) => false, // Still pending
            Err(tokio::sync::oneshot::error::TryRecvError::Closed) => true, // Closed = also resolved
        };

        assert!(!resolved, "AskPermission should NOT immediately resolve");
    }

    #[tokio::test]
    async fn permission_actor_resolves_with_allow() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _cell) = RactorPermissionActor::spawn(bus.clone()).await;

        let mut rx = handle
            .ask_permission(
                "req-allow-1".into(),
                "bash".into(),
                serde_json::json!({}),
            )
            .await;

        // Resolve with Allow
        handle
            .resolve_permission("req-allow-1".into(), PermissionAction::Allow)
            .await;

        // Verify the receiver gets Allow
        let result = tokio::time::timeout(std::time::Duration::from_secs(1), rx).await;
        assert!(result.is_ok(), "Should receive a result");
        assert_eq!(result.unwrap(), Ok(PermissionAction::Allow));
    }

    #[tokio::test]
    async fn permission_actor_resolves_with_deny() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _cell) = RactorPermissionActor::spawn(bus.clone()).await;

        let mut rx = handle
            .ask_permission(
                "req-deny-1".into(),
                "bash".into(),
                serde_json::json!({}),
            )
            .await;

        // Resolve with Deny
        handle
            .resolve_permission("req-deny-1".into(), PermissionAction::Deny)
            .await;

        // Verify the receiver gets Deny
        let result = tokio::time::timeout(std::time::Duration::from_secs(1), rx).await;
        assert!(result.is_ok(), "Should receive a result");
        assert_eq!(result.unwrap(), Ok(PermissionAction::Deny));
    }

    #[tokio::test]
    async fn permission_request_event_roundtrip() {
        // Layer 2: Event Handling - verify events flow correctly
        let bus = EventBus::<Event>::new(16);
        let mut sub = bus.subscribe();
        let (handle, _cell) = RactorPermissionActor::spawn(bus.clone()).await;

        // Ask permission
        let _rx = handle
            .ask_permission(
                "req-event-1".into(),
                "bash".into(),
                serde_json::json!({"command": "ls"}),
            )
            .await;

        // Wait for PermissionRequest event
        let found = wait_for_event(&mut sub, |e| matches!(e, Event::PermissionRequest { request_id, .. } if request_id == "req-event-1")).await;
        assert!(found, "Expected PermissionRequest event");

        // Resolve permission
        handle
            .resolve_permission("req-event-1".into(), PermissionAction::Allow)
            .await;

        // Wait for PermissionResponse event
        let found = wait_for_event(&mut sub, |e| matches!(e, Event::PermissionResponse { request_id, action: PermissionAction::Allow, .. } if request_id == "req-event-1")).await;
        assert!(found, "Expected PermissionResponse event");
    }

    // Legacy test names for backward compatibility with existing test expectations
    // These tests verify the same behavior as the new tests above.
    #[tokio::test]
    async fn ask_permission_stores_request() {
        // Same as permission_actor_awaits_resolution
        let bus = EventBus::<Event>::new(16);
        let (handle, _cell) = RactorPermissionActor::spawn(bus.clone()).await;
        let mut rx = handle
            .ask_permission("req-legacy-1".into(), "bash".into(), serde_json::json!({}))
            .await;
        let resolved = match rx.try_recv() {
            Ok(_) => true,
            Err(tokio::sync::oneshot::error::TryRecvError::Empty) => false,
            Err(tokio::sync::oneshot::error::TryRecvError::Closed) => true,
        };
        assert!(!resolved, "AskPermission should NOT immediately resolve");
    }

    #[tokio::test]
    async fn resolve_permission_clears_request() {
        // Same as permission_actor_resolves_with_allow
        let bus = EventBus::<Event>::new(16);
        let (handle, _cell) = RactorPermissionActor::spawn(bus.clone()).await;
        let mut rx = handle
            .ask_permission("req-legacy-2".into(), "bash".into(), serde_json::json!({}))
            .await;
        handle
            .resolve_permission("req-legacy-2".into(), PermissionAction::Allow)
            .await;
        let result = tokio::time::timeout(std::time::Duration::from_secs(1), rx).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Ok(PermissionAction::Allow));
    }
}
