# Create Grok fixture loader and normalizer

## Status

`todo`

## Context

No Grok Build fixture module exists; captured Grok output contains non-deterministic timestamps, IDs, and temp paths.

## Goal

Add `runie-testing/src/fixtures/grok_build.rs` with `include_dir!`, fixture lookup, and deterministic sanitization of IDs/paths/timestamps.

## Acceptance Criteria
- [ ] Create fixture directory and loader.
- [ ] Implement sanitizer with stable replacements.
- [ ] Register module in `runie-testing`.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for sanitizer replacements.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Loader returns normalized fixture text.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
