# Unify three persistence actors into one `SessionActor`

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

Three actors today share ownership of durable state and all touch `SessionStore` / `SessionIndex`:

| Actor | LOC | Owns |
|-------|-----|------|
| `actors/persistence/actor.rs` | 149 | trust.json + history.jsonl |
| `actors/session_store/actor.rs` | 183 | named-session CRUD (load/save/delete/import/export/list) |
| `session_actor.rs` (NOT in `actors/`) | 355 | durable event append + replay + summary index |

They duplicate the `SessionStore` + `SessionIndex` plumbing, duplicate `SessionMetadata` construction (`actors/session_store/actor.rs:172` and `session_actor.rs:38` build the same struct), and every binary's bootstrap spawns at least two of them. This violates "actor owns state" — durable state ownership is fragmented across three actors and a non-`actors/` file.

Unify into a single `SessionActor` (in `actors/session/`) that owns: trust, input history, named-session CRUD, and durable event append/replay. One actor, one `SessionStore`, one `SessionIndex`, one `SessionMetadata` builder.

## Acceptance Criteria

- [ ] `actors/persistence/` deleted; its trust + history responsibilities moved into the unified `SessionActor`.
- [ ] `actors/session_store/` deleted; named-session CRUD moved into the unified `SessionActor`.
- [ ] `session_actor.rs` (root) deleted; durable append/replay/summary moved into `actors/session/actor.rs`.
- [ ] New `actors/session/` dir contains `actor.rs`, `messages.rs`, `mod.rs`.
- [ ] `SessionMetadata` built in exactly one place.
- [ ] TUI bootstrap spawns one `SessionActor` instead of `PersistenceActor` + `SessionStoreActor` + `SessionActor`.
- [ ] Headless runtime unaffected (it does not spawn persistence actors).
- [ ] `arch_guardrails.rs` path allow-lists updated to reflect deleted files.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `session_metadata_built_once` — single `SessionMetadata` construction path; grep assertion that `fn build_meta` (or equivalent) appears in exactly one file.
- [ ] `trust_decisions_round_trip` — `SetTrust` persists and `TrustLoaded` repopulates state.

### Layer 2 — Event Handling
- [ ] `unified_actor_handles_trust_and_history_and_sessions` — send `SetTrust`, `AppendHistory`, `Save`, `Load` messages; each emits the correct event.
- [ ] `durable_events_appended_and_replayed` — publish durable events to the bus; reload the session and verify replay.

### Layer 3 — Rendering
- N/A — persistence actors emit events; rendering is unchanged.

### Layer 4 — Smoke / Crash
- [ ] `smoke_tui_bootstrap_spawns_one_session_actor` — TUI main spawns exactly one persistence actor; verify via grep on `spawn(` calls in `crates/runie-tui/src/main.rs`.

## Files touched

- `crates/runie-core/src/actors/persistence/` → delete
- `crates/runie-core/src/actors/session_store/` → delete
- `crates/runie-core/src/session_actor.rs` → delete
- `crates/runie-core/src/actors/session/` → new (`actor.rs`, `messages.rs`, `mod.rs`)
- `crates/runie-core/src/actors/mod.rs` — update re-exports
- `crates/runie-core/src/lib.rs` — remove `pub mod session_actor;`, update re-exports
- `crates/runie-tui/src/main.rs` — collapse three spawns into one
- `crates/runie-tui/src/ui_actor.rs` — update `PersistenceActorHandle` → `SessionActorHandle`
- `crates/runie-core/tests/arch_guardrails.rs` — update legacy allow-lists

## Notes

This is the highest-leverage actor consolidation: ~400 LOC removed, 2 actor spawns dropped from every TUI bootstrap, one clear owner of all durable state. The `SessionActor` message enum should be a flat union of the three current message sets. Keep `SessionStore` and `SessionIndex` as pure data-access types (no actor trait) — the actor wraps them. Related: `inline-thin-abstractions` notes that the current `SessionActor` has `type Msg = ()` and is really a bus subscriber; after unification it receives real messages so the `Actor` trait is justified.
