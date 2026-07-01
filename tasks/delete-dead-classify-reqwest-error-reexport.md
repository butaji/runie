# Delete dead classify_reqwest_error re-export

## Status

`todo`

## Context

`crates/runie-provider/src/retry.rs:13-15` keeps `pub use crate::ProviderError as classify_reqwest_error` with `#[allow(unused_imports)]`.

## Goal

Delete the re-export and its allow attribute.

## Acceptance Criteria
- [ ] Delete the dead re-export.
- [ ] `cargo check --workspace` passes.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** `cargo check` passes.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
