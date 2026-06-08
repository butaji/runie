use crate::model::{AppState, ChatMessage, Role};
use crate::event::Event;

fn fresh_state() -> AppState {
    AppState::default()
}

#[test]
fn rapid_double_submit_both_messages_present() {
    let mut state = fresh_state();
    state.update(Event::Input('l'));
    state.update(Event::Input('i'));
    state.update(Event::Input('s'));
    state.update(Event::Input('t'));
    state.update(Event::Submit);
    state.update(Event::Input('l'));
    state.update(Event::Input('i'));
    state.update(Event::Input('s'));
    state.update(Event::Input('t'));
    state.update(Event::Submit);

    let user_msgs: Vec<_> = state.messages.iter().filter(|m| m.role == Role::User).collect();
    assert_eq!(user_msgs.len(), 2, "Both user messages should be present");
}

#[test]
fn rapid_double_submit_different_ids() {
    let mut state = fresh_state();
    state.update(Event::Input('a'));
    state.update(Event::Submit);
    state.update(Event::Input('b'));
    state.update(Event::Submit);

    let ids: Vec<_> = state.messages.iter()
        .filter(|m| m.role == Role::User)
        .map(|m| m.id.clone())
        .collect();
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], ids[1], "Each user message must have a unique ID");
}

#[test]
fn rapid_submit_turn_active_clears_after_done() {
    let mut state = fresh_state();
    state.update(Event::Input('x'));
    state.update(Event::Submit);
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "first".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".into() });

    assert!(!state.turn_active, "turn_active must be false after AgentDone");
    assert!(state.current_action.is_none(), "current_action must be None after turn completes");
}

#[test]
fn second_submit_while_first_active_queues_correctly() {
    let mut state = fresh_state();
    state.update(Event::Input('a'));
    state.update(Event::Submit);
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::Input('b'));
    state.update(Event::Submit);

    assert_eq!(state.message_queue.len(), 1, "Second message should be in message_queue as steering");
    assert_eq!(state.message_queue[0].content, "b", "Steering message should be 'b'");
}

#[test]
fn rapid_submit_both_turns_complete() {
    let mut state = fresh_state();
    state.update(Event::Input('a'));
    state.update(Event::Submit);
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "first".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".into() });

    state.update(Event::AgentThinking { id: "req.1".into() });
    state.update(Event::AgentResponse { id: "req.1".into(), content: "second".into() });
    state.update(Event::AgentTurnComplete { id: "req.1".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.1".into() });

    let turn_count = state.messages.iter().filter(|m| m.role == Role::TurnComplete).count();
    assert_eq!(turn_count, 2, "Each turn should have its own TurnComplete");
}

#[test]
fn turn_complete_not_overwritten_by_second_turn() {
    let mut state = fresh_state();
    state.update(Event::Input('a'));
    state.update(Event::Submit);
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "first".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });

    state.update(Event::Input('b'));
    state.update(Event::Submit);
    state.update(Event::AgentThinking { id: "req.1".into() });
    state.update(Event::AgentResponse { id: "req.1".into(), content: "second".into() });
    state.update(Event::AgentTurnComplete { id: "req.1".into(), duration_secs: 2.0 });

    let turns: Vec<_> = state.messages.iter().filter(|m| m.role == Role::TurnComplete).collect();
    assert_eq!(turns.len(), 2, "Should have 2 TurnComplete messages");
}

#[test]
fn deliver_queued_moves_steering_to_request_queue() {
    let mut state = fresh_state();
    state.update(Event::Input('a'));
    state.update(Event::Submit);
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::Input('b'));
    state.update(Event::Submit);

    assert_eq!(state.message_queue.len(), 1, "Steering should be queued");

    state.update(Event::AgentResponse { id: "req.0".into(), content: "done".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".into() });

    assert!(state.message_queue.is_empty(), "Steering should be delivered");
    let queued: Vec<String> = state.request_queue.iter().map(|(c, _)| c.clone()).collect();
    assert!(queued.contains(&"b".to_string()), "Delivered request should be 'b'. Got: {:?}", queued);
}

#[test]
fn two_full_turns_with_tools_state_correct() {
    let mut state = fresh_state();
    
    // Turn 1: "list files" → tool call
    state.update(Event::Input('l'));
    state.update(Event::Input('i'));
    state.update(Event::Input('s'));
    state.update(Event::Input('t'));
    state.update(Event::Submit);
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "I'll list files.\nTOOL:list_dir:.".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "list_dir".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "file1\nfile2".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Done.".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".into() });

    // Turn 2: also "list files"
    state.update(Event::AgentThinking { id: "req.1".into() });
    state.update(Event::AgentResponse { id: "req.1".into(), content: "I'll list files again.\nTOOL:list_dir:.".into() });
    state.update(Event::AgentThoughtDone { id: "req.1".into() });
    state.update(Event::AgentToolStart { id: "req.1".into(), name: "list_dir".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.3, output: "file1\nfile2".into() });
    state.update(Event::AgentResponse { id: "req.1".into(), content: "Done again.".into() });
    state.update(Event::AgentTurnComplete { id: "req.1".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.1".into() });

    assert!(!state.turn_active, "turn_active must be false after both turns");
    assert!(state.current_action.is_none(), "current_action must be None");
    assert_eq!(state.inflight, 0, "inflight must be 0");
    
    let turns: Vec<_> = state.messages.iter().filter(|m| m.role == Role::TurnComplete).collect();
    assert_eq!(turns.len(), 2, "Should have 2 TurnComplete messages");
}

#[test]
fn steering_message_delivered_after_done() {
    let mut state = fresh_state();
    state.update(Event::Input('a'));
    state.update(Event::Submit);
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::Input('b'));
    state.update(Event::Submit);

    assert_eq!(state.message_queue.len(), 1);

    state.update(Event::AgentResponse { id: "req.0".into(), content: "done".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".into() });

    assert!(state.message_queue.is_empty(), "Steering should be delivered to request_queue");
}

fn run_turn_with_tool(state: &mut AppState, id: &str, tool_name: &str) {
    state.update(Event::AgentThinking { id: id.into() });
    state.update(Event::AgentResponse { id: id.into(), content: format!("I'll list files.\nTOOL:{}:.", tool_name) });
    state.update(Event::AgentThoughtDone { id: id.into() });
    state.update(Event::AgentToolStart { id: id.into(), name: tool_name.into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "file1\nfile2".into() });
    state.update(Event::AgentResponse { id: id.into(), content: "Done.".into() });
    state.update(Event::AgentTurnComplete { id: id.into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: id.into() });
}

#[test]
fn end_tool_updates_content() {
    let mut state = fresh_state();
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "list_dir".into() });
    assert_eq!(state.messages.len(), 1);
    assert_eq!(state.messages[0].content, "⠋ Running list_dir...");
    
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "file1\nfile2".into() });
    assert_eq!(state.messages[0].content, "✓ list_dir 0.5s\nfile1\nfile2");
}

#[test]
fn two_turns_with_tools_no_stuck_timer() {
    let mut state = fresh_state();
    state.update(Event::Input('a'));
    state.update(Event::Submit);
    run_turn_with_tool(&mut state, "req.0", "list_dir");
    assert!(state.tool_started_at.is_none(), "tool_started_at must be cleared after end_tool");

    run_turn_with_tool(&mut state, "req.1", "list_dir");
    assert!(state.tool_started_at.is_none(), "tool_started_at must be cleared after second end_tool");
    
    let tools: Vec<_> = state.messages.iter().filter(|m| m.role == Role::Tool).collect();
    assert_eq!(tools.len(), 2, "Should have 2 tool messages");
    for tool in &tools {
        assert!(!tool.content.contains("⠋ Running"), "Tool should be done, not running: {}", tool.content);
    }
}
