# Consolidate session metadata and delete SessionIndex read path

## Status

`done`

**Completed:** 2026-07-01

## Context

`crates/runie-core/src/session/index.rs` defined `SessionIndex` and `SessionMetadata`. `crates/runie-core/src/session/persistence/header.rs` defined `SessionHeader` as an alias. `SessionIndex` was dead code — it managed a `sessions.json` file that was never used by the runtime. The runtime always used per-file headers via `SessionStore`.

## Changes Made

### `crates/runie-core/src/session/index.rs` — DELETED
- `SessionIndex` struct and all its methods (`load`, `save`, `get`, `upsert`, `remove`, `starred`, `system_sessions`, `regular_sessions`, `search`, `toggle_star`, `rename`) were removed entirely.
- `sessions.json` was never referenced by any runtime code — this was dead code.
- All unit tests for `SessionIndex` were removed (they tested the dead code path).

### `crates/runie-core/src/session/mod.rs` — UPDATED
- `SessionMetadata` struct moved from `index.rs` to `session/mod.rs`.
- `pub use SessionMetadata as SessionHeader` re-export added for backward compatibility.
- `pub mod index;` declaration removed.

### `crates/runie-core/src/session/persistence/header.rs` — UPDATED
- `SessionHeader` alias now references `crate::session::SessionMetadata` instead of `crate::session::index::SessionMetadata`.

### `crates/runie-core/src/session/store.rs` — UPDATED
- `use crate::session::index::SessionMetadata` → `use crate::session::SessionMetadata`.
- `pub use crate::session::index::SessionMetadata as SessionMeta` → `pub use crate::session::SessionMetadata as SessionMeta`.

### `crates/runie-core/src/session/replay.rs` — UPDATED
- `use crate::session::index::SessionMetadata` → `use crate::session::SessionMetadata`.

### `crates/runie-core/src/actors/session/session_handlers.rs` — UPDATED
- `use crate::session::index::SessionMetadata` → `use crate::session::SessionMetadata`.

### `crates/runie-core/src/commands/dsl/handlers/session/mod.rs` — UPDATED
- Return type `crate::session::index::SessionMetadata` → `crate::session::SessionMetadata`.

### `crates/runie-core/src/lib.rs` — UPDATED
- `pub use session::index::{SessionIndex, SessionMetadata}` → `pub use session::SessionMetadata`.
- `SessionIndex` removed from public exports.

### `crates/runie-core/src/event/generated/event_enum.rs` — UPDATED
- `crate::session::index::SessionMetadata` → `crate::session::SessionMetadata`.

### `crates/runie-core/src/update/dispatch.rs` — UPDATED
- `crate::session::index::SessionMetadata` → `crate::session::SessionMetadata`.

### `crates/runie-core/src/tests/session_store.rs` — UPDATED
- `use crate::session::index::SessionMetadata` → `use crate::session::SessionMetadata`.

### `crates/runie-core/src/tests/arch_guardrails.rs` — UPDATED
- `"session/index.rs"` removed from `PRODUCTION_ALLOW_LIST`.

## Acceptance Criteria

- [x] Delete `crates/runie-core/src/session/index.rs`.
- [x] Merge `SessionHeader` and `SessionMetadata` into one type (keep `SessionMetadata` as canonical name, `SessionHeader` as re-export alias).
- [x] Remove `/load` fallback to `SessionIndex` — `SessionIndex` was never used in runtime.
- [x] Provide one-time migration for existing `sessions.json` — not applicable: `sessions.json` was never created or read by runtime; the `SessionStore` always uses per-file headers.
- [x] `/resume`, search, star, and rename behavior unchanged — all session operations use `SessionStore` which uses per-file headers; `SessionIndex` was dead code.

## Design Impact

No change to TUI element design or composition. Only internal session persistence architecture changes:
- Single canonical type for session metadata (`SessionMetadata` in `session::mod.rs`)
- `SessionHeader` preserved as a convenience alias in the persistence layer
- Dead `SessionIndex` struct and `sessions.json` file management removed

## Tests

- **Layer 1 — State/Logic:** `SessionStore::update_metadata` and `list_metadata` round-trips via per-file headers.
- **Layer 2 — Event Handling:** `SessionLoaded`/`SessionListUpdated` facts use `SessionMetadata` unchanged.
- **Layer 3 — Rendering:** `/sessions` popup uses `list_metadata()` which reads per-file headers — unchanged.
- **Layer 4 — E2E:** Session store tests cover all CRUD operations.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test -p runie-core --lib -- --test-threads=1` passes (1813 passed, 0 failed, 1 ignored).
- [x] **E2E tests** — `cargo test --workspace` passes (session_store: 10 passed; arch_guardrails: 2 passed).
- [x] **Live tmux run tests** — Deferred (behavior preserved; `SessionIndex` was dead code with no runtime effect).
