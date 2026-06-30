//! Ractor-based TurnActor implementation.
//!
//! Migrated from custom Actor trait to ractor for consistency with the rest
//! of the actor system.

use ractor::{Actor, ActorProcessingErr, ActorRef};

use crate::actors::ractor_adapter::spawn_ractor;
use crate::bus::EventBus;
use crate::model::{DeliveryMode, QueuedMessage, QueuedMessageKind};
use crate::session::turn_queue::TurnQueue;
use crate::Event;

use super::messages::{NextIdResponse, TurnMsg};
use super::state::TurnState;

/// Ractor State for TurnActor — holds all mutable state.
/// EventBus is Clone and publish takes &self, no Mutex needed.
pub struct TurnActorState {
    pub turn_state: TurnState,
    pub bus: EventBus<Event>,
}

impl TurnActorState {
    fn new(bus: EventBus<Event>) -> Self {
        Self {
            turn_state: TurnState::default(),
            bus,
        }
    }
}

/// Ractor-based TurnActor handle.
#[derive(Clone, Debug)]
pub struct RactorTurnHandle {
    inner: crate::actors::ractor_adapter::RactorHandle<TurnMsg>,
}

impl RactorTurnHandle {
    /// Create a new handle wrapping the inner RactorHandle.
    pub fn new(inner: crate::actors::ractor_adapter::RactorHandle<TurnMsg>) -> Self {
        Self { inner }
    }

    /// Send a message to the actor (fire-and-forget).
    pub async fn send(&self, msg: TurnMsg) {
        let _ = self.inner.send(msg).await;
    }

    /// Try to send a message (non-blocking).
    pub fn try_send(&self, msg: TurnMsg) -> Result<(), ractor::MessagingErr<TurnMsg>> {
        self.inner.try_send(msg)
    }

}

/// TurnActor using ractor State pattern.
pub struct RactorTurnActor;

impl RactorTurnActor {
    fn emit(state: &TurnActorState, event: Event) {
        state.bus.publish(event);
    }

    fn handle_run_if_queued(state: &mut TurnActorState) {
        if state.turn_state.turn_active {
            return;
        }
        let Some((content, id)) = state.turn_state.pop_queue() else {
            return;
        };
        state.turn_state.start_turn();
        Self::emit(
            state,
            Event::TurnStarted {
                id: id.clone(),
                request_id: id,
                content,
            },
        );
    }

    fn handle_abort_turn(state: &mut TurnActorState) {
        let messages: Vec<_> = state.turn_state.message_queue.drain(..).rev().collect();
        for msg in &messages {
            Self::emit(
                state,
                Event::QueueAborted {
                    content: msg.content.clone(),
                },
            );
        }
        state.turn_state.stop_turn();
        Self::emit(state, Event::TurnAborted);
    }

    fn handle_submit_user_message(
        state: &mut TurnActorState,
        content: String,
        id: String,
        source: super::messages::MessageSource,
    ) {
        // Only emit UserMessageSubmitted for fresh submits.
        // For queued/delivered messages, the content is already in the session
        // via FollowUpDelivered -> deliver_queued -> UserMessageSubmitted.
        if source == super::messages::MessageSource::Fresh {
            Self::emit(
                state,
                Event::UserMessageSubmitted {
                    id: id.clone(),
                    content: content.clone(),
                },
            );
        }
        state.turn_state.request_queue.push_back((content, id));
        state.turn_state.next_id += 1;
        // Only auto-start for fresh submits; queued messages are started via
        // RunIfQueued after TurnCompleted/DeliverQueued.
        if source == super::messages::MessageSource::Fresh {
            Self::handle_run_if_queued(state);
        }
    }

    fn handle_queue_steering(state: &mut TurnActorState, content: String) {
        state.turn_state.message_queue.push(QueuedMessage {
            content,
            kind: QueuedMessageKind::Steering,
        });
    }

    fn handle_queue_follow_up(state: &mut TurnActorState, content: String) {
        state.turn_state.message_queue.push(QueuedMessage {
            content,
            kind: QueuedMessageKind::FollowUp,
        });
    }

    fn handle_abort_queue(state: &mut TurnActorState) {
        let messages: Vec<_> = state.turn_state.message_queue.drain(..).rev().collect();
        for msg in &messages {
            Self::emit(
                state,
                Event::QueueAborted {
                    content: msg.content.clone(),
                },
            );
        }
    }

    fn handle_clear_queues(state: &mut TurnActorState) {
        state.turn_state.request_queue.clear();
        state.turn_state.message_queue.clear();
        Self::emit(state, Event::QueuesCleared);
    }

    fn handle_deliver_queued(
        state: &mut TurnActorState,
        steering_mode: DeliveryMode,
        follow_up_mode: DeliveryMode,
    ) {
        if Self::try_deliver_steering(state, steering_mode) {
            if follow_up_mode == DeliveryMode::All {
                Self::try_deliver_follow_up(state, follow_up_mode);
            }
            return;
        }
        Self::try_deliver_follow_up(state, follow_up_mode);
    }

    fn try_deliver_steering(state: &mut TurnActorState, mode: DeliveryMode) -> bool {
        let mut queue = TurnQueue::new(std::mem::take(&mut state.turn_state.message_queue));
        let result = queue.pop_steering(mode);
        state.turn_state.message_queue = queue.into_inner();
        let (content, id) = match result {
            None => return false,
            Some(r) => {
                let id = Self::next_id_internal(&mut state.turn_state);
                state
                    .turn_state
                    .request_queue
                    .push_back((r.content.clone(), id.clone()));
                (r.content, id)
            }
        };
        Self::emit(state, Event::SteeringDelivered { content, id });
        true
    }

    fn try_deliver_follow_up(state: &mut TurnActorState, mode: DeliveryMode) {
        let mut queue = TurnQueue::new(std::mem::take(&mut state.turn_state.message_queue));
        let result = queue.pop_follow_up(mode);
        state.turn_state.message_queue = queue.into_inner();
        let (content, id) = match result {
            None => return,
            Some(r) => {
                let id = Self::next_id_internal(&mut state.turn_state);
                state
                    .turn_state
                    .request_queue
                    .push_back((r.content.clone(), id.clone()));
                (r.content, id)
            }
        };
        Self::emit(state, Event::FollowUpDelivered { content, id });
    }

    fn next_id_internal(turn_state: &mut TurnState) -> String {
        let id = format!("req.{}", turn_state.next_id);
        turn_state.next_id += 1;
        id
    }

    fn handle_dequeue(state: &mut TurnActorState) {
        let content = state.turn_state.message_queue.pop().map(|m| m.content);
        if let Some(content) = content {
            Self::emit(state, Event::MessageDequeued { content });
        }
    }

    fn handle_thinking(state: &mut TurnActorState, id: String) {
        state.turn_state.thinking_started_at = Some(std::time::Instant::now());
        Self::emit(state, Event::Thinking { id });
    }

    fn handle_thought_done(state: &mut TurnActorState) {
        state.turn_state.thinking_started_at = None;
    }

    fn handle_tool_start(state: &mut TurnActorState, id: String, name: String) {
        state.turn_state.tool_started_at = Some(std::time::Instant::now());
        state.turn_state.current_tool_name = Some(name.clone());
        state.turn_state.intermediate_step_count += 1;
        Self::emit(
            state,
            Event::ToolStart {
                id,
                name,
                input: serde_json::Value::Null,
            },
        );
    }

    fn handle_tool_end(state: &mut TurnActorState, id: String, duration_secs: f64, output: String) {
        state.turn_state.tool_started_at = None;
        state.turn_state.current_tool_name = None;
        Self::emit(
            state,
            Event::ToolEnd {
                id,
                duration_secs,
                output,
            },
        );
    }

    fn handle_response_delta(state: &mut TurnActorState, id: String, content: String) {
        let was_streaming = state.turn_state.streaming;
        if !was_streaming {
            state.turn_state.streaming = true;
            Self::emit(state, Event::StreamStarted { id: id.clone() });
        }
        Self::emit(state, Event::ResponseDelta { id, content });
    }

    fn handle_turn_complete(state: &mut TurnActorState, id: String, duration_secs: f64) {
        state.turn_state.turn_started_at = None;
        state.turn_state.turn_tokens_out = 0;
        Self::emit(state, Event::TurnComplete { id, duration_secs });
    }

    fn handle_done(state: &mut TurnActorState) {
        state.turn_state.streaming = false;
        state.turn_state.turn_active = false;
        state.turn_state.inflight = state.turn_state.inflight.saturating_sub(1);
        state.turn_state.current_tool_name = None;
        Self::emit(state, Event::TurnCompleted);
    }

    fn handle_error(state: &mut TurnActorState, id: String, message: String) {
        state.turn_state.streaming = false;
        state.turn_state.turn_active = false;
        state.turn_state.inflight = 0;
        Self::emit(state, Event::TurnErrored { id, message });
    }

    fn handle_update_speed(state: &mut TurnActorState, tokens_out: usize) {
        state.turn_state.tokens_out = tokens_out;
        state.turn_state.turn_tokens_out = tokens_out;
        state.turn_state.speed_window.record(tokens_out);
        state.turn_state.speed_tps = state.turn_state.speed_window.speed();
        state.turn_state.last_speed_update = Some(std::time::Instant::now());
        state.turn_state.tokens_at_last_speed = tokens_out;
        let tokens_in = state.turn_state.tokens_in;
        let speed_tps = state.turn_state.speed_tps;
        Self::emit(
            state,
            Event::TokenStatsUpdated {
                tokens_in,
                tokens_out,
                speed_tps,
            },
        );
    }

    fn handle_next_id(state: &mut TurnActorState) {
        let id = Self::next_id_internal(&mut state.turn_state);
        Self::emit(state, Event::IdGenerated(NextIdResponse { id }));
    }
}

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

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            TurnMsg::RunIfQueued => Self::handle_run_if_queued(state),
            TurnMsg::AbortTurn => Self::handle_abort_turn(state),
            TurnMsg::SubmitUserMessage {
                content,
                id,
                source,
            } => Self::handle_submit_user_message(state, content, id, source),
            TurnMsg::QueueSteering { content } => Self::handle_queue_steering(state, content),
            TurnMsg::QueueFollowUp { content } => Self::handle_queue_follow_up(state, content),
            TurnMsg::AbortQueue => Self::handle_abort_queue(state),
            TurnMsg::ClearQueues => Self::handle_clear_queues(state),
            TurnMsg::DeliverQueued {
                steering_mode,
                follow_up_mode,
            } => Self::handle_deliver_queued(state, steering_mode, follow_up_mode),
            TurnMsg::Dequeue => Self::handle_dequeue(state),
            TurnMsg::Thinking { id } => Self::handle_thinking(state, id),
            TurnMsg::ThoughtDone { .. } => Self::handle_thought_done(state),
            TurnMsg::ToolStart { id, name } => Self::handle_tool_start(state, id, name),
            TurnMsg::ToolEnd {
                id,
                duration_secs,
                output,
            } => Self::handle_tool_end(state, id, duration_secs, output),
            TurnMsg::ResponseDelta { id, content } => {
                Self::handle_response_delta(state, id, content)
            }
            TurnMsg::TurnComplete { id, duration_secs } => {
                Self::handle_turn_complete(state, id, duration_secs)
            }
            TurnMsg::Done { .. } => Self::handle_done(state),
            TurnMsg::Error { id, message } => Self::handle_error(state, id, message),
            TurnMsg::UpdateSpeed { tokens_out } => Self::handle_update_speed(state, tokens_out),
            TurnMsg::NextId => Self::handle_next_id(state),
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
        RactorTurnHandle,
        ractor::ActorCell,
        tokio::task::JoinHandle<()>,
    )> {
        let (handle, join, cell) =
            spawn_ractor(None, Self, bus)
                .await
                .map_err(|e| anyhow::anyhow!("RactorTurnActor spawn failed: {}", e))?;
        Ok((RactorTurnHandle::new(handle), cell, join))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actors::turn::messages::MessageSource;

    #[tokio::test]
    async fn run_if_queued_starts_turn() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _, _) = RactorTurnActor::spawn(bus.clone()).await.unwrap();
        let mut sub = bus.subscribe();
        handle
            .send(TurnMsg::SubmitUserMessage {
                content: "hello".into(),
                id: "req.0".into(),
                source: MessageSource::Fresh,
            })
            .await;
        handle.send(TurnMsg::RunIfQueued).await;
        let mut found = false;
        while let Ok(evt) = sub.recv().await {
            if matches!(evt, Event::TurnStarted { .. }) {
                found = true;
                break;
            }
        }
        assert!(found);
    }

    #[tokio::test]
    async fn abort_turn_clears_state() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _, _) = RactorTurnActor::spawn(bus.clone()).await.unwrap();
        let mut sub = bus.subscribe();
        handle
            .send(TurnMsg::SubmitUserMessage {
                content: "hello".into(),
                id: "req.0".into(),
                source: MessageSource::Fresh,
            })
            .await;
        handle.send(TurnMsg::RunIfQueued).await;
        handle.send(TurnMsg::AbortTurn).await;
        let mut found = false;
        while let Ok(evt) = sub.recv().await {
            if matches!(evt, Event::TurnAborted) {
                found = true;
                break;
            }
        }
        assert!(found);
    }

    #[tokio::test]
    async fn error_emits_turned_errored() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _, _) = RactorTurnActor::spawn(bus.clone()).await.unwrap();
        let mut sub = bus.subscribe();
        handle
            .send(TurnMsg::Error {
                id: "req.0".into(),
                message: "oops".into(),
            })
            .await;
        let mut found = false;
        while let Ok(evt) = sub.recv().await {
            if matches!(evt, Event::TurnErrored { .. }) {
                found = true;
                break;
            }
        }
        assert!(found);
    }

    /// Regression test: QueueFollowUp puts in message_queue; DeliverQueued moves it to
    /// request_queue (via FollowUpDelivered); RunIfQueued starts the next turn.
    ///
    /// Flow: SubmitUserMessage → RunIfQueued → TurnStarted
    ///       QueueFollowUp (queues to message_queue)
    ///       Done → TurnCompleted (turn_active becomes false)
    ///       DeliverQueued → FollowUpDelivered (moves to request_queue, generates new id)
    ///       RunIfQueued → TurnStarted (starts second turn with new id)
    #[tokio::test]
    async fn queue_follow_up_after_done_starts_queued_turn() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _, _) = RactorTurnActor::spawn(bus.clone()).await.unwrap();
        let mut sub = bus.subscribe();

        // First turn starts (fresh submit)
        handle
            .send(TurnMsg::SubmitUserMessage {
                content: "first".into(),
                id: "req.0".into(),
                source: MessageSource::Fresh,
            })
            .await;
        handle.send(TurnMsg::RunIfQueued).await;

        // Wait for TurnStarted
        let mut found_first_turn = false;
        while let Ok(evt) = sub.recv().await {
            if matches!(evt, Event::TurnStarted { id, .. } if id == "req.0") {
                found_first_turn = true;
                break;
            }
        }
        assert!(found_first_turn, "First turn should start");

        // Queue a follow-up while first turn is active
        handle
            .send(TurnMsg::QueueFollowUp {
                content: "second".into(),
            })
            .await;

        // First turn completes - Done message results in TurnCompleted event
        handle
            .send(TurnMsg::Done { id: "req.0".into() })
            .await;

        // Wait for TurnCompleted
        let mut found_completed = false;
        while let Ok(evt) = sub.recv().await {
            if matches!(evt, Event::TurnCompleted) {
                found_completed = true;
                break;
            }
        }
        assert!(found_completed, "First turn should complete");

        // After TurnCompleted, DeliverQueued moves message from message_queue to request_queue
        handle
            .send(TurnMsg::DeliverQueued {
                steering_mode: DeliveryMode::OneAtATime,
                follow_up_mode: DeliveryMode::All,
            })
            .await;

        // Wait for FollowUpDelivered — with timeout to detect bugs
        let result = tokio::time::timeout(std::time::Duration::from_secs(2), async {
            while let Ok(evt) = sub.recv().await {
                if matches!(evt, Event::FollowUpDelivered { id, .. } if id == "req.1") {
                    return;
                }
            }
        })
        .await;
        assert!(result.is_ok(), "FollowUpDelivered should fire within 2s");

        // RunIfQueued starts the queued turn
        handle.send(TurnMsg::RunIfQueued).await;

        // After RunIfQueued, second turn should start — with timeout
        let result = tokio::time::timeout(std::time::Duration::from_secs(2), async {
            while let Ok(evt) = sub.recv().await {
                if matches!(evt, Event::TurnStarted { id, .. } if id == "req.1") {
                    return;
                }
            }
        })
        .await;
        assert!(result.is_ok(), "Second turn should start after DeliverQueued + RunIfQueued");
    }
}
