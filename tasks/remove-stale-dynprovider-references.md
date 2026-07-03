# Remove stale DynProvider references

## Status

`done`

## Context

`crates/runie-core/src/actors/provider/factory.rs:45` had a stale doc comment mentioning `DynProvider`.

## Changes

Updated the doc comment in `factory.rs` to remove the reference to `DynProvider`.

## Acceptance Criteria
- [x] Update `factory.rs` doc comment.
- [x] Update `Architecture.md` provider section. (No changes needed - no reference exists)
- [x] Search for any remaining `DynProvider` references. (Only found the stale doc comment which is now fixed)

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** `cargo doc` builds with no broken links.
- **Live tmux testing session (required):** N/A.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
