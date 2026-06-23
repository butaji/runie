# Inline thin abstractions

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P3

**Depends on**: none
**Blocks**: none

## Description

Three thin abstractions add indirection without value:
- `SessionActor` (formerly `crates/runie-core/src/session_actor.rs`, line 71 at the time of writing) has `type Msg = ()` — never receives messages; `_rx: mpsc::Receiver<()>` is unused. It's purely an event-bus subscriber loop; the `Actor` trait machinery (channel + `spawn_actor` returning a sender never sent to) is overkill. A plain `tokio::spawn` of a subscriber loop would do. (Other actors do receive real messages, so the `Actor` trait itself stays.) File moved to `crates/runie-core/src/session/actor.rs` during the actor trait refactor; verify with `rg 'type Msg' crates/runie-core/src/` before editing.
- `NotificationExt` trait (`crates/runie-core/src/notification.rs:66`) — only impl is `for ()`, always invoked as `<() as NotificationExt>::success(msg)`. Never parameterized over another type. Could be plain free functions.
- `tool/format.rs:27` `resolve_path` is a 1-line forwarder to `path::resolve_path_in`; 7 engine tool files import the wrapper. Switch them to the canonical `runie_core::path::resolve_path_in`.

## Acceptance Criteria

- [ ] `SessionActor` rewritten as a plain `tokio::spawn` subscriber loop (no `Actor` impl, no unit channel), OR a note added explaining why it must be an actor.
- [ ] `NotificationExt` trait removed; `success`/`error`/`info` become free functions in `notification.rs`.
- [ ] 7 engine tool imports switched to `runie_core::path::resolve_path_in`; `tool/format.rs:resolve_path` wrapper deleted.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `notification_free_functions_emit` — free fn `success(msg)` produces the same event as before.
- [ ] `resolve_path_in_used_directly` — grep assertion: no `tool::resolve_path` wrapper remains.

### Layer 2 — Event Handling
- [ ] `session_subscriber_loop_still_receives_events` — the spawned subscriber still observes bus events.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- N/A.

## Files touched

- `crates/runie-core/src/session_actor.rs` (moved to `crates/runie-core/src/session/actor.rs` during the actor trait refactor)
- `crates/runie-core/src/notification.rs`
- `crates/runie-core/src/tool/format.rs`
- `crates/runie-core/src/path.rs`
- 7 engine tool files (`find.rs`, `write_file.rs`, `read_file.rs`, `search/core.rs`, `grep.rs`, `edit_file.rs`, `list_dir.rs`)

## Notes

Keep the `Actor` trait for the 5 actors that receive real messages (`AgentActor`, `IoActor`, `PersistenceActor`, `SessionStoreActor`, `FffIndexerActor`). Only `SessionActor` is a unit-message subscriber.
