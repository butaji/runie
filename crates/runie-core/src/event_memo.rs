//! Small immutable event-log memo used by replayable projections.
//!
//! The memo deliberately contains no async or actor behavior. An actor owns
//! one of these values and replaces it after receiving an event; pure callers
//! can use the same type directly in replay tests.

use std::sync::Arc;

/// Immutable, cheaply clonable view of an actor projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedSnapshot<S>(Arc<S>);

impl<S> SharedSnapshot<S> {
    pub fn new(state: S) -> Self {
        Self(Arc::new(state))
    }

    pub fn get(&self) -> &S {
        self.0.as_ref()
    }

    pub fn strong_count(&self) -> usize {
        Arc::strong_count(&self.0)
    }
}

impl<S> std::ops::Deref for SharedSnapshot<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

/// Build an [`EventMemo`] from an explicit ordered event sequence.
///
/// The reducer remains supplied by the caller, so the expansion is still an
/// inspectable event trace rather than a hidden test harness.
#[macro_export]
macro_rules! event_trace {
    ($initial:expr, $reduce:expr, [$($event:expr),* $(,)?]) => {{
        let memo = $crate::EventMemo::new($initial);
        $(let memo = memo.apply($event, $reduce);)*
        memo
    }};
}

pub use crate::event_trace;

/// A replayable state projection: `state = reduce(initial, events)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventMemo<E, S> {
    events: Arc<Vec<E>>,
    state: S,
}

impl<E, S> EventMemo<E, S> {
    /// Construct a memo from an initial state and an empty event log.
    pub fn new(state: S) -> Self {
        Self {
            events: Arc::new(Vec::new()),
            state,
        }
    }

    /// Construct a memo by replaying an existing event sequence.
    pub fn replay<I>(initial: S, events: I, reduce: impl Fn(&mut S, &E)) -> Self
    where
        I: IntoIterator<Item = E>,
    {
        let events: Vec<E> = events.into_iter().collect();
        let mut state = initial;
        for event in &events {
            reduce(&mut state, event);
        }
        Self {
            events: Arc::new(events),
            state,
        }
    }

    /// Apply one event and retain it for deterministic replay.
    pub fn apply(mut self, event: E, reduce: impl Fn(&mut S, &E)) -> Self
    where
        E: Clone,
    {
        reduce(&mut self.state, &event);
        Arc::make_mut(&mut self.events).push(event);
        self
    }

    pub fn state(&self) -> &S {
        &self.state
    }

    pub fn into_state(self) -> S {
        self.state
    }

    pub fn events(&self) -> &[E] {
        self.events.as_slice()
    }

    /// Transfer the current projection into a shared immutable view.
    pub fn into_shared_state(self) -> SharedSnapshot<S> {
        SharedSnapshot::new(self.state)
    }
}

#[cfg(test)]
mod tests {
    use super::{EventMemo, SharedSnapshot};
    use std::sync::Arc;

    fn add(state: &mut i32, event: &i32) {
        *state += *event;
    }

    #[test]
    fn apply_is_equivalent_to_replay() {
        let applied = crate::event_trace!(10, add, [2, 3]);
        let replayed = EventMemo::replay(10, [2, 3], add);
        assert_eq!(applied, replayed);
        assert_eq!(applied.state(), &15);
        assert_eq!(applied.events(), &[2, 3]);
    }

    #[test]
    fn memo_state_is_not_recomputed_by_readers() {
        let memo = EventMemo::new(String::new()).apply("a", |state, event| {
            state.push_str(event);
        });
        assert_eq!(memo.into_state(), "a");
    }

    #[test]
    fn shared_snapshot_clones_the_pointer_not_the_read_view() {
        let snapshot = SharedSnapshot::new(String::from("state"));
        let clone = snapshot.clone();
        assert_eq!(snapshot.get(), "state");
        assert_eq!(snapshot.strong_count(), 2);
        assert!(Arc::ptr_eq(&snapshot.0, &clone.0));
    }
}
