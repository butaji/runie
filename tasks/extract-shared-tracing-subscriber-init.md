# Extract shared tracing subscriber init

**Status**: done
**Milestone**: R6
**Category**: Observability
**Priority**: P2

**Depends on**: initialize-tracing-subscriber-in-binaries
**Blocks**: none

## Description

`runie-tui/src/main.rs` and `runie-cli/src/main.rs` contain identical `EnvFilter` + `fmt::layer` subscriber setup. Extracted a single `runie_core::telemetry::init()` helper and called it from both binaries.

## Acceptance Criteria

- [x] Add `runie_core::telemetry::init()` that builds the subscriber.
- [x] Call it from `runie-tui/src/main.rs` and `runie-cli/src/main.rs`.
- [x] Preserve env-filter behavior and default filter.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `subscriber_init_is_idempotent` — calling init twice does not panic.

### Layer 2 — Event Handling
- [x] `telemetry_event_emitted_after_init` — a test subscriber captures an event.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `crates/runie-core/src/telemetry.rs` — shared `init()` helper with EnvFilter + fmt::layer
- `crates/runie-tui/src/main.rs` — calls `telemetry::init()` at startup
- `crates/runie-cli/src/main.rs` — calls `runie_core::telemetry::init()` at startup

## Notes

- `tracing_subscriber` is already a workspace dependency.
- The `init()` function uses `OnceLock` for idempotency.
- Default filter is "info" level, configurable via `RUST_LOG`.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
