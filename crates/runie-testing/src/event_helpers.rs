//! Event assertion helpers for test code.
//!
//! These helpers accept `Arc<parking_lot::Mutex<Vec<Event>>>` which is the
//! standard event-collection type used in `TestRunner`, `capture_events`, and
//! the `runie_agent` test files.

use parking_lot::Mutex;
use std::sync::Arc;

use runie_core::event::Event;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Count events matching a predicate.
pub fn count_events<F>(events: &Arc<Mutex<Vec<Event>>>, predicate: F) -> usize
where
    F: Fn(&Event) -> bool,
{
    events.lock().iter().filter(|e| predicate(e)).count()
}

/// Find the first event matching a predicate.
pub fn find_event<F>(events: &Arc<Mutex<Vec<Event>>>, predicate: F) -> Option<Event>
where
    F: Fn(&Event) -> bool,
{
    events.lock().iter().find(|e| predicate(e)).cloned()
}

/// Assert that at least one event matches the predicate.
/// Panics with a helpful message if none match.
#[track_caller]
pub fn assert_event<F>(events: &Arc<Mutex<Vec<Event>>>, predicate: F)
where
    F: Fn(&Event) -> bool,
{
    let guard = events.lock();
    if guard.iter().any(predicate) {
        return;
    }
    panic!(
        "expected event matching predicate, got:\n{:#?}",
        guard.iter().collect::<Vec<_>>()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ev_completed, ev_output_text_delta, ev_response_created};

    fn events() -> Arc<Mutex<Vec<Event>>> {
        Arc::new(Mutex::new(vec![
            ev_response_created("1"),
            ev_output_text_delta("1", "hello"),
            ev_output_text_delta("1", " world"),
            ev_completed("1"),
        ]))
    }

    #[test]
    fn count_events_filters_matching() {
        let evts = events();
        assert_eq!(
            count_events(&evts, |e| matches!(e, Event::Response { .. })),
            1
        );
        assert_eq!(
            count_events(&evts, |e| matches!(e, Event::ResponseDelta { .. })),
            2
        );
        assert_eq!(count_events(&evts, |e| matches!(e, Event::Done { .. })), 1);
        assert_eq!(count_events(&evts, |e| matches!(e, Event::Error { .. })), 0);
    }

    #[test]
    fn find_event_returns_first_match() {
        let evts = events();
        assert!(find_event(&evts, |e| matches!(e, Event::Response { .. })).is_some());
        assert!(find_event(&evts, |e| matches!(e, Event::ResponseDelta { .. })).is_some());
        assert!(find_event(&evts, |e| matches!(e, Event::Error { .. })).is_none());
    }

    #[test]
    fn assert_event_passes_on_match() {
        let evts = events();
        assert_event(&evts, |e| matches!(e, Event::Done { .. })); // no panic
    }

    #[test]
    #[should_panic(expected = "expected event matching predicate")]
    fn assert_event_panics_on_miss() {
        let evts = events();
        assert_event(&evts, |e| matches!(e, Event::Error { .. }));
    }
}
