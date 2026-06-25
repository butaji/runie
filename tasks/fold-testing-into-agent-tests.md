# Fold runie-testing into runie-agent tests

**Status**: wontfix  
*Task description was inaccurate: `runie-testing` has 4 consumers (runie-agent, runie-tui, runie-agent/tests, runie-provider/tests), not 1. Per the task's own notes, "if a second consumer is planned, keep the crate."*
**Milestone**: R4
**Category**: Configuration
**Priority**: P3

**Depends on**: none
**Blocks**: none

## Description

`runie-testing` (344 LOC, 5 files: `events.rs`, `fixtures.rs`, `lib.rs`, `macros.rs`, `mock_tool_runtime.rs`, `runner.rs`) is a workspace crate with exactly 1 external consumer: `crates/runie-agent/src/tests/tool_runtime.rs`. The `AGENTS.md` Layer 4 strategy references it as the home for "Provider Replay / Mock-Tool E2E" helpers, but in practice only one test file imports it.

A separate crate for one consumer is YAGNI. The mock tool runtime, fixture replay, and runner helpers are test-only utilities that fit naturally as `#[cfg(test)]` modules inside `runie-agent` (where the agent turn loop they support already lives). Folding it in removes a workspace member, a `Cargo.toml`, and a dependency edge.

## Acceptance Criteria

- [ ] `crates/runie-testing/` deleted from workspace.
- [ ] `runie-testing` removed from `Cargo.toml` `[workspace]` members.
- [ ] `runie-testing` removed from any `[dev-dependencies]` (verify `runie-agent/Cargo.toml`).
- [ ] Test helpers moved to `crates/runie-agent/src/tests/` (or `crates/runie-agent/tests/`) as `#[cfg(test)]` modules: `mock_tool_runtime.rs`, `fixtures.rs`, `runner.rs`, `events.rs`, `macros.rs`.
- [ ] The 1 existing consumer (`runie-agent/src/tests/tool_runtime.rs`) imports from the new location.
- [ ] Layer 4 tests (`minimax_m3_multi_tool_turn` etc.) still pass.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- N/A — test infrastructure move.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_layer4_tests_pass_after_fold` — the existing Layer 4 replay tests (`minimax_m3_multi_tool_turn`, etc.) pass with helpers imported from `runie-agent::tests::`.
- [ ] `smoke_workspace_has_no_runie_testing_crate` — `rg "runie-testing|runie_testing" Cargo.toml crates/*/Cargo.toml` returns zero hits.

## Files touched

- `crates/runie-testing/` → delete (5 files + Cargo.toml)
- `crates/runie-agent/src/tests/` (or `crates/runie-agent/tests/`) — add folded modules
- `crates/runie-agent/src/tests/tool_runtime.rs` — update imports
- `crates/runie-agent/Cargo.toml` — remove `runie-testing` dev-dep
- `Cargo.toml` (root) — remove from workspace members
- `AGENTS.md` — update Layer 4 section to point at new helper location

## Notes

Low priority: the crate is harmless but adds workspace churn. Only do this if Layer 4 test volume is not expected to grow into a shared cross-crate resource. If `runie-tui` or `runie-server` later need replay fixtures, keep the crate and instead make `runie-agent` import it like everyone else. Re-examine before executing: if a second consumer is planned, cancel this task. The `mock_tool_runtime.rs` is the most valuable asset — ensure it moves cleanly.
