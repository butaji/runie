# Consolidate session metadata and delete SessionIndex read path

## Status

`todo`

## Context

`crates/runie-core/src/actors/session/session_handlers.rs:255-257` still loads `SessionIndex` on `/load`, even though the unified store writes metadata into each session file header. `SessionHeader` and `SessionMetadata` are also duplicate structs.

## Goal

Delete `SessionIndex` entirely; migrate any existing `sessions.json` into per-session headers once at startup; use a single `SessionMetadata` type.

## Acceptance Criteria

- [ ] Delete `crates/runie-core/src/session/index.rs`.
- [ ] Merge `SessionHeader` and `SessionMetadata` into one type.
- [ ] Remove `/load` fallback to `SessionIndex`.
- [ ] Provide one-time migration for existing `sessions.json`.
- [ ] `/resume`, search, star, and rename behavior unchanged.

## Design Impact

No change to TUI element design or composition. Only session persistence behavior changes.

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
