use crate::Event;
use crate::message::Part;
use crate::model::{AppState, ChatMessage, Role};
use crate::tests::fresh_state;

fn add_messages(state: &mut AppState, count: usize) {
    for i in 0..count {
        state.session.messages.push(ChatMessage {
            role: Role::User,
            parts: vec![Part::Text { content: format!("msg{}", i) }],
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.messages_changed();
    state.ensure_fresh();
}

// ── Submit resets scroll ──────────────────────────────────────────────

#[test]
fn submit_resets_scroll_to_bottom() {
    let mut state = fresh_state();
    add_messages(&mut state, 20);
    state.view.scroll = 10; // scrolled up

    state.update(crate::Event::Input('h'));
    state.update(crate::Event::Input('i'));
    state.update(Event::submit());

    assert_eq!(state.view.scroll, 0, "Submit must reset scroll to bottom");
}

#[test]
fn submit_when_turn_active_resets_scroll() {
    let mut state = fresh_state();
    state.agent.turn_active = true;
    state.view.scroll = 5;

    state.update(crate::Event::Input('s'));
    state.update(crate::Event::Input('t'));
    state.update(crate::Event::Input('e'));
    state.update(crate::Event::Input('e'));
    state.update(crate::Event::Input('r'));
    state.update(Event::submit());

    assert_eq!(
        state.view.scroll, 0,
        "Steering submit must reset scroll to bottom"
    );
}

// ── FollowUp resets scroll ────────────────────────────────────────────

#[test]
fn follow_up_resets_scroll_to_bottom() {
    let mut state = fresh_state();
    add_messages(&mut state, 20);
    state.view.scroll = 5;
    state.input.input = "follow".to_string();

    state.update(crate::Event::FollowUp);

    assert_eq!(state.view.scroll, 0, "FollowUp must reset scroll to bottom");
}

// ── Queued delivery resets scroll ─────────────────────────────────────

#[test]
fn steering_delivery_resets_scroll() {
    let mut state = fresh_state();
    state.agent.message_queue.push(crate::model::QueuedMessage {
        content: "steer".to_string(),
        kind: crate::model::QueuedMessageKind::Steering,
    });
    state.view.scroll = 5;

    state.deliver_queued();

    assert_eq!(
        state.view.scroll, 0,
        "Steering delivery must reset scroll to bottom"
    );
}

#[test]
fn follow_up_delivery_resets_scroll() {
    let mut state = fresh_state();
    state.agent.message_queue.push(crate::model::QueuedMessage {
        content: "follow".to_string(),
        kind: crate::model::QueuedMessageKind::FollowUp,
    });
    state.view.scroll = 5;

    state.deliver_queued();

    assert_eq!(
        state.view.scroll, 0,
        "Follow-up delivery must reset scroll to bottom"
    );
}

// ── Agent content visible when at bottom ──────────────────────────────

#[test]
fn at_bottom_shows_new_agent_response() {
    let mut state = fresh_state();
    add_messages(&mut state, 20);
    state.view.scroll = 0;

    state.update(crate::Event::Response {
        id: "req.0".to_string(),
        content: "hi".to_string(),
    });
    state.ensure_fresh();

    let visible = crate::tests::visible_helper::compute_viewport(&state, 5);
    let last = visible.elements.last().unwrap();
    match last {
        crate::view::elements::Element::AgentMessage { content, .. } => {
            assert_eq!(
                content, "hi",
                "New agent response should be visible at bottom"
            );
        }
        _ => panic!("Expected AgentMessage at bottom, got: {:?}", last),
    }
}

#[test]
fn at_bottom_shows_new_thought() {
    let mut state = fresh_state();
    add_messages(&mut state, 20);
    state.view.scroll = 0;

    state.update(crate::Event::Thinking {
        id: "req.0".to_string(),
    });
    state.update(crate::Event::Response {
        id: "req.0".to_string(),
        content: "Thinking...".to_string(),
    });
    state.update(crate::Event::ThoughtDone {
        id: "req.0".to_string(),
    });
    state.ensure_fresh();

    let visible = crate::tests::visible_helper::compute_viewport(&state, 5);
    let last = visible.elements.last().unwrap();
    assert!(
        matches!(last, crate::view::elements::Element::ThoughtMarker { .. }),
        "New thought should be visible at bottom: {:?}",
        last
    );
}

#[test]
fn at_bottom_shows_new_tool() {
    let mut state = fresh_state();
    add_messages(&mut state, 20);
    state.view.scroll = 0;

    state.update(crate::Event::ToolStart {
        id: "req.0".to_string(),
        name: "ls".to_string(),
        input: serde_json::Value::Null,
    });
    state.update(crate::Event::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.5,
        output: "file1".to_string(),
    });
    state.ensure_fresh();

    let visible = crate::tests::visible_helper::compute_viewport(&state, 5);
    let last = visible.elements.last().unwrap();
    assert!(
        matches!(last, crate::view::elements::Element::ToolDone { .. }),
        "New tool should be visible at bottom: {:?}",
        last
    );
}

// ── Manual scroll preserved during streaming ──────────────────────────

#[test]
fn scrolled_up_stays_scrolled_up_on_agent_response() {
    let mut state = fresh_state();
    add_messages(&mut state, 20);
    state.view.scroll = 10;

    state.update(crate::Event::Response {
        id: "req.0".to_string(),
        content: "new".to_string(),
    });
    state.ensure_fresh();

    assert_eq!(
        state.view.scroll, 10,
        "Manual scroll should be preserved when agent responds"
    );
}

#[test]
fn scroll_up_and_down_returns_to_bottom() {
    let mut state = fresh_state();
    add_messages(&mut state, 20);
    state.view.scroll = 0;

    state.update(crate::Event::Up);
    state.update(crate::Event::Up);
    state.update(crate::Event::Up);
    assert_eq!(state.view.scroll, 3, "ScrollUp should increase scroll");

    state.update(crate::Event::Down);
    state.update(crate::Event::Down);
    state.update(crate::Event::Down);
    assert_eq!(state.view.scroll, 0, "ScrollDown should return to bottom");
}

#[test]
fn scroll_down_cannot_go_below_zero() {
    let mut state = fresh_state();
    state.view.scroll = 0;
    state.update(crate::Event::Down);
    state.update(crate::Event::Down);
    assert_eq!(
        state.view.scroll, 0,
        "ScrollDown at bottom should stay at 0"
    );
}

// ── Edge cases ────────────────────────────────────────────────────────

#[test]
fn empty_chat_scroll_is_zero() {
    let state = fresh_state();
    assert_eq!(state.view.scroll, 0, "Empty chat should have scroll=0");
    let visible = crate::tests::visible_helper::compute_viewport(&state, 5);
    assert!(
        visible.elements.is_empty(),
        "Empty chat should return empty visible"
    );
}

#[test]
fn single_message_visible() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::User,
        parts: vec![crate::message::Part::Text { content: "hello".into() }],

        timestamp: 0.0,
        id: "u1".into(),
        ..Default::default()
    });
    state.messages_changed();
    state.ensure_fresh();

    let visible = crate::tests::visible_helper::compute_viewport(&state, 5);
    assert_eq!(
        visible.elements.len(),
        1,
        "Single message should be visible"
    );
}

#[test]
fn slash_command_shows_at_bottom() {
    let mut state = fresh_state();
    add_messages(&mut state, 20);
    state.view.scroll = 5;

    state.input.input = "/help".to_string();
    state.update(Event::submit());
    state.ensure_fresh();

    assert_eq!(
        state.view.scroll, 0,
        "Slash command output should be visible at bottom"
    );
}

#[test]
fn agent_done_keeps_bottom_when_already_there() {
    let mut state = fresh_state();
    add_messages(&mut state, 10);
    state.view.scroll = 0;

    state.update(crate::Event::Done {
        id: "req.0".to_string(),
    });
    state.ensure_fresh();

    assert_eq!(
        state.view.scroll, 0,
        "AgentDone should keep user at bottom when already there"
    );
}

#[test]
fn agent_done_preserves_scroll_when_not_at_bottom() {
    let mut state = fresh_state();
    add_messages(&mut state, 10);
    state.view.scroll = 5;

    state.update(crate::Event::Done {
        id: "req.0".to_string(),
    });
    state.ensure_fresh();

    assert_eq!(
        state.view.scroll, 5,
        "AgentDone should not change scroll when user is not at bottom"
    );
}
