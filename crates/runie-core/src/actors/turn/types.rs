//! TurnActor types: state struct and handle.

use ractor::ActorRef;

use crate::bus::EventBus;
use crate::Event;

use super::messages::{DeliverQueuedRpcResult, TurnMsg};
use super::state::TurnState;

/// Ractor State for TurnActor — holds all mutable state.
/// EventBus is Clone and publish takes &self, no Mutex needed.
#[derive(Clone)]
pub struct TurnActorState {
    pub turn_state: TurnState,
    pub bus: EventBus<Event>,
}

impl TurnActorState {
    pub fn new(bus: EventBus<Event>) -> Self {
        Self { turn_state: TurnState::default(), bus }
    }
}

/// Ractor-based TurnActor handle.
#[derive(Clone, Debug)]
pub struct RactorTurnHandle {
    /// Public for ergonomic access by agent extensions.
    pub inner: ActorRef<TurnMsg>,
}

impl RactorTurnHandle {
    /// Create a new handle wrapping an ActorRef.
    pub fn new(inner: ActorRef<TurnMsg>) -> Self {
        Self { inner }
    }

    /// Send a message to the actor (fire-and-forget).
    pub async fn send(&self, msg: TurnMsg) {
        let _ = self.inner.send_message(msg);
    }

    /// Try to send a message (non-blocking).
    pub fn try_send(&self, msg: TurnMsg) -> Result<(), ractor::MessagingErr<TurnMsg>> {
        self.inner.send_message(msg)
    }

    /// Deliver queued messages and wait for the actor to emit SteeringDelivered/
    /// FollowUpDelivered before returning. Uses ractor RPC so delivery is atomic.
    pub async fn deliver_queued(
        &self,
        steering_mode: crate::model::DeliveryMode,
        follow_up_mode: crate::model::DeliveryMode,
    ) -> DeliverQueuedRpcResult {
        use ractor::rpc::CallResult;
        match self
            .inner
            .call(
                |tx| TurnMsg::DeliverQueued { steering_mode, follow_up_mode, reply: Some(tx) },
                None,
            )
            .await
        {
            Ok(CallResult::Success(r)) => DeliverQueuedRpcResult::Delivered(r),
            Ok(CallResult::SenderError) => DeliverQueuedRpcResult::SenderError,
            Ok(CallResult::Timeout) => DeliverQueuedRpcResult::ActorError("RPC timeout".to_string()),
            Err(e) => DeliverQueuedRpcResult::ActorError(e.to_string()),
        }
    }
}
