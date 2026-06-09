use crate::model::{AppState, ChatMessage, Role};
use crate::event::Event;

#[test]
fn queue_empty_by_default() {
    let state = AppState::default();
    assert!(state.agent.message_queue.is_empty());
}

#[test]
fn submit_during_turn_queues_steering() {
    let mut state = AppState::default();
    state.agent.turn_active = true;
    state.update(Event::Input('h'));
    state.update(Event::Input('i'));
    state.update(Event::Submit);
    assert_eq!(state.agent.message_queue.len(), 1);
    assert_eq!(state.agent.message_queue[0].content, "hi");
    assert!(matches!(state.agent.message_queue[0].kind, crate::model::QueuedMessageKind::Steering));
    assert!(state.input.input.is_empty());
}

#[test]
fn submit_when_idle_sends_immediately() {
    let mut state = AppState::default();
    state.agent.turn_active = false;
    state.update(Event::Input('h'));
    state.update(Event::Input('i'));
    state.update(Event::Submit);
    assert!(state.agent.message_queue.is_empty());
    assert_eq!(state.session.messages.len(), 1);
    assert_eq!(state.session.messages[0].role, Role::User);
    assert_eq!(state.session.messages[0].content, "hi");
}

#[test]
fn follow_up_queues_for_later() {
    let mut state = AppState::default();
    state.agent.turn_active = true;
    state.update(Event::Input('b'));
    state.update(Event::Input('y'));
    state.update(Event::FollowUp);
    assert_eq!(state.agent.message_queue.len(), 1);
    assert!(matches!(state.agent.message_queue[0].kind, crate::model::QueuedMessageKind::FollowUp));
}

#[test]
fn deliver_queue_at_turn_end() {
    let mut state = AppState::default();
    state.agent.turn_active = true;
    state.update(Event::Input('h'));
    state.update(Event::Input('i'));
    state.update(Event::Submit);
    assert_eq!(state.agent.message_queue.len(), 1);

    state.update(Event::AgentDone { id: "req.0".to_string() });
    assert!(state.agent.message_queue.is_empty());
    assert_eq!(state.session.messages.len(), 1);
    assert_eq!(state.session.messages[0].content, "hi");
}

#[test]
fn queue_multiple_messages() {
    let mut state = AppState::default();
    state.agent.turn_active = true;
    state.update(Event::Input('a'));
    state.update(Event::Submit);
    state.update(Event::Input('b'));
    state.update(Event::FollowUp);
    assert_eq!(state.agent.message_queue.len(), 2);
    assert!(matches!(state.agent.message_queue[0].kind, crate::model::QueuedMessageKind::Steering));
    assert!(matches!(state.agent.message_queue[1].kind, crate::model::QueuedMessageKind::FollowUp));
}

#[test]
fn escape_clears_queue_and_restores() {
    let mut state = AppState::default();
    state.agent.turn_active = true;
    state.update(Event::Input('h'));
    state.update(Event::Input('i'));
    state.update(Event::Submit);
    assert_eq!(state.agent.message_queue.len(), 1);

    state.update(Event::Abort);
    assert!(state.agent.message_queue.is_empty());
    assert_eq!(state.input.input, "hi");
}

#[test]
fn steering_delivered_before_follow_up() {
    let mut state = AppState::default();
    state.agent.turn_active = true;
    state.update(Event::Input('s'));
    state.update(Event::Submit);
    state.update(Event::Input('f'));
    state.update(Event::FollowUp);

    state.update(Event::AgentDone { id: "req.0".to_string() });
    let user_msgs: Vec<&ChatMessage> = state.session.messages.iter().filter(|m| m.role == Role::User).collect();
    assert_eq!(user_msgs.len(), 1);
    assert_eq!(user_msgs[0].content, "s");

    state.update(Event::AgentDone { id: "req.1".to_string() });
    let user_msgs: Vec<&ChatMessage> = state.session.messages.iter().filter(|m| m.role == Role::User).collect();
    assert_eq!(user_msgs.len(), 2);
    assert_eq!(user_msgs[1].content, "f");
}

// Delivery mode tests

#[test]
fn delivery_mode_defaults_to_one_at_a_time() {
    let state = AppState::default();
    assert!(matches!(state.steering_mode, crate::model::DeliveryMode::OneAtATime));
    assert!(matches!(state.follow_up_mode, crate::model::DeliveryMode::OneAtATime));
}

#[test]
fn steering_mode_all_batches_messages() {
    use crate::model::DeliveryMode;
    let mut state = AppState::default();
    state.agent.turn_active = true;
    state.steering_mode = DeliveryMode::All;

    // Queue three steering messages
    state.update(Event::Input('a'));
    state.update(Event::Submit);
    state.update(Event::Input('b'));
    state.update(Event::Submit);
    state.update(Event::Input('c'));
    state.update(Event::Submit);

    assert_eq!(state.agent.message_queue.len(), 3);

    // Trigger delivery
    state.update(Event::AgentDone { id: "req.0".to_string() });

    // All three should be batched into one request
    assert!(state.agent.message_queue.is_empty());
    assert_eq!(state.agent.request_queue.len(), 1);
    let (content, _) = &state.agent.request_queue[0];
    assert_eq!(content, "a\nb\nc");
}

#[test]
fn follow_up_mode_all_batches_messages() {
    use crate::model::DeliveryMode;
    let mut state = AppState::default();
    state.agent.turn_active = true;
    state.follow_up_mode = DeliveryMode::All;

    // Queue three follow-up messages
    state.update(Event::Input('x'));
    state.update(Event::FollowUp);
    state.update(Event::Input('y'));
    state.update(Event::FollowUp);
    state.update(Event::Input('z'));
    state.update(Event::FollowUp);

    assert_eq!(state.agent.message_queue.len(), 3);

    // First complete a turn to trigger delivery
    state.update(Event::Input('i'));
    state.update(Event::Submit);
    
    // After submit, message_queue should have [x, y, z, "i"]
    assert_eq!(state.agent.message_queue.len(), 4, "Expected 4 queued messages before turn done");
    
    state.update(Event::AgentDone { id: "req.0".to_string() });

    // All three follow-ups should be batched into one request
    assert!(state.agent.message_queue.is_empty(), "Expected empty queue after turn done, got {:?}", state.agent.message_queue);
    assert_eq!(state.agent.request_queue.len(), 2); // init + batched follow-ups
    let (_, id) = &state.agent.request_queue[1];
    assert_eq!(id, "req.1"); // The batched request
}

#[test]
fn one_at_a_time_delivers_separately() {
    use crate::model::DeliveryMode;
    let mut state = AppState::default();
    state.agent.turn_active = true;
    state.steering_mode = DeliveryMode::OneAtATime;

    // Queue three steering messages
    state.update(Event::Input('a'));
    state.update(Event::Submit);
    state.update(Event::Input('b'));
    state.update(Event::Submit);
    state.update(Event::Input('c'));
    state.update(Event::Submit);

    assert_eq!(state.agent.message_queue.len(), 3);

    // Trigger first delivery
    state.update(Event::AgentDone { id: "req.0".to_string() });

    // Only one should be delivered
    assert_eq!(state.agent.message_queue.len(), 2);
    assert_eq!(state.agent.request_queue.len(), 1);

    // Second delivery
    state.update(Event::AgentDone { id: "req.1".to_string() });
    assert_eq!(state.agent.message_queue.len(), 1);
    assert_eq!(state.agent.request_queue.len(), 2);
}

#[test]
fn steering_and_follow_up_modes_independent() {
    use crate::model::DeliveryMode;
    let mut state = AppState::default();
    state.agent.turn_active = true;
    state.steering_mode = DeliveryMode::All;
    state.follow_up_mode = DeliveryMode::OneAtATime;

    // Queue two steering and two follow-up
    state.update(Event::Input('a'));
    state.update(Event::Submit);
    state.update(Event::Input('b'));
    state.update(Event::Submit);
    state.update(Event::Input('x'));
    state.update(Event::FollowUp);
    state.update(Event::Input('y'));
    state.update(Event::FollowUp);

    // Complete a turn - steering should batch, follow-up should not
    state.update(Event::AgentDone { id: "req.0".to_string() });

    assert_eq!(state.agent.message_queue.len(), 2); // Two follow-ups still queued
    assert_eq!(state.agent.request_queue.len(), 1); // One batched steering
    assert_eq!(state.agent.request_queue[0].0, "a\nb"); // Batched content

    // Complete second turn - first follow-up delivered
    state.update(Event::AgentDone { id: "req.1".to_string() });
    assert_eq!(state.agent.message_queue.len(), 1); // One follow-up left
    assert_eq!(state.agent.request_queue.len(), 2);
}

// Dequeue tests

#[test]
fn dequeue_restores_last() {
    let mut state = AppState::default();
    state.agent.turn_active = true;
    state.update(Event::Input('h'));
    state.update(Event::Input('i'));
    state.update(Event::Submit);
    assert_eq!(state.agent.message_queue.len(), 1);

    state.update(Event::Dequeue);
    assert!(state.agent.message_queue.is_empty());
    assert_eq!(state.input.input, "hi");
}

#[test]
fn dequeue_sets_cursor_end() {
    let mut state = AppState::default();
    state.agent.turn_active = true;
    state.update(Event::Input('a'));
    state.update(Event::Input('b'));
    state.update(Event::Submit);

    state.update(Event::Dequeue);
    assert_eq!(state.input.cursor_pos, 2);
}

#[test]
fn dequeue_replaces_input() {
    let mut state = AppState::default();
    state.agent.turn_active = true;
    state.update(Event::Input('o'));
    state.update(Event::Input('l'));
    state.update(Event::Input('d'));
    state.update(Event::Submit);

    state.update(Event::Input('n'));
    state.update(Event::Input('e'));
    state.update(Event::Input('w'));
    assert_eq!(state.input.input, "new");

    state.update(Event::Dequeue);
    assert_eq!(state.input.input, "old");
}

#[test]
fn dequeue_empty_flashes() {
    let mut state = AppState::default();
    assert!(state.agent.message_queue.is_empty());
    assert_eq!(state.input.input_flash, 0);

    state.update(Event::Dequeue);
    assert_eq!(state.input.input_flash, 3);
}

#[test]
fn dequeue_lifo() {
    let mut state = AppState::default();
    state.agent.turn_active = true;
    state.update(Event::Input('a'));
    state.update(Event::Submit);
    state.update(Event::Input('b'));
    state.update(Event::FollowUp);
    assert_eq!(state.agent.message_queue.len(), 2);

    state.update(Event::Dequeue);
    assert_eq!(state.agent.message_queue.len(), 1);
    assert_eq!(state.input.input, "b");

    state.update(Event::Dequeue);
    assert!(state.agent.message_queue.is_empty());
    assert_eq!(state.input.input, "a");
}
