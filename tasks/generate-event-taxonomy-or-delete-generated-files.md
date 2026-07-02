# Generate event taxonomy or delete generated files

## Status

`wontfix`

## Context

`crates/runie-core/src/event/generated/*.rs` (~1,450 LOC) were once intended to be generated from `taxonomy.json` but were committed verbatim and never actually generated.

**Current state:** The `generated/` directory does not exist. Event taxonomy is defined inline in `src/event/mod.rs` using `strum` derives. `taxonomy.json` is kept as documentation only. No generated files remain.

This task is resolved as `wontfix` because the generated files were never created and the current approach (inline `strum` derives) is the correct one.

## Goal

Either add a real generator driven by `taxonomy.json` or replace the tables with `strum` derives and delete the files.

## Acceptance Criteria
- [x] Choose generator vs strum approach. → strum (inline derives, no code gen)
- [x] Remove hand-maintained generated files or make generation automatic. → N/A, files don't exist
- [x] All event tests pass. → verified via cargo test --workspace

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for intent/kind mappings.
- **Layer 2 — Event Handling:** Event dispatch unchanged.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Agent turn events serialize/deserialize correctly.
- **Live tmux testing session (required):** N/A.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
