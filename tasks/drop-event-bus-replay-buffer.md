# Drop EventBus in-memory replay buffer

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: none
**Blocks**: drop-small-stdlib-replaceable-deps

## Description

`crates/runie-core/src/bus.rs` wraps `tokio::sync::broadcast` AND adds its own `ReplayBuffer` (`parking_lot::Mutex<VecDeque<E>>`) so late subscribers can catch up on the last N events via `subscribe_with_replay`. This duplicates the disk-based replay that `SessionActor::replay_existing_events` already performs: on startup the `SessionActor` loads durable events from the session log and publishes them to the bus, so a `UiActor` that subscribes before the `SessionActor` publishes will receive the replayed history through the normal live channel.

Reversal argument under the YAGNI / event-based posture:

- Two replay mechanisms for the same purpose (in-memory ring + disk JSONL) is one too many.
- The in-memory replay only helps non-durable events (input, scroll, animation), which a late subscriber should not need.
- Removing `ReplayBuffer` makes `EventBus` a thin `broadcast::channel` wrapper and removes the `parking_lot` use in `bus.rs` (unblocks `drop-small-stdlib-replaceable-deps`).

`event-bus-replay-semantics` (done) fixed a drain bug in the existing replay buffer; this task reopens whether the buffer should exist at all.

## Acceptance Criteria

- [ ] Decision made: EITHER
  - (a) **Drop** — `ReplayBuffer` + `ReplayReceiver` removed; `EventBus` becomes `pub struct EventBus<E>(broadcast::Sender<E>);` with `new`, `publish`, `subscribe`, `subscriber_count`; `subscribe_with_replay` removed; all callers migrated to `subscribe` (with `SessionActor` disk-replay covering startup); OR
  - (b) **Keep + document** — a concrete late-subscriber scenario that cannot be served by `SessionActor` disk-replay is written into `bus.rs` module docs.
- [ ] If (a): `rg "subscribe_with_replay\|ReplayBuffer\|ReplayReceiver" crates/` returns zero hits (except maybe a thin `ReplayReceiver` type alias over `broadcast::Receiver` if needed for call-site compatibility).
- [ ] If (a): `bus.rs` no longer imports `parking_lot`.
- [ ] Startup replay still works: `UiActor` that subscribes before `SessionActor` publishes receives the durable history.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds (including `session_actor_replays_*` tests).

## Tests

### Layer 1 — State/Logic
- [ ] `event_bus_pub_sub_round_trip` — publish N events, subscriber receives them in order (existing test stays green).
- [ ] `event_bus_subscriber_count` — `subscriber_count()` reflects active receivers.

### Layer 2 — Event Handling
- [ ] `session_actor_replays_to_subscriber` — the existing `session_actor_replays_to_uactor` test stays green after the in-memory replay buffer is gone (proves disk-replay covers the case).
- [ ] `session_actor_replays_available_to_late_subscriber` — the existing late-subscriber test stays green.
- [ ] `session_actor_replays_after_subscriber_ready` — the existing ordering test stays green.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_ui_actor_receives_history_on_startup` — a `UiActor` spawned before `SessionActor` still sees the replayed message events on its `subscribe()` receiver within 1s.

## Files touched

- `crates/runie-core/src/bus.rs` (drop `ReplayBuffer`, `ReplayReceiver`, `subscribe_with_replay`)
- `crates/runie-core/src/session_actor.rs` (already publishes durable events; confirm no `subscribe_with_replay` use)
- `crates/runie-tui/src/main.rs` / `ui_actor.rs` (replace `subscribe_with_replay` with `subscribe` if used)
- `crates/runie-core/src/actor.rs` (test helpers that use `subscribe_with_replay`)

## Notes

If any non-durable late-subscriber case genuinely needs in-memory replay (e.g. a diagnostics actor that joins after some input events), option (b) is the safe choice — document it and close as `wontfix`. The done task `event-bus-replay-semantics` fixed the drain bug; this task is the follow-up "is the feature worth keeping" decision. Run before `drop-small-stdlib-replaceable-deps` so the `parking_lot` site in `bus.rs` disappears first.
