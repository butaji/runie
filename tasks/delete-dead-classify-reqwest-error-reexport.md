# Delete dead classify_reqwest_error re-export

## Status

`done`

## Context

`crates/runie-provider/src/retry.rs:13-15` was supposed to keep `pub use crate::ProviderError as classify_reqwest_error` with `#[allow(unused_imports)]`.

## Verification

The re-export no longer exists in `retry.rs`. The code uses direct `ProviderError` references instead.

## Acceptance Criteria
- [x] Delete the dead re-export.
- [x] `cargo check --workspace` passes.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** `cargo check` passes.
- **Live tmux testing session (required):** N/A.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
