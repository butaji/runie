# Finish derive_builder for ModelInfo and provider configs

## Status

`todo`

## Context

`crates/runie-core/src/model_catalog/mod.rs:10-19` only applied `derive_builder` to `ModelCapabilities`; `ModelInfo` and provider config structs still use hand-written builder methods.

## Goal

Apply `derive_builder` to `ModelInfo` and provider/model config structs; update call sites.

## Acceptance Criteria
- [ ] Add derives to target structs.
- [ ] Update callers and tests.
- [ ] `cargo check --workspace` passes.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for builder defaults.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Model/provider tests pass.
- **Live tmux validation:** `/model` and `/provider` work.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
