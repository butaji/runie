# Enrich provider trait with metadata and retry config

## Status

`todo`

## Context

`Provider` trait lacks metadata, retry config, and fast-model fallback; consumers couple `DynProvider` with the global registry.

## Goal

Add `ProviderMetadata`, `RetryConfig`, and a default `complete_fast` method to the trait.

## Acceptance Criteria
- [ ] Extend trait without breaking existing providers.
- [ ] Expose model info and retry policy through trait.
- [ ] Update registry integration.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for metadata and retry config defaults.
- **Layer 2 — Event Handling:** Provider-loaded facts include metadata.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Mock and replay providers implement new methods.
- **Live tmux validation:** `/provider` shows model metadata.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
