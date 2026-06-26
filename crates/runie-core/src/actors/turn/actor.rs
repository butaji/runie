//! TurnActor — owns the agent turn lifecycle and queues.

use tokio::sync::mpsc;

use crate::actors::turn::messages::{NextIdResponse, TurnMsg};
use crate::actors::{spawn_actor, Actor, ActorHandle};
use crate::bus::EventBus;
use crate::Event;

use super::state::TurnState;

/// TurnActor owns turn lifecycle state.
pub struct TurnActor {
    bus: EventBus<Event>,
    state: TurnState,
}

impl TurnActor {
    pub fn spawn(bus: EventBus<Event>) -> (TurnActorHandle, ActorHandle) {
        let actor = Self { bus: bus.clone(), state: TurnState::default() };
        let (tx, handle) = spawn_actor(actor, bus);
        (TurnActorHandle::new(tx), handle)
    }

    fn emit(&self, event: Event) {
        let _ = self.bus.publish(event);
    }
}

/// Ergonomic handle for sending commands to a `TurnActor`.
#[derive(Clone, Debug)]
pub struct TurnActorHandle {
    tx: std::sync::Arc<mpsc::Sender<TurnMsg>>,
}

impl TurnActorHandle {
    pub fn new(tx: mpsc::Sender<TurnMsg>) -> Self {
        Self { tx: std::sync::Arc::new(tx) }
    }

    pub async fn send(&self, msg: TurnMsg) {
        let _ = self.tx.send(msg).await;
    }

    pub fn try_send(&self, msg: TurnMsg) {
        let _ = self.tx.try_send(msg);
    }
}

impl Actor for TurnActor {
    type Msg = TurnMsg;
    type Event = Event;

    async fn run_body(mut self, mut rx: mpsc::Receiver<Self::Msg>, _bus: EventBus<Event>) {
        while let Some(msg) = rx.recv().await {
            self.handle_msg(msg).await;
        }
    }
}

impl TurnActor {
    async fn handle_msg(&mut self, msg: TurnMsg) {
        match msg {
            TurnMsg::RunIfQueued => self.handle_run_if_queued().await,
            TurnMsg::AbortTurn => self.handle_abort_turn(),
            TurnMsg::SubmitUserMessage { content, id } => {
                self.handle_submit_user_message(content, id);
            }
            TurnMsg::QueueSteering { content } => self.handle_queue_steering(content),
            TurnMsg::QueueFollowUp { content } => self.handle_queue_follow_up(content),
            TurnMsg::AbortQueue => self.handle_abort_queue(),
            TurnMsg::ClearQueues => self.handle_clear_queues(),
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

    async fn handle_run_if_queued(&mut self) {
        if self.state.turn_active {
            return;
        }
        let Some((content, id)) = self.state.pop_queue() else {
            return;
        };
        self.state.start_turn();
        self.emit(Event::TurnStarted {
            id: id.clone(),
            request_id: id,
            content,
        });
    }

    fn handle_abort_turn(&mut self) {
        let messages: Vec<_> = self.state.message_queue.drain(..).rev().collect();
        for msg in messages {
            self.emit(Event::QueueAborted { content: msg.content });
        }
        self.state.stop_turn();
        self.emit(Event::TurnAborted);
    }

    fn handle_submit_user_message(&mut self, content: String, id: String) {
        self.emit(Event::UserMessageSubmitted {
            id: id.clone(),
            content: content.clone(),
        });
        self.state.request_queue.push_back((content, id));
    }

    fn handle_queue_steering(&mut self, content: String) {
        self.state.message_queue.push(crate::model::QueuedMessage {
            content,
            kind: crate::model::QueuedMessageKind::Steering,
        });
    }

    fn handle_queue_follow_up(&mut self, content: String) {
        self.state.message_queue.push(crate::model::QueuedMessage {
            content,
            kind: crate::model::QueuedMessageKind::FollowUp,
        });
    }

    fn handle_abort_queue(&mut self) {
        let messages: Vec<_> = self.state.message_queue.drain(..).rev().collect();
        for msg in messages {
            self.emit(Event::QueueAborted { content: msg.content });
        }
    }

    fn handle_clear_queues(&mut self) {
        self.state.request_queue.clear();
        self.state.message_queue.clear();
        self.emit(Event::QueuesCleared);
    }

    fn handle_thinking(&mut self, id: String) {
        self.state.thinking_started_at = Some(std::time::Instant::now());
        self.emit(Event::Thinking { id });
    }

    fn handle_thought_done(&mut self) {
        self.state.thinking_started_at = None;
    }

    fn handle_tool_start(&mut self, id: String, name: String) {
        self.state.tool_started_at = Some(std::time::Instant::now());
        self.state.current_tool_name = Some(name.clone());
        self.state.intermediate_step_count += 1;
        self.emit(Event::ToolStart {
            id,
            name,
            input: serde_json::Value::Null,
        });
    }

    fn handle_tool_end(&mut self, id: String, duration_secs: f64, output: String) {
        self.state.tool_started_at = None;
        self.state.current_tool_name = None;
        self.emit(Event::ToolEnd { id, duration_secs, output });
    }

    fn handle_response_delta(&mut self, id: String, content: String) {
        if !self.state.streaming {
            self.state.streaming = true;
            self.emit(Event::StreamStarted { id: id.clone() });
        }
        self.emit(Event::ResponseDelta { id, content });
    }

    fn handle_turn_complete(&mut self, id: String, duration_secs: f64) {
        self.state.turn_started_at = None;
        self.state.turn_tokens_out = 0;
        self.emit(Event::TurnComplete { id, duration_secs });
    }

    fn handle_done(&mut self) {
        self.state.streaming = false;
        self.state.turn_active = false;
        self.state.inflight = self.state.inflight.saturating_sub(1);
        self.state.current_tool_name = None;
        self.emit(Event::TurnCompleted);
    }

    fn handle_error(&mut self, id: String, message: String) {
        self.state.streaming = false;
        self.state.turn_active = false;
        self.state.inflight = 0;
        self.emit(Event::TurnErrored { id, message });
    }

    fn handle_update_speed(&mut self, tokens_out: usize) {
        self.state.tokens_out = tokens_out;
        self.state.turn_tokens_out = tokens_out;
        self.state.speed_window.record(tokens_out);
        self.state.speed_tps = self.state.speed_window.speed();
        self.state.last_speed_update = Some(std::time::Instant::now());
        self.state.tokens_at_last_speed = tokens_out;
        self.emit(Event::TokenStatsUpdated {
            tokens_in: self.state.tokens_in,
            tokens_out: self.state.tokens_out,
            speed_tps: self.state.speed_tps,
        });
    }

    fn handle_next_id(&mut self) {
        let id = format!("req.{}", self.state.next_id);
        self.state.next_id += 1;
        self.emit(Event::IdGenerated(NextIdResponse { id }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_if_queued_starts_turn_when_queue_has_message() {
        let bus = crate::bus::EventBus::new(16);
        let (handle, _actor) = TurnActor::spawn(bus.clone());
        let mut sub = bus.subscribe();

        handle.send(TurnMsg::SubmitUserMessage {
            content: "hello".into(),
            id: "req.0".into(),
        }).await;

        handle.send(TurnMsg::RunIfQueued).await;

        // Use recv().await to block until event arrives
        let mut found = false;
        while let Ok(evt) = sub.recv().await {
            if matches!(evt, Event::TurnStarted { .. }) {
                found = true;
                break;
            }
        }
        assert!(found, "TurnStarted should be emitted");
    }

    #[tokio::test]
    async fn abort_turn_clears_state() {
        let bus = crate::bus::EventBus::new(16);
        let (handle, _actor) = TurnActor::spawn(bus.clone());
        let mut sub = bus.subscribe();

        handle.send(TurnMsg::SubmitUserMessage {
            content: "hello".into(),
            id: "req.0".into(),
        }).await;
        handle.send(TurnMsg::RunIfQueued).await;
        handle.send(TurnMsg::AbortTurn).await;

        // Use recv().await to block until event arrives
        let mut found = false;
        while let Ok(evt) = sub.recv().await {
            if matches!(evt, Event::TurnAborted) {
                found = true;
                break;
            }
        }
        assert!(found, "TurnAborted should be emitted");
    }

    #[tokio::test]
    async fn error_emits_turned_errored() {
        let bus = crate::bus::EventBus::new(16);
        let (handle, _actor) = TurnActor::spawn(bus.clone());
        let mut sub = bus.subscribe();

        handle.send(TurnMsg::Error {
            id: "req.0".into(),
            message: "oops".into(),
        }).await;

        // Use recv().await to block until event arrives
        let mut found = false;
        while let Ok(evt) = sub.recv().await {
            if matches!(evt, Event::TurnErrored { .. }) {
                found = true;
                break;
            }
        }
        assert!(found, "TurnErrored should be emitted");
    }
}
