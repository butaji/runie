//! Ractor-based TurnActor implementation.
//!
//! Migrated from custom Actor trait to ractor for consistency with the rest
//! of the actor system.

use ractor::{Actor, ActorProcessingErr, ActorRef};

use crate::actors::ractor_adapter::spawn_ractor;
use crate::bus::EventBus;
use crate::Event;
use tracing::instrument;

use super::handlers;
use super::messages::TurnMsg;
use super::types::TurnActorState;

/// TurnActor using ractor State pattern.
pub struct RactorTurnActor;

#[ractor::async_trait]
impl Actor for RactorTurnActor {
    type Msg = TurnMsg;
    type State = TurnActorState;
    type Arguments = EventBus<Event>;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        bus: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(TurnActorState::new(bus))
    }

    #[instrument(name = "turn_actor", skip_all, fields(msg = ?msg))]
    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            TurnMsg::RunIfQueued => handlers::handle_run_if_queued(state),
            TurnMsg::AbortTurn => handlers::handle_abort_turn(state),
            TurnMsg::SubmitUserMessage {
                content,
                id,
                source,
            } => handlers::handle_submit_user_message(state, content, id, source),
            TurnMsg::QueueSteering { content } => handlers::handle_queue_steering(state, content),
            TurnMsg::QueueFollowUp { content } => handlers::handle_queue_follow_up(state, content),
            TurnMsg::AbortQueue => handlers::handle_abort_queue(state),
            TurnMsg::ClearQueues => handlers::handle_clear_queues(state),
            TurnMsg::DeliverQueued {
                steering_mode,
                follow_up_mode,
                reply,
            } => handlers::handle_deliver_queued(state, steering_mode, follow_up_mode, reply),
            TurnMsg::Dequeue => handlers::handle_dequeue(state),
            TurnMsg::Thinking { id } => handlers::handle_thinking(state, id),
            TurnMsg::ThoughtDone { id: _ } => handlers::handle_thought_done(state),
            TurnMsg::ToolStart { id, name } => handlers::handle_tool_start(state, id, name),
            TurnMsg::ToolEnd {
                id,
                duration_secs,
                output,
            } => handlers::handle_tool_end(state, id, duration_secs, output),
            TurnMsg::ResponseDelta { id, content } => {
                handlers::handle_response_delta(state, id, content)
            }
            TurnMsg::TurnComplete { id, duration_secs } => {
                handlers::handle_turn_complete(state, id, duration_secs)
            }
            TurnMsg::Done { id: _ } => handlers::handle_done(state),
            TurnMsg::Error { id, message } => handlers::handle_error(state, id, message),
            TurnMsg::UpdateSpeed { tokens_out } => handlers::handle_update_speed(state, tokens_out),
            TurnMsg::NextId => handlers::handle_next_id(state),
        }
        Ok(())
    }
}

impl RactorTurnActor {
    /// Spawn a `RactorTurnActor` on the given event bus.
    ///
    /// Returns a `Result` to allow callers to handle spawn failures gracefully.
    pub async fn spawn(
        bus: EventBus<Event>,
    ) -> anyhow::Result<(
        super::RactorTurnHandle,
        ractor::ActorCell,
        tokio::task::JoinHandle<()>,
    )> {
        let (handle, join, cell) = spawn_ractor(None, Self, bus)
            .await
            .map_err(|e| anyhow::anyhow!("RactorTurnActor spawn failed: {}", e))?;
        Ok((super::RactorTurnHandle::new(handle), cell, join))
    }
}
