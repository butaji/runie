//! Permission actor — stub. All permission checks now bypass.
//!
//! The policy engine has been removed. `PermissionGate` always delegates to the
//! approval sink. This actor is kept as a minimal stub so existing spawn sites
//! (leader, agent) continue to compile without structural changes.

use ractor::async_trait;
use ractor::{Actor, ActorCell, ActorProcessingErr, ActorRef};

use crate::actors::ractor_adapter::spawn_ractor;
use crate::actors::RactorConfigHandle;
use crate::bus::EventBus;
use crate::permissions::{PermissionAction, PermissionMode, PermissionSet};
use crate::Event;

use super::messages::PermissionMsg;

/// Stub handle — all permission checks bypass.
#[derive(Clone)]
pub struct RactorPermissionHandle {
    _cell: ActorCell,
}

impl RactorPermissionHandle {
    pub fn new(_cell: ActorCell) -> Self {
        Self { _cell }
    }

    /// Stub: always returns None (bypass, no UI needed).
    pub async fn ask_permission(
        &self,
        _request_id: String,
        _tool: String,
        _input: serde_json::Value,
    ) -> Option<PermissionAction> {
        None
    }

    pub fn try_cancel_permission(&self, _request_id: String) {}

    pub fn try_resolve_permission(&self, _request_id: String, _action: PermissionAction) {}

    pub fn try_upsert_rule(&self, _tool: String, _action: PermissionAction) {}

    pub fn try_upsert_session_rule(&self, _tool: String, _action: PermissionAction) {}

    pub fn try_send(&self, _msg: PermissionMsg) -> Result<(), ractor::MessagingErr<PermissionMsg>> {
        Ok(())
    }

    pub async fn load_rules(&self) {}

    pub async fn get_rules(&self) -> PermissionSet {
        PermissionSet::default()
    }

    pub fn set_mode(&self, _mode: PermissionMode) {}

    pub async fn get_mode(&self) -> PermissionMode {
        PermissionMode::default()
    }

    pub async fn trust_project(&self) {}

    pub async fn untrust_project(&self) {}

    pub async fn upsert_rule(&self, _tool: String, _action: PermissionAction) {}
}

/// Stub actor state.
pub struct PermissionActorState;

impl PermissionActorState {
    fn emit(&self, _event: Event) {}
}

/// Stub actor — does nothing.
pub struct RactorPermissionActor;

impl RactorPermissionActor {
    pub async fn spawn(
        bus: EventBus<Event>,
        _config_h: RactorConfigHandle,
    ) -> anyhow::Result<(RactorPermissionHandle, ActorCell, ractor::concurrency::JoinHandle<()>)> {
        let (actor_ref, join, cell) = spawn_ractor(None, Self, bus).await?;
        Ok((RactorPermissionHandle::new(cell.clone()), cell, join))
    }

    /// Spawn a stub permission actor (no config needed, all bypass).
    pub async fn spawn_for_testing(
        bus: EventBus<Event>,
    ) -> anyhow::Result<(RactorPermissionHandle, ActorCell, ractor::concurrency::JoinHandle<()>)> {
        let (_actor_ref, join, cell) = spawn_ractor(None, Self, bus).await?;
        Ok((RactorPermissionHandle::new(cell.clone()), cell, join))
    }
}

#[ractor::async_trait]
impl Actor for RactorPermissionActor {
    type Msg = PermissionMsg;
    type State = PermissionActorState;
    type Arguments = EventBus<Event>;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _bus: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(PermissionActorState)
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        _msg: Self::Msg,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        Ok(())
    }
}
