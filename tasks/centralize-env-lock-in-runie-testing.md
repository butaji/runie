# Centralize ENV_LOCK in runie-testing

## Status

`todo`

## Context

Several test modules define their own `static ENV_LOCK: Mutex<()>` to serialize env-var mutation:
- `crates/runie-core/src/tests/support.rs:17`
- `crates/runie-core/src/tests/copy.rs:12`
- `crates/runie-provider/src/tests.rs:20`
- `crates/runie-provider/src/config/tests.rs:6`
- `crates/runie-tui/src/tests/render/render_slash.rs:4`

## Goal

Move the lock and a helper like `env_lock()` into `runie-testing` and have all crates import it from one place.

## Acceptance Criteria

- [ ] Add `pub static ENV_LOCK` and `env_lock()` helper to `runie-testing`.
- [ ] Replace the duplicate statics in all crates.
- [ ] All tests still pass.

## Design Impact

No change to TUI element design or composition. Only test infrastructure changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** All tests pass under `cargo test --workspace`.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
