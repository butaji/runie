//! Tests for empty line between elements in chat feed.

use crate::model::AppState;
use crate::event::Event;
use crate::ui::LazyCache;

fn fresh_state() -> AppState {
    AppState::default()
}

fn _feed_lines(state: &AppState) -> usize {
    let feed = LazyCache::feed(state);
    feed.elements.iter().map(|e| e.line_count()).sum()
}

#[test]
fn spacer_contributes_one_line() {
    let mut state = fresh_state();
    state.update(Event::Input('H'));
    state.update(Event::Submit);
    state.ensure_fresh();
    let feed = LazyCache::feed(&state);
    let spacers: Vec<_> = feed.elements.iter().filter(|e| matches!(e, crate::ui::Element::Spacer { .. })).collect();
    assert!(!spacers.is_empty(), "Feed should have spacers");
    for spacer in spacers {
        assert_eq!(spacer.line_count(), 1, "Spacer must contribute exactly 1 empty line");
    }
}

#[test]
fn single_user_message_has_spacer_after() {
    let mut state = fresh_state();
    state.update(Event::Input('H'));
    state.update(Event::Input('i'));
    state.update(Event::Submit);
    state.ensure_fresh();
    let feed = LazyCache::feed(&state);
    assert_eq!(feed.elements.len(), 2, "UserMessage + Spacer");
    assert!(matches!(feed.elements[1], crate::ui::Element::Spacer { .. }));
    assert_eq!(feed.elements[1].line_count(), 1);
}

#[test]
fn two_messages_have_spacer_between_and_after() {
    let mut state = fresh_state();
    state.update(Event::Input('A'));
    state.update(Event::Submit);
    state.streaming = true;
    state.update(Event::AgentResponse { id: "req.0".into(), content: "B".into() });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();
    let feed = LazyCache::feed(&state);
    // Expected: UserMessage, Spacer, AgentMessage, Spacer
    assert_eq!(feed.elements.len(), 4);
    assert!(matches!(feed.elements[1], crate::ui::Element::Spacer { .. }));
    assert!(matches!(feed.elements[3], crate::ui::Element::Spacer { .. }));
}

#[test]
fn total_lines_includes_spacers() {
    let mut state = fresh_state();
    state.update(Event::Input('A'));
    state.update(Event::Submit);
    state.ensure_fresh();
    let feed = LazyCache::feed(&state);
    let total: usize = feed.elements.iter().map(|e| e.line_count()).sum();
    // UserMessage is 1 content line + 2 margin lines = 3, Spacer is 1 line = 4 total
    assert_eq!(total, 4, "Total lines should include spacer empty line");
}
