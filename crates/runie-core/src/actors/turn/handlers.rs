//! TurnActor message handlers.
//!
//! Each handler function takes a `TurnActorState` reference and implements the
//! side-effect-free state transitions for one `TurnMsg` variant.

use crate::actors::turn::types::TurnActorState;
use crate::model::{DeliveryMode, QueuedMessage, QueuedMessageKind};
use crate::session::turn_queue::TurnQueue;
use crate::Event;

use super::messages::{DeliverQueuedResponse, MessageSource};


/// Emit an event through the actor's event bus.
pub(super) fn emit(state: &TurnActorState, event: Event) {
    state.bus.publish(event);
}

/// Handle `TurnMsg::RunIfQueued` — start next turn if idle and queue is non-empty.
pub(super) fn handle_run_if_queued(state: &mut TurnActorState) {
    if state.turn_state.turn_active {
        return;
    }
    let Some((content, id)) = state.turn_state.pop_queue() else {
        return;
    };
    state.turn_state.start_turn();
    emit(
        state,
        Event::TurnStarted {
            id: id.clone(),
            request_id: id,
            content,
        },
    );
}

/// Handle `TurnMsg::AbortTurn` — abort active turn and clear queued messages.
pub(super) fn handle_abort_turn(state: &mut TurnActorState) {
    let messages: Vec<_> = state.turn_state.message_queue.drain(..).rev().collect();
    for msg in &messages {
        emit(
            state,
            Event::QueueAborted {
                content: msg.content.clone(),
            },
        );
    }
    state.turn_state.stop_turn();
    emit(state, Event::TurnAborted);
}

/// Handle `TurnMsg::SubmitUserMessage` — queue a new message and optionally start turn.
pub(super) fn handle_submit_user_message(
    state: &mut TurnActorState,
    content: String,
    id: String,
    source: MessageSource,
) {
    // Only emit UserMessageSubmitted for fresh submits.
    if source == MessageSource::Fresh {
        emit(
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
    if source == MessageSource::Fresh {
        handle_run_if_queued(state);
    }
}

/// Handle `TurnMsg::QueueSteering` — add steering content to the queue.
pub(super) fn handle_queue_steering(state: &mut TurnActorState, content: String) {
    state.turn_state.message_queue.push(QueuedMessage {
        content,
        kind: QueuedMessageKind::Steering,
    });
}

/// Handle `TurnMsg::QueueFollowUp` — add follow-up content to the queue.
pub(super) fn handle_queue_follow_up(state: &mut TurnActorState, content: String) {
    state.turn_state.message_queue.push(QueuedMessage {
        content,
        kind: QueuedMessageKind::FollowUp,
    });
}

/// Handle `TurnMsg::AbortQueue` — drain and abort all queued messages.
pub(super) fn handle_abort_queue(state: &mut TurnActorState) {
    let messages: Vec<_> = state.turn_state.message_queue.drain(..).rev().collect();
    for msg in &messages {
        emit(
            state,
            Event::QueueAborted {
                content: msg.content.clone(),
            },
        );
    }
}

/// Handle `TurnMsg::ClearQueues` — clear both request and message queues.
pub(super) fn handle_clear_queues(state: &mut TurnActorState) {
    state.turn_state.request_queue.clear();
    state.turn_state.message_queue.clear();
    emit(state, Event::QueuesCleared);
}

/// Handle `TurnMsg::DeliverQueued` — pop and deliver queued steering/follow-up messages.
pub(super) fn handle_deliver_queued(
    state: &mut TurnActorState,
    steering_mode: DeliveryMode,
    follow_up_mode: DeliveryMode,
    reply: ractor::RpcReplyPort<Option<DeliverQueuedResponse>>,
) {
    let steering_result = try_deliver_steering(state, steering_mode);
    if steering_result.is_some() {
        if follow_up_mode == DeliveryMode::All {
            try_deliver_follow_up(state, follow_up_mode);
        }
        let _ = reply.send(steering_result.map(|(content, id)| DeliverQueuedResponse::Steering { content, id }));
        return;
    }
    let follow_up_result = try_deliver_follow_up(state, follow_up_mode);
    let _ = reply.send(follow_up_result.map(|(content, id)| DeliverQueuedResponse::FollowUp { content, id }));
}

/// Try to deliver the next steering message from the queue.
fn try_deliver_steering(state: &mut TurnActorState, mode: DeliveryMode) -> Option<(String, String)> {
    let mut queue = TurnQueue::new(std::mem::take(&mut state.turn_state.message_queue));
    let result = queue.pop_steering(mode);
    state.turn_state.message_queue = queue.into_inner();
    let r = result?;
    let id = next_id_internal(&mut state.turn_state);
    state.turn_state.request_queue.push_back((r.content.clone(), id.clone()));
    let content = r.content;
    emit(state, Event::SteeringDelivered { content: content.clone(), id: id.clone() });
    Some((content, id))
}

/// Try to deliver the next follow-up message from the queue.
fn try_deliver_follow_up(state: &mut TurnActorState, mode: DeliveryMode) -> Option<(String, String)> {
    let mut queue = TurnQueue::new(std::mem::take(&mut state.turn_state.message_queue));
    let result = queue.pop_follow_up(mode);
    state.turn_state.message_queue = queue.into_inner();
    let r = result?;
    let id = next_id_internal(&mut state.turn_state);
    state.turn_state.request_queue.push_back((r.content.clone(), id.clone()));
    let content = r.content;
    emit(state, Event::FollowUpDelivered { content: content.clone(), id: id.clone() });
    Some((content, id))
}

/// Generate the next request ID.
fn next_id_internal(turn_state: &mut crate::actors::turn::state::TurnState) -> String {
    let id = format!("req.{}", turn_state.next_id);
    turn_state.next_id += 1;
    id
}

/// Handle `TurnMsg::Dequeue` — pop and emit the oldest queued message.
pub(super) fn handle_dequeue(state: &mut TurnActorState) {
    let content = state.turn_state.message_queue.pop().map(|m| m.content);
    if let Some(content) = content {
        emit(state, Event::MessageDequeued { content });
    }
}

/// Handle `TurnMsg::Thinking` — record thinking start time.
pub(super) fn handle_thinking(state: &mut TurnActorState, id: String) {
    state.turn_state.thinking_started_at = Some(std::time::Instant::now());
    emit(state, Event::Thinking { id });
}

/// Handle `TurnMsg::ThoughtDone` — clear thinking start time.
pub(super) fn handle_thought_done(state: &mut TurnActorState) {
    state.turn_state.thinking_started_at = None;
}

/// Handle `TurnMsg::ToolStart` — record tool start time and emit event.
pub(super) fn handle_tool_start(state: &mut TurnActorState, id: String, name: String) {
    state.turn_state.tool_started_at = Some(std::time::Instant::now());
    state.turn_state.current_tool_name = Some(name.clone());
    state.turn_state.intermediate_step_count += 1;
    emit(
        state,
        Event::ToolStart {
            id,
            name,
            input: serde_json::Value::Null,
        },
    );
}

/// Handle `TurnMsg::ToolEnd` — clear tool state and emit event.
pub(super) fn handle_tool_end(state: &mut TurnActorState, id: String, duration_secs: f64, output: String) {
    state.turn_state.tool_started_at = None;
    state.turn_state.current_tool_name = None;
    emit(state, Event::tool_end(id, duration_secs, output));
}

/// Handle `TurnMsg::ResponseDelta` — emit streaming delta.
pub(super) fn handle_response_delta(state: &mut TurnActorState, id: String, content: String) {
    let was_streaming = state.turn_state.streaming;
    if !was_streaming {
        state.turn_state.streaming = true;
        emit(state, Event::StreamStarted { id: id.clone() });
    }
    emit(state, Event::ResponseDelta { id, content });
}

/// Handle `TurnMsg::TurnComplete` — finalize turn and emit event.
pub(super) fn handle_turn_complete(state: &mut TurnActorState, id: String, duration_secs: f64) {
    state.turn_state.turn_started_at = None;
    state.turn_state.turn_tokens_out = 0;
    emit(state, Event::TurnComplete { id, duration_secs });
}

/// Handle `TurnMsg::Done` — clear turn-active state.
pub(super) fn handle_done(state: &mut TurnActorState) {
    state.turn_state.streaming = false;
    state.turn_state.turn_active = false;
    state.turn_state.inflight = state.turn_state.inflight.saturating_sub(1);
    state.turn_state.current_tool_name = None;
    emit(state, Event::TurnCompleted);
}

/// Handle `TurnMsg::Error` — reset state and emit error event.
pub(super) fn handle_error(state: &mut TurnActorState, id: String, message: String) {
    state.turn_state.streaming = false;
    state.turn_state.turn_active = false;
    state.turn_state.inflight = 0;
    emit(state, Event::TurnErrored { id, message });
}

/// Handle `TurnMsg::UpdateSpeed` — update token speed tracking.
pub(super) fn handle_update_speed(state: &mut TurnActorState, tokens_out: usize) {
    state.turn_state.tokens_out = tokens_out;
    state.turn_state.turn_tokens_out = tokens_out;
    state.turn_state.speed_window.record(tokens_out);
    state.turn_state.speed_tps = state.turn_state.speed_window.speed();
    state.turn_state.last_speed_update = Some(std::time::Instant::now());
    state.turn_state.tokens_at_last_speed = tokens_out;
    let tokens_in = state.turn_state.tokens_in;
    let speed_tps = state.turn_state.speed_tps;
    emit(
        state,
        Event::TokenStatsUpdated {
            tokens_in,
            tokens_out,
            speed_tps,
        },
    );
}

/// Handle `TurnMsg::NextId` — generate and emit next ID.
pub(super) fn handle_next_id(state: &mut TurnActorState) {
    use super::messages::NextIdResponse;
    let id = next_id_internal(&mut state.turn_state);
    emit(state, Event::IdGenerated(NextIdResponse { id }));
}
