//! Ractor-based TurnActor implementation.
//!
//! Migrated from custom Actor trait to ractor for consistency with the rest
//! of the actor system.

use parking_lot::Mutex;

use ractor::{Actor, ActorProcessingErr, ActorRef};

use crate::actors::ractor_adapter::{spawn_ractor, EventBusBridge};
use crate::bus::EventBus;
use crate::model::{DeliveryMode, QueuedMessage, QueuedMessageKind};
use crate::session::turn_queue::TurnQueue;
use crate::Event;

use super::messages::{NextIdResponse, TurnMsg};
use super::state::TurnState;

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

/// TurnActor state for ractor.
pub struct RactorTurnActor {
    /// The authoritative turn state.
    state: Mutex<TurnState>,
    /// Bridge to the event bus for publishing facts.
    bus_bridge: EventBusBridge<Event>,
}

impl RactorTurnActor {
    fn new(bus: EventBus<Event>) -> Self {
        Self {
            state: Mutex::new(TurnState::default()),
            bus_bridge: EventBusBridge::new(bus),
        }
    }

    fn emit(&self, event: Event) {
        self.bus_bridge.publish(event);
    }

    fn handle_msg(&self, msg: TurnMsg) {
        match msg {
            TurnMsg::RunIfQueued => self.handle_run_if_queued(),
            TurnMsg::AbortTurn => self.handle_abort_turn(),
            TurnMsg::SubmitUserMessage { content, id } => {
                self.handle_submit_user_message(content, id);
            }
            TurnMsg::QueueSteering { content } => self.handle_queue_steering(content),
            TurnMsg::QueueFollowUp { content } => self.handle_queue_follow_up(content),
            TurnMsg::AbortQueue => self.handle_abort_queue(),
            TurnMsg::ClearQueues => self.handle_clear_queues(),
            TurnMsg::DeliverQueued { steering_mode, follow_up_mode } => {
                self.handle_deliver_queued(steering_mode, follow_up_mode);
            }
            TurnMsg::Dequeue => self.handle_dequeue(),
            TurnMsg::Thinking { id } => self.handle_thinking(id),
            TurnMsg::ThoughtDone { .. } => self.handle_thought_done(),
            TurnMsg::ToolStart { id, name } => self.handle_tool_start(id, name),
            TurnMsg::ToolEnd { id, duration_secs, output } => {
                self.handle_tool_end(id, duration_secs, output);
            }
            TurnMsg::ResponseDelta { id, content } => self.handle_response_delta(id, content),
            TurnMsg::TurnComplete { id, duration_secs } => {
                self.handle_turn_complete(id, duration_secs);
            }
            TurnMsg::Done { .. } => self.handle_done(),
            TurnMsg::Error { id, message } => self.handle_error(id, message),
            TurnMsg::UpdateSpeed { tokens_out } => self.handle_update_speed(tokens_out),
            TurnMsg::NextId => self.handle_next_id(),
        }
    }

    fn handle_run_if_queued(&self) {
        let mut state = self.state.lock();
        if state.turn_active {
            return;
        }
        let Some((content, id)) = state.pop_queue() else {
            return;
        };
        state.start_turn();
        drop(state);
        self.emit(Event::TurnStarted {
            id: id.clone(),
            request_id: id,
            content,
        });
    }

    fn handle_abort_turn(&self) {
        {
            let mut state = self.state.lock();
            let messages: Vec<_> = state.message_queue.drain(..).rev().collect();
            for msg in &messages {
                self.emit(Event::QueueAborted { content: msg.content.clone() });
            }
            state.stop_turn();
        }
        self.emit(Event::TurnAborted);
    }

    fn handle_submit_user_message(&self, content: String, id: String) {
        self.emit(Event::UserMessageSubmitted {
            id: id.clone(),
            content: content.clone(),
        });
        let mut state = self.state.lock();
        state.request_queue.push_back((content, id));
    }

    fn handle_queue_steering(&self, content: String) {
        let mut state = self.state.lock();
        state.message_queue.push(QueuedMessage {
            content,
            kind: QueuedMessageKind::Steering,
        });
    }

    fn handle_queue_follow_up(&self, content: String) {
        let mut state = self.state.lock();
        state.message_queue.push(QueuedMessage {
            content,
            kind: QueuedMessageKind::FollowUp,
        });
    }

    fn handle_abort_queue(&self) {
        {
            let mut state = self.state.lock();
            let messages: Vec<_> = state.message_queue.drain(..).rev().collect();
            for msg in &messages {
                self.emit(Event::QueueAborted { content: msg.content.clone() });
            }
        }
    }

    fn handle_clear_queues(&self) {
        let mut state = self.state.lock();
        state.request_queue.clear();
        state.message_queue.clear();
        drop(state);
        self.emit(Event::QueuesCleared);
    }

    fn handle_deliver_queued(&self, steering_mode: DeliveryMode, follow_up_mode: DeliveryMode) {
        if self.try_deliver_steering(steering_mode) {
            // Steering delivered - in "All" mode, also try follow-ups via pop_follow_up
            if follow_up_mode == DeliveryMode::All {
                self.try_deliver_follow_up(follow_up_mode);
            }
            return;
        }
        self.try_deliver_follow_up(follow_up_mode);
    }

    fn try_deliver_steering(&self, mode: DeliveryMode) -> bool {
        let id = {
            let mut state = self.state.lock();
            let mut queue = TurnQueue::new(std::mem::take(&mut state.message_queue));
            let result = queue.pop_steering(mode);
            state.message_queue = queue.into_inner();
            match result {
                None => return false,
                Some(r) => {
                    let id = self.next_id_internal(&mut state);
                    state.request_queue.push_back((r.content.clone(), id.clone()));
                    (r.content, id)
                }
            }
        };
        self.emit(Event::SteeringDelivered { content: id.0, id: id.1 });
        true
    }

    fn try_deliver_follow_up(&self, mode: DeliveryMode) {
        let id = {
            let mut state = self.state.lock();
            let mut queue = TurnQueue::new(std::mem::take(&mut state.message_queue));
            let result = queue.pop_follow_up(mode);
            state.message_queue = queue.into_inner();
            match result {
                None => return,
                Some(r) => {
                    let id = self.next_id_internal(&mut state);
                    state.request_queue.push_back((r.content.clone(), id.clone()));
                    (r.content, id)
                }
            }
        };
        self.emit(Event::FollowUpDelivered { content: id.0, id: id.1 });
    }

    fn next_id_internal(&self, state: &mut TurnState) -> String {
        let id = format!("req.{}", state.next_id);
        state.next_id += 1;
        id
    }

    fn handle_dequeue(&self) {
        let content = {
            let mut state = self.state.lock();
            state.message_queue.pop().map(|m| m.content)
        };
        if let Some(content) = content {
            self.emit(Event::MessageDequeued { content });
        }
    }

    fn handle_thinking(&self, id: String) {
        {
            let mut state = self.state.lock();
            state.thinking_started_at = Some(std::time::Instant::now());
        }
        self.emit(Event::Thinking { id });
    }

    fn handle_thought_done(&self) {
        let mut state = self.state.lock();
        state.thinking_started_at = None;
    }

    fn handle_tool_start(&self, id: String, name: String) {
        {
            let mut state = self.state.lock();
            state.tool_started_at = Some(std::time::Instant::now());
            state.current_tool_name = Some(name.clone());
            state.intermediate_step_count += 1;
        }
        self.emit(Event::ToolStart {
            id,
            name,
            input: serde_json::Value::Null,
        });
    }

    fn handle_tool_end(&self, id: String, duration_secs: f64, output: String) {
        {
            let mut state = self.state.lock();
            state.tool_started_at = None;
            state.current_tool_name = None;
        }
        self.emit(Event::ToolEnd { id, duration_secs, output });
    }

    fn handle_response_delta(&self, id: String, content: String) {
        let res = {
            let mut state = self.state.lock();
            if !state.streaming {
                state.streaming = true;
                true
            } else {
                false
            }
        }; if res {
            self.emit(Event::StreamStarted { id: id.clone() });
        }
        self.emit(Event::ResponseDelta { id, content });
    }

    fn handle_turn_complete(&self, id: String, duration_secs: f64) {
        {
            let mut state = self.state.lock();
            state.turn_started_at = None;
            state.turn_tokens_out = 0;
        }
        self.emit(Event::TurnComplete { id, duration_secs });
    }

    fn handle_done(&self) {
        {
            let mut state = self.state.lock();
            state.streaming = false;
            state.turn_active = false;
            state.inflight = state.inflight.saturating_sub(1);
            state.current_tool_name = None;
        }
        self.emit(Event::TurnCompleted);
    }

    fn handle_error(&self, id: String, message: String) {
        {
            let mut state = self.state.lock();
            state.streaming = false;
            state.turn_active = false;
            state.inflight = 0;
        }
        self.emit(Event::TurnErrored { id, message });
    }

    fn handle_update_speed(&self, tokens_out: usize) {
        let (tokens_in, speed_tps) = {
            let mut state = self.state.lock();
            state.tokens_out = tokens_out;
            state.turn_tokens_out = tokens_out;
            state.speed_window.record(tokens_out);
            state.speed_tps = state.speed_window.speed();
            state.last_speed_update = Some(std::time::Instant::now());
            state.tokens_at_last_speed = tokens_out;
            (state.tokens_in, state.speed_tps)
        };
        self.emit(Event::TokenStatsUpdated { tokens_in, tokens_out, speed_tps });
    }

    fn handle_next_id(&self) {
        let id = {
            let mut state = self.state.lock();
            self.next_id_internal(&mut state)
        };
        self.emit(Event::IdGenerated(NextIdResponse { id }));
    }
}

#[ractor::async_trait]
impl Actor for RactorTurnActor {
    type Msg = TurnMsg;
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
        msg: Self::Msg,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        self.handle_msg(msg);
        Ok(())
    }
}

impl RactorTurnActor {
    /// Spawn a `RactorTurnActor` on the given event bus.
    pub async fn spawn(bus: EventBus<Event>) -> (RactorTurnHandle, ractor::ActorCell, tokio::task::JoinHandle<()>) {
        let actor = Self::new(bus.clone());
        let (handle, join, cell) = spawn_ractor(None, actor, bus).await.unwrap();
        (RactorTurnHandle::new(handle), cell, join)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_if_queued_starts_turn() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _, _) = RactorTurnActor::spawn(bus.clone()).await;
        let mut sub = bus.subscribe();
        handle.send(TurnMsg::SubmitUserMessage { content: "hello".into(), id: "req.0".into() }).await;
        handle.send(TurnMsg::RunIfQueued).await;
        let mut found = false;
        while let Ok(evt) = sub.recv().await {
            if matches!(evt, Event::TurnStarted { .. }) { found = true; break; }
        }
        assert!(found);
    }

    #[tokio::test]
    async fn abort_turn_clears_state() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _, _) = RactorTurnActor::spawn(bus.clone()).await;
        let mut sub = bus.subscribe();
        handle.send(TurnMsg::SubmitUserMessage { content: "hello".into(), id: "req.0".into() }).await;
        handle.send(TurnMsg::RunIfQueued).await;
        handle.send(TurnMsg::AbortTurn).await;
        let mut found = false;
        while let Ok(evt) = sub.recv().await {
            if matches!(evt, Event::TurnAborted) { found = true; break; }
        }
        assert!(found);
    }

    #[tokio::test]
    async fn error_emits_turned_errored() {
        let bus = EventBus::<Event>::new(16);
        let (handle, _, _) = RactorTurnActor::spawn(bus.clone()).await;
        let mut sub = bus.subscribe();
        handle.send(TurnMsg::Error { id: "req.0".into(), message: "oops".into() }).await;
        let mut found = false;
        while let Ok(evt) = sub.recv().await {
            if matches!(evt, Event::TurnErrored { .. }) { found = true; break; }
        }
        assert!(found);
    }
}
