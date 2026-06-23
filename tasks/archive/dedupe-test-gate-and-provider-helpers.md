# Dedupe test gate/provider helpers into runie-testing

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Testing
**Priority**: P3

**Depends on**: none
**Blocks**: none

## Description

`fn allow_all_gate() -> PermissionGate` is duplicated in `runie-agent/src/headless.rs:291`, `runie-agent/src/tests/turn.rs:14`, `runie-agent/tests/minimax_turn.rs:30` — all identical `PermissionGate::new(PermissionManager::default(), Arc::new(AutoAllowSink))`. `fn mock_provider()` is duplicated in `runie-agent/src/subagent.rs:148`, `runie-agent/src/tests/turn.rs:18`, `runie-testing/src/fixtures.rs:28`. `runie-testing` already exists for exactly this.

## Acceptance Criteria

- [ ] `runie-testing` exposes `pub fn allow_all_gate()` and `pub fn mock_provider()`.
- [ ] All 3 `allow_all_gate` copies replaced with the shared import.
- [ ] All 3 `mock_provider` copies replaced with the shared import.
- [ ] `rg -c "fn allow_all_gate" crates/` returns exactly 1.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `shared_allow_all_gate_is_default_manager` — gate wraps `PermissionManager::default()`.
- [ ] `shared_mock_provider_returns_dyn` — helper returns a usable `DynProvider`.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- N/A.

## Files touched

- `crates/runie-testing/src/lib.rs` (or `fixtures.rs`)
- `crates/runie-agent/src/headless.rs`
- `crates/runie-agent/src/subagent.rs`
- `crates/runie-agent/src/tests/turn.rs`
- `crates/runie-agent/tests/minimax_turn.rs`

## Notes

`runie-testing` already ships `mock_tool_runtime` and `fixtures`; this completes the set.
