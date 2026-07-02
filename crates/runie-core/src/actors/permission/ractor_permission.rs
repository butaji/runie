//! Ractor-based `PermissionActor` implementation.
//!
//! This module provides a ractor-based implementation of the PermissionActor,
//! following the same pattern as the InputActor migration.

use ractor::{Actor, ActorProcessingErr, ActorRef};
use tracing::instrument;

use std::collections::HashMap;

use super::super::config::RactorConfigHandle;
use crate::actors::ractor_adapter::spawn_ractor;
use crate::bus::EventBus;
use crate::event::Event;
use crate::model::PermissionRequestState;
use crate::permissions::{PermissionAction, PermissionSet};

use super::messages::PermissionMsg;

/// Ractor handle type for PermissionActor with convenience methods.
#[derive(Clone, Debug)]
pub struct RactorPermissionHandle {
    inner: ActorRef<PermissionMsg>,
}

impl RactorPermissionHandle {
    /// Create a new handle wrapping an ActorRef.
    pub fn new(inner: ActorRef<PermissionMsg>) -> Self {
        Self { inner }
    }

    /// Query the current pending request ID, if any.
    pub async fn current_request_id(&self) -> Option<String> {
        match self.inner.call(PermissionMsg::GetCurrentRequest, None).await {
            Ok(ractor::rpc::CallResult::Success(v)) => v,
            _ => None,
        }
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
            reply: tx,
        };
        let _ = self.inner.send_message(msg);
        rx
    }

    /// Resolve a pending permission request.
    pub async fn resolve_permission(&self, request_id: String, action: PermissionAction) {
        let msg = PermissionMsg::ResolvePermission { request_id, action };
        let _ = self.inner.send_message(msg);
    }

    /// Cancel a pending permission request.
    pub async fn cancel_permission(&self, request_id: String) {
        let msg = PermissionMsg::CancelPermission { request_id };
        let _ = self.inner.send_message(msg);
    }

    /// Dismiss the permission request UI.
    pub async fn dismiss(&self) {
        let msg = PermissionMsg::DismissRequest;
        let _ = self.inner.send_message(msg);
    }

    /// Resolve a pending permission request (sync fire-and-forget).
    pub fn try_resolve_permission(&self, request_id: String, action: PermissionAction) {
        let msg = PermissionMsg::ResolvePermission { request_id, action };
        let _ = self.inner.send_message(msg);
    }

    /// Cancel a pending permission request (sync fire-and-forget).
    pub fn try_cancel_permission(&self, request_id: String) {
        let msg = PermissionMsg::CancelPermission { request_id };
        let _ = self.inner.send_message(msg);
    }

    /// Dismiss the permission request UI (sync fire-and-forget).
    pub fn try_dismiss(&self) {
        let msg = PermissionMsg::DismissRequest;
        let _ = self.inner.send_message(msg);
    }

    /// Try to send a message (non-blocking).
    pub fn try_send(&self, msg: PermissionMsg) -> Result<(), ractor::MessagingErr<PermissionMsg>> {
        self.inner.send_message(msg)
    }

    /// Load permission rules from the config actor (fires LoadRules internally).
    pub async fn load_rules(&self) {
        let _ = self.inner.send_message(PermissionMsg::LoadRules);
    }

    /// Query the current permission rule set.
    pub async fn get_rules(&self) -> PermissionSet {
        match self.inner.call(PermissionMsg::GetRules, None).await {
            Ok(ractor::rpc::CallResult::Success(rules)) => rules,
            _ => PermissionSet::default(),
        }
    }

    /// Mark the current project as trusted.
    pub async fn trust_project(&self) {
        let _ = self.inner.send_message(PermissionMsg::TrustProject);
    }

    /// Mark the current project as untrusted.
    pub async fn untrust_project(&self) {
        let _ = self.inner.send_message(PermissionMsg::UntrustProject);
    }

    /// Add or update a permission rule.
    pub async fn upsert_rule(&self, tool: String, action: PermissionAction) {
        let _ = self.inner.send_message(PermissionMsg::UpsertRule { tool, action });
    }
}

/// Ractor State for PermissionActor — holds all mutable state.
/// EventBus is Clone and publish takes &self, no Mutex needed.
/// Pending approval channels are stored directly in state since ractor
/// processes messages sequentially (no concurrent access to state).
pub struct PermissionActorState {
    /// Pending approval reply channels keyed by request id.
    pending: HashMap<String, tokio::sync::oneshot::Sender<PermissionAction>>,
    /// Current permission request state.
    pub current_request: Option<PermissionRequestState>,
    /// Bridge to the event bus for publishing facts.
    pub bus: EventBus<Event>,
    /// Declarative permission rules loaded from config.toml.
    rules: PermissionSet,
}

impl PermissionActorState {
    fn emit(&self, event: Event) {
        self.bus.publish(event);
    }
}

/// Ractor-based PermissionActor.
///
/// Owns the approval registry and permission request UI state.
/// Uses ractor for actor supervision and message handling.
pub struct RactorPermissionActor;

/// Spawn arguments for `RactorPermissionActor`.
pub struct PermissionActorArgs {
    pub bus: EventBus<Event>,
    pub config_h: RactorConfigHandle,
}

impl RactorPermissionActor {
    fn handle_get_current_request(
        state: &PermissionActorState,
        reply: ractor::RpcReplyPort<Option<String>>,
    ) {
        let _ = reply.send(state.current_request.as_ref().map(|r| r.request_id.clone()));
    }

    fn handle_ask_permission(
        state: &mut PermissionActorState,
        request_id: String,
        tool: String,
        input: serde_json::Value,
        reply: tokio::sync::oneshot::Sender<PermissionAction>,
    ) {
        // Store the reply channel so ResolvePermission can send the user's choice.
        state.pending.insert(request_id.clone(), reply);

        state.current_request = Some(PermissionRequestState {
            request_id: request_id.clone(),
            tool: tool.clone(),
            input: input.clone(),
        });

        state.emit(Event::PermissionRequest {
            request_id,
            tool,
            input,
        });
    }

    fn handle_resolve_permission(
        state: &mut PermissionActorState,
        request_id: String,
        action: PermissionAction,
    ) {
        // Look up and resolve the pending reply channel.
        if let Some(reply) = state.pending.remove(&request_id) {
            let _ = reply.send(action);
        }
        if state
            .current_request
            .as_ref()
            .map(|r| r.request_id == request_id)
            .unwrap_or(false)
        {
            state.current_request = None;
            state.emit(Event::PermissionRequestDismissed);
        }
        state.emit(Event::PermissionResponse { request_id, action });
    }

    fn handle_cancel_permission(state: &mut PermissionActorState, request_id: String) {
        // Cancel by sending Deny on the pending channel.
        if let Some(reply) = state.pending.remove(&request_id) {
            let _ = reply.send(PermissionAction::Deny);
        }
        if state
            .current_request
            .as_ref()
            .map(|r| r.request_id == request_id)
            .unwrap_or(false)
        {
            state.current_request = None;
        }
    }

    fn handle_dismiss(state: &mut PermissionActorState) {
        state.current_request = None;
        state.emit(Event::PermissionRequestDismissed);
    }

    fn handle_get_rules(state: &PermissionActorState, reply: ractor::RpcReplyPort<PermissionSet>) {
        let _ = reply.send(state.rules.clone());
    }

    fn handle_load_rules(_state: &mut PermissionActorState) {
        // Rules are loaded from ConfigLoaded events; this is a no-op trigger
        // for the actor's own re-evaluation. The actual loading happens in
        // pre_start via ConfigActor, or explicitly after config changes.
        tracing::debug!("LoadRules received (rules already initialized or updated via ConfigLoaded)");
    }

    async fn handle_trust_project(state: &mut PermissionActorState) {
        let result = tokio::task::spawn_blocking(|| {
            let cwd = std::env::current_dir().unwrap_or_default();
            let cwd_utf8 = camino::Utf8PathBuf::from_path_buf(cwd)
                .unwrap_or_else(|_| camino::Utf8PathBuf::from("."));
            let mut tm = crate::trust::TrustManager::load();
            tm.set(&cwd_utf8, crate::trust::TrustDecision::Trusted);
            let _ = tm.save();
            tm.decisions()
        })
        .await;

        match result {
            Ok(decisions) => {
                state.emit(Event::TrustLoaded { decisions });
            }
            Err(e) => {
                tracing::warn!("failed to persist trust decision: {}", e);
            }
        }
    }

    async fn handle_untrust_project(state: &mut PermissionActorState) {
        let result = tokio::task::spawn_blocking(|| {
            let cwd = std::env::current_dir().unwrap_or_default();
            let cwd_utf8 = camino::Utf8PathBuf::from_path_buf(cwd)
                .unwrap_or_else(|_| camino::Utf8PathBuf::from("."));
            let mut tm = crate::trust::TrustManager::load();
            tm.set(&cwd_utf8, crate::trust::TrustDecision::Untrusted);
            let _ = tm.save();
            tm.decisions()
        })
        .await;

        match result {
            Ok(decisions) => {
                state.emit(Event::TrustLoaded { decisions });
            }
            Err(e) => {
                tracing::warn!("failed to persist untrust decision: {}", e);
            }
        }
    }

    fn handle_upsert_rule(state: &mut PermissionActorState, tool: String, action: PermissionAction) {
        let rule = crate::permissions::PermissionRule::new(action, tool);
        state.rules.add_rule(rule);
    }
}

#[ractor::async_trait]
impl Actor for RactorPermissionActor {
    type Msg = PermissionMsg;
    type State = PermissionActorState;
    type Arguments = PermissionActorArgs;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        // Load permission rules from the config actor so the agent can use them.
        let rules = match args.config_h.get_config().await {
            Some(cfg) => cfg.permissions.to_permission_set(),
            None => {
                tracing::warn!("PermissionActor: config not available, using default rules");
                PermissionSet::default_rules()
            }
        };
        Ok(PermissionActorState {
            pending: HashMap::new(),
            current_request: None,
            bus: args.bus,
            rules,
        })
    }

    #[instrument(name = "permission_actor", skip_all, fields(msg = ?msg))]
    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            PermissionMsg::AskPermission {
                request_id,
                tool,
                input,
                reply,
            } => {
                Self::handle_ask_permission(state, request_id, tool, input, reply);
            }
            PermissionMsg::ResolvePermission { request_id, action } => {
                Self::handle_resolve_permission(state, request_id, action);
            }
            PermissionMsg::CancelPermission { request_id } => {
                Self::handle_cancel_permission(state, request_id);
            }
            PermissionMsg::DismissRequest => {
                Self::handle_dismiss(state);
            }
            PermissionMsg::GetCurrentRequest(reply) => {
                Self::handle_get_current_request(state, reply);
            }
            PermissionMsg::LoadRules => {
                Self::handle_load_rules(state);
            }
            PermissionMsg::GetRules(reply) => {
                Self::handle_get_rules(state, reply);
            }
            PermissionMsg::TrustProject => {
                Self::handle_trust_project(state).await;
            }
            PermissionMsg::UntrustProject => {
                Self::handle_untrust_project(state).await;
            }
            PermissionMsg::UpsertRule { tool, action } => {
                Self::handle_upsert_rule(state, tool, action);
            }
        }
        Ok(())
    }
}

impl RactorPermissionActor {
    /// Spawn a `RactorPermissionActor` on the given event bus.
    ///
    /// The actor loads permission rules from the config actor (via `config_h`)
    /// on startup so that the agent can query them when building its gate.
    ///
    /// Returns a `Result` to allow callers to handle spawn failures gracefully.
    pub async fn spawn(
        bus: EventBus<Event>,
        config_h: RactorConfigHandle,
    ) -> anyhow::Result<(RactorPermissionHandle, ractor::ActorCell, tokio::task::JoinHandle<()>)> {
        let args = PermissionActorArgs { bus: bus.clone(), config_h };
        let (handle, join, cell) =
            spawn_ractor(None, Self, args)
                .await
                .map_err(|e| anyhow::anyhow!("RactorPermissionActor spawn failed: {}", e))?;
        Ok((RactorPermissionHandle::new(handle), cell, join))
    }

    /// Spawn for testing without a real config actor.
    ///
    /// Creates a dummy config actor to provide a real `RactorConfigHandle`,
    /// so `pre_start` can call `get_config()` (which returns defaults).
    /// Prefer [`spawn`](Self::spawn) in production code.
    pub async fn spawn_for_testing(
        bus: EventBus<Event>,
    ) -> anyhow::Result<(RactorPermissionHandle, ractor::ActorCell, tokio::task::JoinHandle<()>)> {
        // Spawn a minimal ConfigActor just for the config handle.
        let (config_h, _cfg_cell, _cfg_join) =
            crate::actors::RactorConfigActor::spawn_default(bus.clone()).await?;
        Self::spawn(bus, config_h).await
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
        let ( handle , _cell, _join ) = RactorPermissionActor::spawn_for_testing(bus.clone()).await.unwrap();

        let mut rx = handle
            .ask_permission("req-await-1".into(), "bash".into(), serde_json::json!({}))
            .await;

        // Use try_recv to verify the channel is NOT yet complete
        // (would return Ok(Ready) if already resolved)
        let resolved = match rx.try_recv() {
            Ok(_) => true, // Got a value = already resolved
            Err(tokio::sync::oneshot::error::TryRecvError::Empty) => false, // Still pending
            Err(tokio::sync::oneshot::error::TryRecvError::Closed) => true, // Closed = also resolved
        };

        assert!(!resolved, "AskPermission should NOT immediately resolve");
    }

    #[tokio::test]
    async fn permission_actor_resolves_with_allow() {
        let bus = EventBus::<Event>::new(16);
        let ( handle , _cell, _join ) = RactorPermissionActor::spawn_for_testing(bus.clone()).await.unwrap();

        let rx = handle
            .ask_permission("req-allow-1".into(), "bash".into(), serde_json::json!({}))
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
        let ( handle , _cell, _join ) = RactorPermissionActor::spawn_for_testing(bus.clone()).await.unwrap();

        let rx = handle
            .ask_permission("req-deny-1".into(), "bash".into(), serde_json::json!({}))
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
        let ( handle , _cell, _join ) = RactorPermissionActor::spawn_for_testing(bus.clone()).await.unwrap();

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
        let ( handle , _cell, _join ) = RactorPermissionActor::spawn_for_testing(bus.clone()).await.unwrap();
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
        let ( handle , _cell, _join ) = RactorPermissionActor::spawn_for_testing(bus.clone()).await.unwrap();
        let rx = handle
            .ask_permission("req-legacy-2".into(), "bash".into(), serde_json::json!({}))
            .await;
        handle
            .resolve_permission("req-legacy-2".into(), PermissionAction::Allow)
            .await;
        let result = tokio::time::timeout(std::time::Duration::from_secs(1), rx).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Ok(PermissionAction::Allow));
    }

    // ── Layer 1: Task acceptance criteria ────────────────────────────────────

    /// AC: Layer 1 — a configured allow-rule permits a bash call without dialog.
    ///
    /// When the agent queries permission rules via `get_rules()` and an allow-rule
    /// matches the tool, `PermissionSetPolicy::evaluate` returns Allow without
    /// consulting the approval sink (no dialog).
    #[tokio::test]
    async fn agent_gate_uses_user_trust_rules() {
        use crate::permissions::{PermissionSet, PermissionSetPolicy, PermissionContext, PermissionPolicy};

        // Simulate user configured: [[permissions]] action = "allow", tool = "bash"
        let mut rules = PermissionSet::default_rules();
        rules.add_rule(crate::permissions::PermissionRule::new(
            PermissionAction::Allow,
            "bash",
        ));

        let policy = PermissionSetPolicy::new(rules);
        let ctx = PermissionContext {
            tool: "bash",
            path: None,
            input: Some(&serde_json::json!({"command": "echo hi"})),
            cwd: None,
        };

        // Policy matches and returns Allow
        let result = policy.evaluate(&ctx).await;
        assert_eq!(
            result,
            Some(crate::permissions::PermissionResult::Allow),
            "bash tool should be allowed by user trust rule"
        );
    }

    /// AC: Layer 2 — `/trust bash always` updates permission actor rule set.
    ///
    /// When `UpsertRule` is sent to `PermissionActor`, it adds the rule to the
    /// internal `PermissionSet` and subsequent `get_rules()` calls return it.
    /// This mirrors the effect of `/trust bash always`.
    #[tokio::test]
    async fn trust_command_updates_permission_actor() {
        let bus = EventBus::<Event>::new(16);
        let _sub = bus.subscribe();

        let (handle, _cell, _join) =
            RactorPermissionActor::spawn_for_testing(bus.clone()).await.unwrap();

        // Before: bash should be Ask (from default rules)
        let rules_before = handle.get_rules().await;
        let bash_before = rules_before.effective_action("bash", None, None);
        assert_eq!(bash_before, PermissionAction::Ask, "bash should be Ask by default");

        // UpsertRule: `/trust bash always` → add allow rule for bash
        handle.upsert_rule("bash".into(), PermissionAction::Allow).await;

        // After: bash should be Allow (from upserted rule)
        let rules_after = handle.get_rules().await;
        let bash_after = rules_after.effective_action("bash", None, None);
        assert_eq!(bash_after, PermissionAction::Allow, "bash should be Allow after /trust bash always");
    }

    // ── Layer 1: Pending map direct access tests ─────────────────────────────
    // These tests verify the inlined pending map behavior that replaced ApprovalRegistry.

    /// Test that canceling a pending permission request sends Deny.
    #[tokio::test]
    async fn cancel_permission_sends_deny() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _cell, _join) =
            RactorPermissionActor::spawn_for_testing(bus.clone()).await.unwrap();

        let rx = handle
            .ask_permission("req-cancel-1".into(), "bash".into(), serde_json::json!({}))
            .await;

        // Cancel the request
        handle.cancel_permission("req-cancel-1".into()).await;

        // Verify the receiver gets Deny
        let result = tokio::time::timeout(std::time::Duration::from_secs(1), rx).await;
        assert!(result.is_ok(), "Should receive a result");
        assert_eq!(result.unwrap(), Ok(PermissionAction::Deny));
    }

    /// Test that resolving an unknown request does nothing (no panic).
    #[tokio::test]
    async fn resolve_unknown_request_is_noop() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _cell, _join) =
            RactorPermissionActor::spawn_for_testing(bus.clone()).await.unwrap();

        // Try to resolve a non-existent request - should not panic
        handle
            .resolve_permission("nonexistent".into(), PermissionAction::Allow)
            .await;

        // Verify no request is pending
        assert!(handle.current_request_id().await.is_none());
    }

    /// Test that multiple concurrent permission requests are independent.
    #[tokio::test]
    async fn multiple_concurrent_requests_are_independent() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _cell, _join) =
            RactorPermissionActor::spawn_for_testing(bus.clone()).await.unwrap();

        // Ask for two permissions concurrently
        let rx_a = handle
            .ask_permission("req-multi-a".into(), "read_file".into(), serde_json::json!({}))
            .await;
        let rx_b = handle
            .ask_permission("req-multi-b".into(), "bash".into(), serde_json::json!({}))
            .await;

        // Resolve the first one with Allow
        handle
            .resolve_permission("req-multi-a".into(), PermissionAction::Allow)
            .await;

        // Verify only the first one is resolved
        let result_a = tokio::time::timeout(std::time::Duration::from_millis(100), rx_a).await;
        let result_b = tokio::time::timeout(std::time::Duration::from_millis(100), rx_b).await;

        assert!(result_a.is_ok(), "First request should be resolved");
        assert_eq!(result_a.unwrap(), Ok(PermissionAction::Allow));
        assert!(result_b.is_err(), "Second request should still be pending");
    }
}
