# Step 06: Steering + Follow-up queue actors

**Status:** implemented; queue behavior is covered by event/replay tests
**Depends on:** 05

## Goal
Implement the two queue actors that own the steering and follow-up message queues.

## Changes
- `crates/runie-core/src/queues/steering.rs`:
  - `SteeringQueueActor` struct with `mpsc::Sender<SteeringCommand>`.
  - `SteeringCommand`: `Push(AgentMessage)`, `DrainOne`, `DrainAll`, `Clear`, `Snapshot`.
  - Worker holds `Mutex<Vec<AgentMessage>>` and a `tokio::sync::Notify` (wakes the loop).
  - `SteeringQueueSnapshot { len: usize, is_empty: bool }`.
- `crates/runie-core/src/queues/follow_up.rs`: mirror of steering for follow-up queue.
- `crates/runie-core/src/queues/mod.rs`: re-exports.
- `QueueMode` lives on the loop actor (set per `steering_mode` / `follow_up_mode`).

## Verification
- `cargo check -p runie-core` → exit 0.
- Unit test: push 3, `DrainOne` returns 1, `len` = 2, `DrainAll` returns 2, `is_empty`.
- Unit test: `Clear` empties.

## Notes
- The Notify wakes the loop's queue-poll `select!` arm. No polling in the actor itself.
- Steering messages poll **after** the current assistant turn's tools finalize and **before** the next provider request. Follow-up polls only when no more tool calls and no steering. (Loop integration is step 09.)
