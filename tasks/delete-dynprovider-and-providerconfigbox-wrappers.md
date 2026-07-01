# Delete DynProvider and ProviderConfigBox wrappers

## Status

`todo`

## Context

`DynProvider` wraps `BuiltProvider` solely for backward compatibility; `ProviderConfigBox` is a cloneable wrapper around `Arc<dyn ProviderConfig>`.

## Goal

Delete both wrappers and migrate callers to the underlying types.

## Acceptance Criteria
- [ ] Remove `DynProvider` and `ProviderConfigBox` definitions.
- [ ] Update all call sites and tests.
- [ ] `cargo check --workspace` passes.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** All provider and agent tests pass.
- **Live tmux validation:** `/provider` and headless provider smoke test pass.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
