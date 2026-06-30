# Unify headless runtime bootstrap

**Status**: done
**Milestone**: R7
**Category**: CLI / Architecture
**Priority**: P3

**Depends on**: unify-cli-json-rpc-transport-and-remove-dead-acp
**Blocks**: none

## Description

`runie-cli/src/main.rs` uses `runie_provider::spawn_headless_runtime()`, while `runie-core/src/headless_runtime.rs` provides a parallel `HeadlessRuntime`. Consolidate on one path.

## Acceptance Criteria

- [x] Decide canonical headless runtime (recommend `runie-core::HeadlessRuntime`).
- [x] Migrate CLI to use it, or merge the two implementations.
- [x] Delete the duplicate path.
- [x] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `headless_runtime_turn_completes` — CLI headless mode completes a provider replay turn.

## Files touched

- `crates/runie-cli/src/main.rs`
- `crates/runie-core/src/headless_runtime.rs`
- `crates/runie-agent/src/headless/mod.rs`
- `crates/runie-provider/src/lib.rs`

## Notes

- Low priority; the duplication does not break functionality.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
