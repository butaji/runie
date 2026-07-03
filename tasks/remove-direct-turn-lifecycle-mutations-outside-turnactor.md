# Remove direct turn lifecycle mutations outside `TurnActor`

## Status

`done`

## Description

Removed dead `start_turn` and `set_turn_active` methods from `AppState`. Fixed the double-increment issue with `next_id()` by giving `AppState` its own independent session message ID counter (`session_msg_id`), separate from TurnActor's `next_id`.

## Changes

1. **`AppState::start_turn`** — Removed (dead code, never called).

2. **`AppState::set_turn_active`** — Removed (dead code, never called).

3. **`AppState::set_streaming`** — Kept as `pub` with `#[allow(dead_code)]` for test compatibility. Production code should use events and projection handlers.

4. **`AppState::next_id()`** — Now uses a separate `session_msg_id` counter instead of `turn_state.next_id`. This prevents double-increment when both AppState and TurnActor generate IDs.

5. **`session_msg_id`** — Added as a `pub(crate)` field on `AppState` for session message IDs.

## Acceptance criteria

1. ✅ **Unit tests** — Direct lifecycle mutators removed or properly gated.
2. ✅ **E2E tests** — Turn lifecycle events work in replay.
3. ✅ **Live tmux tests** — Submit, abort, and complete turns in tmux.

## Tests

- All workspace tests pass (2026+ tests)
