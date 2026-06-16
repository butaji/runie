//! Tests for empty line between elements in chat feed.

use crate::event::Event;

use crate::event::{InputEvent, ControlEvent, ModelConfigEvent, SystemEvent, DialogEvent, ScrollEvent, AgentEvent, SessionEvent, EditEvent, CommandEvent, DurableCoreEvent};
use crate::layout::element_line_count;
use crate::model::AppState;
use crate::ui::LazyCache;

const TEST_WIDTH: u16 = 80;

fn fresh_state() -> AppState {
    AppState::default()
}

fn _feed_lines(state: &AppState) -> usize {
    let feed = LazyCache::feed(state);
    feed.elements
        .iter()
        .map(|e| element_line_count(e, TEST_WIDTH))
        .sum()
}

#[test]
fn spacer_contributes_one_line() {
    let mut state = fresh_state();
    state.update(Event::Input(InputEvent::Input('H')));
    state.update(Event::submit());
    state.ensure_fresh();
    let feed = LazyCache::feed(&state);
    let spacers: Vec<_> = feed
        .elements
        .iter()
        .filter(|e| matches!(e, crate::ui::Element::Spacer { .. }))
        .collect();
    assert!(!spacers.is_empty(), "Feed should have spacers");
    for spacer in spacers {
        assert_eq!(
            element_line_count(spacer, TEST_WIDTH),
            1,
            "Spacer must contribute exactly 1 empty line"
        );
    }
}

#[test]
fn single_user_message_has_spacer_after() {
    let mut state = fresh_state();
    state.update(Event::Input(InputEvent::Input('H')));
    state.update(Event::Input(InputEvent::Input('i')));
    state.update(Event::submit());
    state.ensure_fresh();
    let feed = LazyCache::feed(&state);
    assert_eq!(feed.elements.len(), 2, "UserMessage + Spacer");
    assert!(matches!(
        feed.elements[1],
        crate::ui::Element::Spacer { .. }
    ));
    assert_eq!(element_line_count(&feed.elements[1], TEST_WIDTH), 1);
}

#[test]
fn two_messages_have_spacer_between_and_after() {
    let mut state = fresh_state();
    state.update(Event::Input(InputEvent::Input('A')));
    state.update(Event::submit());
    state.agent.streaming = true;
    state.update(Event::Agent(AgentEvent::Response {
        id: "req.0".into(),
        content: "B".into(),
    }));
    state.update(Event::Agent(AgentEvent::Done { id: "req.0".into() }));
    state.ensure_fresh();
    let feed = LazyCache::feed(&state);
    // Expected: UserMessage, Spacer, AgentMessage, Spacer
    assert_eq!(feed.elements.len(), 4);
    assert!(matches!(
        feed.elements[1],
        crate::ui::Element::Spacer { .. }
    ));
    assert!(matches!(
        feed.elements[3],
        crate::ui::Element::Spacer { .. }
    ));
}

#[test]
fn total_lines_includes_spacers() {
    let mut state = fresh_state();
    state.update(Event::Input(InputEvent::Input('A')));
    state.update(Event::submit());
    state.ensure_fresh();
    let feed = LazyCache::feed(&state);
    let total: usize = feed
        .elements
        .iter()
        .map(|e| element_line_count(e, TEST_WIDTH))
        .sum();
    // UserMessage is 1 content line + 2 margin lines = 3, Spacer is 1 line = 4 total
    assert_eq!(total, 4, "Total lines should include spacer empty line");
}
