# Unify headless runtime bootstrap

**Status**: todo
**Milestone**: R7
**Category**: CLI / Architecture
**Priority**: P3

**Depends on**: unify-cli-json-rpc-transport-and-remove-dead-acp
**Blocks**: none

## Description

`runie-cli/src/main.rs` uses `runie_provider::spawn_headless_runtime()`, while `runie-core/src/headless_runtime.rs` provides a parallel `HeadlessRuntime`. Consolidate on one path.

## Acceptance Criteria

- [ ] Decide canonical headless runtime (recommend `runie-core::HeadlessRuntime`).
- [ ] Migrate CLI to use it, or merge the two implementations.
- [ ] Delete the duplicate path.
- [ ] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `headless_runtime_turn_completes` — CLI headless mode completes a provider replay turn.

## Files touched

- `crates/runie-cli/src/main.rs`
- `crates/runie-core/src/headless_runtime.rs`
- `crates/runie-agent/src/headless/mod.rs`
- `crates/runie-provider/src/lib.rs`

## Notes

- Low priority; the duplication does not break functionality.
