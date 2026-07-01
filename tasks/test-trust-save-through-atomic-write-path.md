# Test trust save through atomic write path

## Status

`todo`

## Context

`crates/runie-core/src/trust.rs:113-138` writes the trust file directly with `std::fs::write` instead of `TrustManager::save()`, bypassing the new atomic-write path.

## Goal

Use `tm.save()` in the test and assert `0o600` permissions on Unix.

## Acceptance Criteria
- [ ] Replace direct write with `tm.save()`.
- [ ] Assert file permissions.
- [ ] Test passes.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit test for save path and permissions.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Trust tests pass.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
