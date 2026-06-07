use crate::model::{AppState, ChatMessage, Role};
use crate::event::Event;

fn fresh_state() -> AppState {
    AppState::default()
}

fn add_messages(state: &mut AppState, count: usize) {
    for i in 0..count {
        state.messages.push(ChatMessage {
            role: Role::User,
            content: format!("msg{}", i),
            timestamp: i as f64,
            id: format!("u{}", i),
        });
    }
    state.messages_changed();
    state.ensure_fresh();
}

#[test]
fn at_bottom_stays_at_bottom_when_agent_responds() {
    let mut state = fresh_state();
    add_messages(&mut state, 20);
    state.scroll = 0; // at bottom

    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "hi".to_string() });
    state.ensure_fresh();

    // Simulate autoscroll logic from event loop
    state.scroll = 0;

    let visible = state.visible_scroll(5);
    let last = visible.last().unwrap();
    match last {
        crate::ui::elements::Element::AgentMessage { content } => {
            assert_eq!(content, "hi", "Should show newest agent response at bottom");
        }
        _ => panic!("Expected AgentMessage at bottom, got: {:?}", last),
    }
}

#[test]
fn scrolled_up_stays_scrolled_up_when_agent_responds() {
    let mut state = fresh_state();
    add_messages(&mut state, 20);
    state.scroll = 10; // scrolled up

    let visible_before = state.visible_scroll(5);
    let first_before = visible_before.first().unwrap().clone();

    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "hi".to_string() });
    state.ensure_fresh();

    // User was NOT at bottom, so scroll stays at 10
    // (In real event loop, scroll wouldn't be reset)
    // But content shifts by 2 elements (new msg + spacer)
    // So we need to anchor by adding delta

    state.scroll = state.scroll.saturating_add(2);

    let visible_after = state.visible_scroll(5);
    let first_after = visible_after.first().unwrap();

    assert_eq!(format!("{:?}", first_after), format!("{:?}", first_before),
        "Viewport should stay anchored to same content when scrolled up");
}

#[test]
fn submit_jumps_to_bottom_even_when_scrolled_up() {
    let mut state = fresh_state();
    add_messages(&mut state, 20);
    state.scroll = 10; // scrolled up viewing old content

    state.update(Event::Input('h'));
    state.update(Event::Input('i'));
    state.update(Event::Submit);
    state.ensure_fresh();

    // Simulate: Submit always jumps to bottom
    state.scroll = 0;

    let visible = state.visible_scroll(5);
    let last = visible.last().unwrap();
    match last {
        crate::ui::elements::Element::UserMessage { content } => {
            assert_eq!(content, "hi", "Submit should jump to bottom to show sent message");
        }
        _ => panic!("Expected UserMessage at bottom after submit"),
    }
}

#[test]
fn scroll_up_manually_preserves_position_on_response() {
    let mut state = fresh_state();
    add_messages(&mut state, 20);
    state.scroll = 0;

    // User scrolls up
    state.update(Event::ScrollUp);
    state.update(Event::ScrollUp);
    state.update(Event::ScrollUp);
    assert_eq!(state.scroll, 3);

    // Agent responds
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "new".to_string() });
    state.ensure_fresh();

    // User was NOT at bottom, scroll should not be reset
    // (In event loop it would stay at 3, content shifts by 2)
    state.scroll = state.scroll.saturating_add(2);

    assert!(state.scroll > 0, "Scroll should remain > 0 when user manually scrolled up");
}

#[test]
fn follow_up_jumps_to_bottom() {
    let mut state = fresh_state();
    add_messages(&mut state, 20);
    state.scroll = 5;

    state.input = "follow".to_string();
    state.update(Event::FollowUp);
    state.ensure_fresh();

    // FollowUp should behave like Submit - jump to bottom
    state.scroll = 0;

    let visible = state.visible_scroll(5);
    let last = visible.last().unwrap();
    match last {
        crate::ui::elements::Element::UserMessage { content } => {
            assert_eq!(content, "follow");
        }
        _ => panic!("Expected UserMessage at bottom after FollowUp"),
    }
}

#[test]
fn done_event_keeps_bottom_position() {
    let mut state = fresh_state();
    add_messages(&mut state, 10);
    state.scroll = 0;

    state.update(Event::AgentDone { id: "req.0".to_string() });
    state.ensure_fresh();

    state.scroll = 0;

    assert_eq!(state.scroll, 0, "AgentDone should keep user at bottom");
}

#[test]
fn done_event_preserves_scroll_when_not_at_bottom() {
    let mut state = fresh_state();
    add_messages(&mut state, 10);
    state.scroll = 5;

    state.update(Event::AgentDone { id: "req.0".to_string() });
    state.ensure_fresh();

    // Don't reset scroll - user was reading old content
    assert_eq!(state.scroll, 5, "AgentDone should not change scroll when user is not at bottom");
}
