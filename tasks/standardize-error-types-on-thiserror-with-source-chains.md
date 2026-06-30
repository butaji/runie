# Standardize error types on thiserror with source chains

## Status

`todo`

## Context

Runie has eight overlapping error types (`RunieError`, `ProviderError`, `MissingApiKeyError`, `ModelError`, `SanitizeError`, `ToolParseError`, `SubagentError`, `TimeoutError`, `proto::Error`). Several flatten underlying errors via `e.to_string()`, losing `#[source]` chains.

## Goal

Standardize on `thiserror` everywhere, use `#[source]`/`#[from]` to preserve chains, and keep `anyhow` only at binary boundaries. Unify `ModelError` and `ProviderError`.

## Acceptance Criteria

- [ ] Convert error enums to `thiserror` with `#[source]`.
- [ ] Delete `RunieErrorKind` or derive it from `thiserror` discriminants.
- [ ] Preserve programmatic matching where needed.
- [ ] All tests pass.

## Design Impact

No change to TUI element design or composition. Only error reporting/debugging behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for error source chains and `Display`.
- **Layer 2 — Event Handling:** Error events carry the same user-facing messages.
- **Layer 3 — Rendering:** Error dialog text unchanged.
- **Layer 4 — E2E:** Provider replay error fixtures pass.
- **Live tmux validation:** Trigger an error and verify the message is still clear.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
