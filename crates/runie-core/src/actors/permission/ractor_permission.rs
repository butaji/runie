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
        match self
            .inner
            .call(|tx| PermissionMsg::GetCurrentRequest(Some(tx)), None)
            .await
        {
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
            reply: Some(tx),
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

    /// Add or update a permission rule (sync fire-and-forget).
    pub fn try_upsert_rule(&self, tool: String, action: PermissionAction) {
        let msg = PermissionMsg::UpsertRule { tool, action };
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
        match self
            .inner
            .call(|tx| PermissionMsg::GetRules(Some(tx)), None)
            .await
        {
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
        let _ = self
            .inner
            .send_message(PermissionMsg::UpsertRule { tool, action });
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
        reply: Option<ractor::RpcReplyPort<Option<String>>>,
    ) {
        if let Some(reply) = reply {
            let _ = reply.send(state.current_request.as_ref().map(|r| r.request_id.clone()));
        }
    }

    fn handle_ask_permission(
        state: &mut PermissionActorState,
        request_id: String,
        tool: String,
        input: serde_json::Value,
        reply: Option<tokio::sync::oneshot::Sender<PermissionAction>>,
    ) {
        // Store the reply channel so ResolvePermission can send the user's choice.
        if let Some(reply) = reply {
            state.pending.insert(request_id.clone(), reply);
        }

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

    fn handle_get_rules(
        state: &PermissionActorState,
        reply: Option<ractor::RpcReplyPort<PermissionSet>>,
    ) {
        if let Some(reply) = reply {
            let _ = reply.send(state.rules.clone());
        }
    }

    fn handle_load_rules(_state: &mut PermissionActorState) {
        // Rules are loaded from ConfigLoaded events; this is a no-op trigger
        // for the actor's own re-evaluation. The actual loading happens in
        // pre_start via ConfigActor, or explicitly after config changes.
        tracing::debug!(
            "LoadRules received (rules already initialized or updated via ConfigLoaded)"
        );
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

    fn handle_upsert_rule(
        state: &mut PermissionActorState,
        tool: String,
        action: PermissionAction,
    ) {
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
    ) -> anyhow::Result<(
        RactorPermissionHandle,
        ractor::ActorCell,
        tokio::task::JoinHandle<()>,
    )> {
        let args = PermissionActorArgs {
            bus: bus.clone(),
            config_h,
        };
        let (handle, join, cell) = spawn_ractor(None, Self, args)
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
    ) -> anyhow::Result<(
        RactorPermissionHandle,
        ractor::ActorCell,
        tokio::task::JoinHandle<()>,
    )> {
        // Spawn a minimal ConfigActor just for the config handle.
        let (config_h, _cfg_cell, _cfg_join) =
            crate::actors::RactorConfigActor::spawn_default(bus.clone()).await?;
        Self::spawn(bus, config_h).await
    }
}

#[cfg(test)]
mod tests;
