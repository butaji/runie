# Delete leftover `telemetry.rs` stub

**Status**: done
**Milestone**: R6
**Category**: Observability
**Priority**: P3

**Depends on**: replace-custom-telemetry-with-tracing-layer
**Blocks**: none

## Description

The task title assumed `telemetry.rs` was a dead stub after migrating to `tracing`. In reality, `telemetry.rs` is the **active implementation**: it exposes `init()` which sets up the tracing subscriber (EnvFilter + formatted layer + thread IDs), and both `runie-cli` and `runie-tui` call `telemetry::init()` at startup.

There is nothing to delete — the module is correct as-is.

## Acceptance Criteria

- [x] Move the two tests to an appropriate `config` test module. — **N/A**: the file contains a real `init()` function, not two stub tests. The stale description has been corrected.
- [x] Delete `crates/runie-core/src/telemetry.rs`. — **N/A**: the module is required by `runie-cli` and `runie-tui` binaries.
- [x] Remove `pub mod telemetry` from `crates/runie-core/src/lib.rs`. — **N/A**: the module is required.
- [x] `cargo test --workspace` succeeds after the change. — Already verified.
- [x] `cargo check --workspace` succeeds with no new warnings. — Already verified.

## Tests

### Layer 1 — State/Logic
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- None — the file is active and required.

## Notes

- `telemetry::init()` is called by `crates/runie-cli/src/main.rs` and `crates/runie-tui/src/main.rs`.
- The module provides idempotent tracing subscriber setup with `RUST_LOG` env filter, formatted layer, and thread IDs.
- If a future refactor moves `init()` into the binary entry points directly, this task would become relevant again.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
