use crate::event::Event;
use crate::model::AppState;

fn fresh_state() -> AppState {
    AppState::default()
}

fn push_user_msg(state: &mut AppState, content: &str, id: &str) {
    state.session.messages.push(crate::model::ChatMessage {
        role: crate::model::Role::User,
        content: content.into(),
        timestamp: 0.0,
        id: id.into(),
        ..Default::default()
    });
}

fn thinking_started(state: &AppState) -> std::time::Instant {
    use crate::ui::Element;
    state
        .view
        .elements_cache()
        .iter()
        .find_map(|e| match e {
            Element::Thinking { started, .. } => Some(*started),
            _ => None,
        })
        .expect("Should have Thinking element")
}

#[test]
fn test_input_adds_character() {
    let mut state = fresh_state();
    state.update(Event::Input('H'));
    state.update(Event::Input('i'));
    assert_eq!(state.input.input, "Hi");
}

#[test]
fn test_backspace_removes_character() {
    let mut state = fresh_state();
    state.update(Event::Input('H'));
    state.update(Event::Input('i'));
    state.update(Event::Backspace);
    assert_eq!(state.input.input, "H");
}

#[test]
fn test_backspace_empty_input() {
    let mut state = fresh_state();
    state.update(Event::Backspace);
    assert_eq!(state.input.input, "");
}

#[test]
fn test_submit_empty_input() {
    let mut state = fresh_state();
    state.update(Event::Submit);
    assert_eq!(state.input.input, "");
}

#[test]
fn test_submit_reset_command() {
    let mut state = fresh_state();
    state.update(Event::Input('/'));
    state.update(Event::Input('r'));
    state.update(Event::Input('e'));
    state.update(Event::Input('s'));
    state.update(Event::Input('e'));
    state.update(Event::Input('t'));
    state.update(Event::Submit);

    assert_eq!(state.session.messages.len(), 1);
    assert!(
        state.session.messages[0].content.contains("State cleared"),
        "reset confirmation: {}",
        state.session.messages[0].content
    );
    assert_eq!(state.input.input, "");
}

#[test]
fn typing_without_at_does_not_open_dialog() {
    let mut state = fresh_state();
    for c in "hello".chars() {
        state.update(Event::Input(c));
    }
    assert!(
        state.completion.at_suggestions.is_none(),
        "Typing without @ should not trigger suggestions"
    );
    assert!(state.completion.last_at_query.is_none());
}

#[test]
fn input_change_marks_dirty_but_does_not_bump_cache_gen() {
    let mut state = fresh_state();
    state.session.messages.push(crate::model::ChatMessage {
        role: crate::model::Role::User,
        content: "hi".into(),
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.ensure_fresh();
    let gen_before = state.cache_generation();
    state.update(Event::Input('x'));
    assert!(
        state.is_dirty(),
        "Input change should mark dirty for render"
    );
    assert_eq!(
        state.cache_generation(),
        gen_before,
        "Input change must NOT bump cache generation"
    );
}

#[test]
fn message_change_bumps_cache_gen() {
    let mut state = fresh_state();
    state.ensure_fresh();
    let gen_before = state.cache_generation();
    state.update(Event::AgentResponse {
        id: "req.0".to_string(),
        content: "Hello".to_string(),
    });
    assert!(
        state.cache_generation() > gen_before,
        "Message change must bump cache generation"
    );
}

#[test]
fn ensure_fresh_skips_rebuild_when_only_input_changed() {
    let mut state = fresh_state();
    state.session.messages.push(crate::model::ChatMessage {
        role: crate::model::Role::User,
        content: "hi".into(),
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.ensure_fresh();
    let cache_before = state.view.elements_cache().len();
    state.update(Event::Input('x'));
    state.ensure_fresh();
    assert_eq!(
        state.view.elements_cache().len(),
        cache_before,
        "Only input change should skip cache rebuild"
    );
}

#[test]
fn thinking_element_stores_instant_not_elapsed() {
    use crate::ui::Element;
    let mut state = fresh_state();
    state.session.messages.push(crate::model::ChatMessage {
        role: crate::model::Role::User,
        content: "hi".into(),
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.thinking_started_at = Some(std::time::Instant::now() - std::time::Duration::from_secs(3));
    state.agent.turn_active = true;
    state.messages_changed();
    state.ensure_fresh();

    let started = state
        .view
        .elements_cache()
        .iter()
        .find_map(|e| match e {
            Element::Thinking { started, .. } => Some(*started),
            _ => None,
        })
        .expect("Should have Thinking element");

    let elapsed = started.elapsed().as_secs_f64();
    assert!(
        elapsed >= 2.9,
        "Timer should compute elapsed at render time: {:.1}s",
        elapsed
    );
}

#[test]
fn tool_running_element_stores_instant_not_elapsed() {
    use crate::ui::Element;
    let mut state = fresh_state();
    state.session.messages.push(crate::model::ChatMessage {
        role: crate::model::Role::Tool,
        content: "⠋ Running list_files...".into(),
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.agent.tool_started_at =
        Some(std::time::Instant::now() - std::time::Duration::from_secs(2));
    state.agent.turn_active = true;
    state.messages_changed();
    state.ensure_fresh();

    let started = state
        .view
        .elements_cache()
        .iter()
        .find_map(|e| match e {
            Element::ToolRunning { started, .. } => Some(*started),
            _ => None,
        })
        .expect("Should have ToolRunning element");

    let elapsed = started.elapsed().as_secs_f64();
    assert!(
        elapsed >= 1.9,
        "Tool timer should compute elapsed at render time: {:.1}s",
        elapsed
    );
}

#[test]
fn timer_advances_without_cache_rebuild() {
    let mut state = fresh_state();
    push_user_msg(&mut state, "hi", "t1");
    state.thinking_started_at = Some(std::time::Instant::now() - std::time::Duration::from_secs(5));
    state.agent.turn_active = true;
    state.messages_changed();
    state.ensure_fresh();
    let gen_before = state.cache_generation();
    state.tick_animation();
    assert_eq!(
        state.cache_generation(),
        gen_before,
        "tick_animation must not bump cache gen"
    );
    assert!(state.is_dirty(), "tick_animation must mark dirty for render");
    let elapsed = thinking_started(&state).elapsed().as_secs_f64();
    assert!(
        elapsed >= 4.9,
        "Timer should advance without cache rebuild: {:.1}s",
        elapsed
    );
}

#[test]
fn input_not_delayed_by_animation_when_idle() {
    let mut state = fresh_state();
    state.agent.turn_active = false;
    state.update(Event::Input('x'));
    assert!(state.is_dirty(), "Input must mark dirty immediately");
    assert!(
        !state.agent.turn_active,
        "Idle state must not require animation timer"
    );
}

#[test]
fn tick_animation_noop_when_not_turn_active() {
    let mut state = fresh_state();
    state.agent.turn_active = false;
    state.update(Event::Input('x'));
    state.ensure_fresh();
    let was_dirty = state.is_dirty();

    state.tick_animation();
    assert!(
        !state.is_dirty(),
        "tick_animation must be noop when !turn_active"
    );
    assert!(!was_dirty, "State should remain clean after noop tick");
}

#[test]
fn external_editor_done_updates_input() {
    let mut state = fresh_state();
    state.update(Event::Input('o'));
    state.update(Event::Input('l'));
    state.update(Event::Input('d'));
    assert_eq!(state.input.input, "old");
    assert_eq!(state.input.cursor_pos, 3);

    state.update(Event::ExternalEditorDone {
        content: "new text".to_string(),
    });
    assert_eq!(state.input.input, "new text");
    assert_eq!(state.input.cursor_pos, 8);
}
