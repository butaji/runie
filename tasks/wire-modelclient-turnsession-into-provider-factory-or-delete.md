# Wire ModelClient/TurnSession into provider factory or delete

## Status

`todo`

## Context

`crates/runie-provider/src/model_client.rs` introduced `ModelClient`/`TurnSession` for connection reuse, but production builds still use `OpenAiProvider::from_http_client` with `BuiltProvider::cached_http_client`.

## Goal

Either integrate `ModelClient` into the provider factory or delete the dead module and re-exports.

## Acceptance Criteria
- [ ] Audit all call sites.
- [ ] Choose wire or delete.
- [ ] Remove unused code and update tests.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** All provider tests pass; `cargo machete` clean.
- **Live tmux testing session (required):** Real provider request works.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
