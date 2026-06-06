use crate::model::{AppState, ChatMessage, Role};
use crate::event::Event;

#[test]
fn queue_empty_by_default() {
    let state = AppState::default();
    assert!(state.message_queue.is_empty());
}

#[test]
fn submit_during_turn_queues_steering() {
    let mut state = AppState::default();
    state.turn_active = true;
    state.update(Event::Input('h'));
    state.update(Event::Input('i'));
    state.update(Event::Submit);
    assert_eq!(state.message_queue.len(), 1);
    assert_eq!(state.message_queue[0].content, "hi");
    assert!(matches!(state.message_queue[0].kind, crate::model::QueuedMessageKind::Steering));
    assert!(state.input.is_empty());
}

#[test]
fn submit_when_idle_sends_immediately() {
    let mut state = AppState::default();
    state.turn_active = false;
    state.update(Event::Input('h'));
    state.update(Event::Input('i'));
    state.update(Event::Submit);
    assert!(state.message_queue.is_empty());
    assert_eq!(state.messages.len(), 1);
    assert_eq!(state.messages[0].role, Role::User);
    assert_eq!(state.messages[0].content, "hi");
}

#[test]
fn follow_up_queues_for_later() {
    let mut state = AppState::default();
    state.turn_active = true;
    state.update(Event::Input('b'));
    state.update(Event::Input('y'));
    state.update(Event::FollowUp);
    assert_eq!(state.message_queue.len(), 1);
    assert!(matches!(state.message_queue[0].kind, crate::model::QueuedMessageKind::FollowUp));
}

#[test]
fn deliver_queue_at_turn_end() {
    let mut state = AppState::default();
    state.turn_active = true;
    state.update(Event::Input('h'));
    state.update(Event::Input('i'));
    state.update(Event::Submit);
    assert_eq!(state.message_queue.len(), 1);

    state.update(Event::AgentDone { id: "req.0".to_string() });
    assert!(state.message_queue.is_empty());
    assert_eq!(state.messages.len(), 1);
    assert_eq!(state.messages[0].content, "hi");
}

#[test]
fn queue_multiple_messages() {
    let mut state = AppState::default();
    state.turn_active = true;
    state.update(Event::Input('a'));
    state.update(Event::Submit);
    state.update(Event::Input('b'));
    state.update(Event::FollowUp);
    assert_eq!(state.message_queue.len(), 2);
    assert!(matches!(state.message_queue[0].kind, crate::model::QueuedMessageKind::Steering));
    assert!(matches!(state.message_queue[1].kind, crate::model::QueuedMessageKind::FollowUp));
}

#[test]
fn escape_clears_queue_and_restores() {
    let mut state = AppState::default();
    state.turn_active = true;
    state.update(Event::Input('h'));
    state.update(Event::Input('i'));
    state.update(Event::Submit);
    assert_eq!(state.message_queue.len(), 1);

    state.update(Event::Abort);
    assert!(state.message_queue.is_empty());
    assert_eq!(state.input, "hi");
}

#[test]
fn steering_delivered_before_follow_up() {
    let mut state = AppState::default();
    state.turn_active = true;
    state.update(Event::Input('s'));
    state.update(Event::Submit);
    state.update(Event::Input('f'));
    state.update(Event::FollowUp);

    state.update(Event::AgentDone { id: "req.0".to_string() });
    let user_msgs: Vec<&ChatMessage> = state.messages.iter().filter(|m| m.role == Role::User).collect();
    assert_eq!(user_msgs.len(), 1);
    assert_eq!(user_msgs[0].content, "s");

    state.update(Event::AgentDone { id: "req.1".to_string() });
    let user_msgs: Vec<&ChatMessage> = state.messages.iter().filter(|m| m.role == Role::User).collect();
    assert_eq!(user_msgs.len(), 2);
    assert_eq!(user_msgs[1].content, "f");
}
