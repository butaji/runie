# Consolidate session metadata and delete SessionIndex read path

## Status

`todo`

## Context

`crates/runie-core/src/session/index.rs` defines `SessionIndex` and `SessionMetadata`. `crates/runie-core/src/session/persistence/header.rs` defines `SessionHeader` as an alias: `pub use crate::session::index::SessionMetadata as SessionHeader`. `SessionIndex` is not used in the runtime path (session loading uses per-file headers), but the struct and its tests remain in `index.rs`.

## Goal

Delete `SessionIndex` and `crates/runie-core/src/session/index.rs`; keep only `SessionMetadata` (aliased as `SessionHeader`); update `lib.rs` re-exports.

**Design impact:** No change to TUI element design or composition. Only session persistence behavior changes.

## Acceptance Criteria

- [ ] Delete `crates/runie-core/src/session/index.rs`.
- [ ] Merge `SessionHeader` and `SessionMetadata` into one type (or keep `SessionMetadata` as the canonical name).
- [ ] Remove `/load` fallback to `SessionIndex`.
- [ ] Provide one-time migration for existing `sessions.json`.
- [ ] `/resume`, search, star, and rename behavior unchanged.

## Tests

- **Layer 1 — State/Logic:** Unit tests for header migration and metadata round-trip.
- **Layer 2 — Event Handling:** `SessionLoaded`/`SessionListUpdated` facts unchanged.
- **Layer 3 — Rendering:** `/sessions` popup snapshots match.
- **Layer 4 — E2E:** Headless CLI `/load` and `/sessions` work.
- **Live tmux validation:** Create, star, rename, and resume sessions in the TUI.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
