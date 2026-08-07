# Step 04: Event bus + subscribe barriers

**Status:** implemented (2026-08-07)
**Depends on:** 03

## Goal
Implement the event bus with `agent_end` barrier semantics and registration-order listener dispatch.

## Changes
- `crates/runie-core/src/events/bus.rs`:
  - `EventBus` struct holding `broadcast::Sender<AgentEvent>` with `BUS_CAPACITY: usize = 1024` (named constant).
  - `EventBus::new()`, `EventBus::publish(&self, event: AgentEvent)`, `EventBus::subscribe(&self) -> broadcast::Receiver<AgentEvent>`.
- `crates/runie-core/src/events/subscribe.rs`:
  - `Subscriber` trait with `async fn handle(&mut self, event: &AgentEvent)`.
  - `SubscriberRegistry` with `Mutex<Vec<Box<dyn Subscriber>>>` (drained at barrier points).
  - `register(&self, sub) -> SubId`, `unregister(&self, id)`, `dispatch(&self, event: &AgentEvent)` (awaits each subscriber in registration order).
- `crates/runie-core/src/events/mod.rs`: re-exports.

## Verification
- `cargo check -p runie-core` → exit 0.
- Unit test (in `events/subscribe.rs`): register 5 subscribers, dispatch event, assert they execute in registration order (use `tokio::join!` and an `AtomicUsize` counter).
- Unit test: dispatch 100 events, assert no subscriber drops when capacity >= subscriber count.

## Notes
- Barrier rule from README §Events: `agent_end` listeners must complete before `waitForIdle()` returns. Implement by awaiting the dispatch future inside the loop task before resolving.
- `BUS_CAPACITY` is a magic-number-class constant; declare it with `pub const BUS_CAPACITY: usize = 1024;`.
