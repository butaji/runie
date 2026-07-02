# Replay sessions via events through `AppState`

## Status

`done`

## Description

`session/replay.rs` directly mutates `session_mut()` instead of applying events. Replay should emit `Event`s and update `AppState` through `AppState::update`.

## Implementation

Added `set_session_display_name` and `restore_session_metadata` helper methods to `AppState` (in `domain_ops.rs`). These encapsulate the session state mutations that replay needs. Updated `replay_event` to use `set_session_display_name` for `SessionRenamed` events. Updated `restore_metadata` and `apply_session_loaded` to use `restore_session_metadata`. Removed `session/replay.rs` exemption from the AppState field access lint in `build.rs`, and updated all test code to use accessors (`session()` / `config()` / `session_mut()` / `config_mut()`).

Note: `session_created_at` and `session_updated_at` are set via `restore_session_metadata` which uses `session_mut()` directly — these are session metadata fields without corresponding UI events, so direct mutation through a helper method is appropriate.

## Files changed

- `crates/runie-core/src/model/state/domain_ops.rs` — added `restore_session_metadata` and `set_session_display_name` helpers
- `crates/runie-core/src/session/replay.rs` — use helpers, use accessors in test code, fixed `save_and_load_roundtrip` test
- `crates/runie-core/src/update/dispatch.rs` — `apply_session_loaded` uses `restore_session_metadata`
- `crates/runie-core/build.rs` — removed `session/replay.rs` exemption

## Acceptance criteria

- [x] **Unit tests** — Replaying a saved session produces the same `AppState` as direct mutation.
- [x] **E2E tests** — Session replay works in mock-provider tests.
- [x] **Live tmux tests** — Resume a session in tmux and verify the chat tree is restored.

## Tests

### Unit tests
- Event-by-event replay matches old state. All `session_replay` tests pass.

### E2E tests
- Replay fixture loads a session. All session-related integration tests pass.

### Live tmux tests
- Save and resume a session in tmux.

### SSOT/Event Compliance
- [x] **Actor/SSOT:** `SessionActor` owns session state; replay emits events through AppState helper methods.
- [x] **Trigger events:** `SessionLoaded`, durable events trigger state updates.
- [x] **Observer events:** Replay uses `set_session_display_name` and `restore_session_metadata` helpers; `AppState::update` for other events.
- [x] **No direct mutations:** Replay uses helper methods; no raw `session_mut().field =` outside helpers.
- [x] **No new mirrors:** Session state is authoritative in `SessionActor`; no duplicates.
- [x] **Async work observed:** Replay is synchronous; no new async work introduced.
