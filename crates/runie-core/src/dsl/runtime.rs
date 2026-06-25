//! Runtime implementations for the declarative actor DSL.
//!
//! ## Architecture
//!
//! The DSL is runtime-agnostic: `Flow::run(rt, input)` calls methods on the
//! `Runtime` trait. Two concrete implementations are provided:
//!
//! - **`TestRuntime`** — records intents, facts, and notifications in memory.
//!   Used in unit tests to verify flows produce the expected side-effects.
//!
//! - **`RealRuntime`** — sends intents to actors via `ActorHandles`, broadcasts
//!   facts to the `EventBus`, and shows notifications via `NotificationActor`.
//!   This is the production runtime used by command handlers and input events.
//!
//! ## Adding new runtime methods
//!
//! When a new combinator needs a new runtime capability (e.g. clipboard),
//! add the method to `Runtime` with a default no-op implementation so all
//! existing runtimes continue to compile.

use crate::dsl::fact::Fact;
use crate::dsl::flow::Flow;
use crate::event::intent::Intent;
use crate::event::TransientLevel;

// ── Runtime trait ──────────────────────────────────────────────────────────────

/// Abstract runtime for the DSL.
///
/// All production flows execute against a `RealRuntime` (backed by
/// `ActorHandles` + `EventBus`). Unit tests use `TestRuntime`.
///
/// Note: `read_state` is **not** on this trait to keep it dyn-safe.
/// Read actual state inside handlers or use `.select()` at the call site.
pub trait Runtime {
    /// Send an intent to its owning actor.
    fn send_intent(&mut self, intent: Intent);

    /// Broadcast a fact to all subscribers.
    fn broadcast_fact(&mut self, fact: Fact);

    /// Show a transient notification.
    fn notify(&mut self, content: &str, level: TransientLevel);
}

// ── TestRuntime ───────────────────────────────────────────────────────────────

/// In-memory runtime that records DSL side-effects for assertions.
///
/// ```ignore
/// let mut rt = TestRuntime::new();
/// on(()).intent(Intent::Quit).run(&mut rt, ());
/// assert!(rt.intents().any(|i| matches!(i, Intent::Quit)));
/// ```
#[derive(Debug, Default)]
pub struct TestRuntime {
    intents: Vec<Intent>,
    facts: Vec<Fact>,
    notifications: Vec<(String, TransientLevel)>,
}

impl TestRuntime {
    /// Create a fresh empty test runtime.
    pub fn new() -> Self {
        Self::default()
    }

    /// Recorded intents in order.
    pub fn intents(&self) -> impl Iterator<Item = &Intent> {
        self.intents.iter()
    }

    /// Recorded facts in order.
    pub fn facts(&self) -> impl Iterator<Item = &Fact> {
        self.facts.iter()
    }

    /// Recorded notifications in order, as `(content, level)` pairs.
    pub fn notifications(&self) -> &[(String, TransientLevel)] {
        &self.notifications
    }

    /// Assert that exactly `expected` intents were recorded, in order.
    pub fn assert_intents(&self, expected: &[Intent]) {
        assert_eq!(
            self.intents.len(),
            expected.len(),
            "expected {} intents, got {:?}",
            expected.len(),
            self.intents
        );
        for (got, exp) in self.intents.iter().zip(expected.iter()) {
            assert_eq!(format!("{got:?}"), format!("{exp:?}"), "intent mismatch");
        }
    }

    /// Assert no intents were recorded.
    pub fn assert_no_intents(&self) {
        assert!(
            self.intents.is_empty(),
            "expected no intents, got {:?}",
            self.intents
        );
    }

    /// Assert no notifications were recorded.
    pub fn assert_no_notifications(&self) {
        assert!(
            self.notifications.is_empty(),
            "expected no notifications, got {:?}",
            self.notifications
        );
    }

    /// Clear all recorded side-effects (useful for reusable test fixtures).
    pub fn reset(&mut self) {
        self.intents.clear();
        self.facts.clear();
        self.notifications.clear();
    }
}

impl Runtime for TestRuntime {
    fn send_intent(&mut self, intent: Intent) {
        self.intents.push(intent);
    }

    fn broadcast_fact(&mut self, fact: Fact) {
        self.facts.push(fact);
    }

    fn notify(&mut self, content: &str, level: TransientLevel) {
        self.notifications.push((content.to_string(), level));
    }
}

// ── RealRuntime ───────────────────────────────────────────────────────────────

/// Route an intent to its owning actor via ActorHandles.
///
/// Intents are fire-and-forget: we spawn the async calls so they run
/// in the background without blocking the sync DSL runtime.
fn route_intent(handles: &Option<crate::actors::ActorHandles>, intent: &Intent) {
    let Some(h) = handles else { return };
    match intent {
        Intent::SetTheme { .. } | Intent::ReloadConfig => {
            if let Some(ref c) = h.config {
                let h = c.clone();
                tokio::spawn(async move { h.reload().await });
            }
        }
        Intent::SetTrust { path, decision } => {
            let h = h.clone();
            let path = path.clone();
            let decision = *decision;
            tokio::spawn(async move { h.send_set_trust(path, decision).await });
        }
        Intent::AppendHistory { entry } => {
            let h = h.clone();
            let entry = entry.clone();
            tokio::spawn(async move { h.send_append_history(entry).await });
        }
        Intent::RunBash { command } => {
            let h = h.clone();
            let command = command.clone();
            tokio::spawn(async move { h.run_bash(command).await });
        }
        Intent::WriteFiles { edits } => {
            let h = h.clone();
            let edits = edits.clone();
            tokio::spawn(async move { h.write_files(edits).await });
        }
        // TODO(r5): map remaining Intent variants to their owning actors
        _ => {}
    }
}

/// Production runtime backed by the actor system.
///
/// Sends intents to actors via `ActorHandles`, broadcasts facts to the
/// `EventBus`, and dispatches notifications to `NotificationActor`.
pub struct RealRuntime {
    /// Actor handles — `None` when actors are not yet spawned (e.g. headless).
    actor_handles: Option<crate::actors::ActorHandles>,
}

impl RealRuntime {
    /// Create a new real runtime with the given actor handles.
    pub fn new(actor_handles: Option<crate::actors::ActorHandles>) -> Self {
        Self { actor_handles }
    }

    /// Execute a flow immediately using the current actor handles.
    ///
    /// This is the main entry point for command handlers and input events.
    /// ```ignore
    /// RealRuntime::current().run(flow, ());
    /// ```
    pub fn run<T>(&mut self, flow: Flow<T>, input: T) {
        flow.run(self, input);
    }
}

impl Runtime for RealRuntime {
    fn send_intent(&mut self, intent: Intent) {
        route_intent(&self.actor_handles, &intent);
    }

    fn broadcast_fact(&mut self, _fact: Fact) {
        // Facts are broadcast via the EventBus by the emitting actor.
        // RealRuntime delegates to the bus, which is injected at construction.
        // TODO(r5): inject EventBus here.
    }

    fn notify(&mut self, content: &str, _level: TransientLevel) {
        // Delegate to NotificationActor via actor handles.
        // TODO(r5): implement when NotificationActor is built.
        let _ = content;
    }
}

/// Global static for the current RealRuntime.
///
/// Initialized once during app bootstrap and set to `Some`.
/// Code that runs before the runtime is spawned (e.g. config loading)
/// sees `None` and falls back to no-op.
thread_local! {
    static CURRENT_RUNTIME: std::cell::RefCell<Option<RealRuntime>> = std::cell::RefCell::new(None);
}

/// Get the current runtime, if any.
pub fn with_runtime<F, R>(f: F) -> R
where
    F: FnOnce(&mut dyn Runtime) -> R,
{
    CURRENT_RUNTIME.with(|cell| {
        let mut borrowed = cell.borrow_mut();
        if let Some(ref mut rt) = *borrowed {
            f(rt)
        } else {
            // No runtime set — use NoOp so the flow still runs without panicking.
            struct NoOp;
            impl Runtime for NoOp {
                fn send_intent(&mut self, _: Intent) {}
                fn broadcast_fact(&mut self, _: Fact) {}
                fn notify(&mut self, _: &str, _: TransientLevel) {}
            }
            let mut noop = NoOp;
            f(&mut noop)
        }
    })
}

/// Set the current runtime (called once during app bootstrap).
pub fn set_runtime(rt: RealRuntime) {
    CURRENT_RUNTIME.with(|cell| {
        *cell.borrow_mut() = Some(rt);
    });
}

/// Run a flow using the current thread-local runtime.
pub fn run_flow<T>(flow: Flow<T>, input: T) {
    with_runtime(|rt| flow.run(rt, input))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::fact::Fact;
    use crate::dsl::flow::Flow;
    use crate::dsl::on;
    use crate::event::intent::Intent;

    // ── Layer 1: TestRuntime records intents and facts ───────────────────────

    #[test]
    fn test_runtime_records_intents() {
        let mut rt = TestRuntime::new();
        let flow: Flow<()> = on(())
            .intent(Intent::Quit)
            .then(on(()).intent(Intent::Abort));
        flow.run(&mut rt, ());

        let intents: Vec<_> = rt.intents().cloned().collect();
        assert_eq!(intents.len(), 2);
        assert!(matches!(intents[0], Intent::Quit));
        assert!(matches!(intents[1], Intent::Abort));
    }

    #[test]
    fn test_runtime_records_facts() {
        let mut rt = TestRuntime::new();
        let flow: Flow<()> = on(())
            .fact(Fact::SessionChanged)
            .fact(Fact::ViewInvalidated);
        flow.run(&mut rt, ());

        assert_eq!(rt.facts().count(), 2);
    }

    #[test]
    fn test_runtime_records_notifications() {
        let mut rt = TestRuntime::new();
        let flow: Flow<()> = on(())
            .notify("hello")
            .notify_level("error!", TransientLevel::Error);
        flow.run(&mut rt, ());

        let notes = rt.notifications();
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].0, "hello");
        assert_eq!(notes[1].1, TransientLevel::Error);
    }

    #[test]
    fn test_runtime_reset_clears_all() {
        let mut rt = TestRuntime::new();
        on(()).intent(Intent::Quit).run(&mut rt, ());
        rt.reset();
        rt.assert_no_intents();
        assert!(rt.facts().count() == 0);
        rt.assert_no_notifications();
    }

    #[test]
    fn test_runtime_assert_intents_passes() {
        let mut rt = TestRuntime::new();
        on(()).intent(Intent::Quit).run(&mut rt, ());
        rt.assert_intents(&[Intent::Quit]);
    }

    #[test]
    fn test_runtime_assert_intents_fails_on_mismatch() {
        let mut rt = TestRuntime::new();
        on(()).intent(Intent::Quit).run(&mut rt, ());
        let result = std::panic::catch_unwind(|| {
            rt.assert_intents(&[Intent::Abort]);
        });
        assert!(result.is_err(), "assert_intents should panic on mismatch");
    }

    #[test]
    fn test_runtime_assert_no_intents_passes() {
        let mut rt = TestRuntime::new();
        on(()).notify("hi").run(&mut rt, ());
        rt.assert_no_intents();
    }

    #[test]
    fn test_runtime_no_intents_fails_when_intents_exist() {
        let mut rt = TestRuntime::new();
        on(()).intent(Intent::Quit).run(&mut rt, ());
        let result = std::panic::catch_unwind(|| rt.assert_no_intents());
        assert!(
            result.is_err(),
            "assert_no_intents should panic when intents exist"
        );
    }

    // ── Layer 2: .then() chains flows ───────────────────────────────────────

    #[test]
    fn then_chains_intents_and_notifications() {
        let mut rt = TestRuntime::new();
        let flow = on(())
            .intent(Intent::SetTheme {
                name: "dark".into(),
            })
            .then(on(()).notify("done!"));
        flow.run(&mut rt, ());

        assert!(rt
            .intents()
            .any(|i| matches!(i, Intent::SetTheme { name } if name == "dark")));
        assert!(rt.notifications().iter().any(|(n, _)| n == "done!"));
    }

    #[test]
    fn then_produces_correct_count() {
        let mut rt = TestRuntime::new();
        let a = on(()).intent(Intent::ReloadConfig);
        let b = on(()).intent(Intent::Quit);
        let combined = a.then(b);
        combined.run(&mut rt, ());
        assert_eq!(rt.intents().count(), 2);
    }

    // ── Layer 2: .notify_level() records level ───────────────────────────────

    #[test]
    fn notify_level_records_error() {
        let mut rt = TestRuntime::new();
        on(())
            .notify_level("failed", TransientLevel::Error)
            .run(&mut rt, ());
        assert!(rt
            .notifications()
            .iter()
            .any(|(n, l)| n == "failed" && *l == TransientLevel::Error));
    }

    // ── Layer 2: empty flow does nothing ─────────────────────────────────────

    #[test]
    fn empty_flow_does_nothing() {
        let mut rt = TestRuntime::new();
        let flow: Flow<()> = on(());
        flow.run(&mut rt, ());
        rt.assert_no_intents();
        rt.assert_no_notifications();
    }

    // ── Layer 2: with_runtime falls back to NoOp ────────────────────────────

    #[test]
    fn with_runtime_noop_does_not_panic() {
        // Without setting a runtime, with_runtime falls back to NoOp.
        // This tests that the fallback path works without panicking.
        let result = std::panic::catch_unwind(|| {
            run_flow(on(()).intent(Intent::Quit), ());
        });
        assert!(result.is_ok(), "with_runtime fallback should not panic");
    }
}
