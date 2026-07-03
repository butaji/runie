# Fix `build.rs` lint scope and tests

**Status**: done
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: none
**Blocks**: capture-orphan-spawns-across-workspace

## Description

The guardrails in `crates/runie-core/build.rs` are advertised in `AGENTS.md` as enforcing AppState field access, magic numbers, and observed async work. In practice:

1. The unit tests for `needs_spawn_lint` have reversed assertions.
2. The orphan-spawn linter only scans `crates/runie-core/src`, missing violations in `runie-agent`, `runie-tui`, `runie-cli`, and `runie-provider`.
3. The linter treats `let _ = tokio::spawn(...)` as acceptable, which contradicts the SSOT ADR rule against unbounded fire-and-forget spawns.

## Acceptance Criteria

- [x] Fix the three reversed assertions in `build.rs` `#[cfg(test)]` tests.
- [x] Extend the lint to scan every crate in the workspace (or move it to a workspace-level xtask/CI step).
- [x] Remove the `let _ = tokio::spawn(...)` exception so that explicit discards are still flagged.
- [x] Fix or exempt with documented reasons every orphan spawn discovered by the widened lint.
- [x] `cargo test --workspace` passes.
- [x] `cargo check --workspace` passes with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `build_script_spawn_lint_tests_pass` — corrected assertions for `needs_spawn_lint`.
- [x] `orphan_spawn_lint_finds_cross_crate_violations` — lint reports spawns outside `runie-core`.

### Layer 2 — Event Handling
- [x] N/A — lint/static-analysis concern.

### Layer 3 — Rendering
- [x] N/A — lint/static-analysis concern.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A — lint/static-analysis concern.

### Live Tmux Testing Session
- [x] Start and exit the TUI after fixing widened lint violations to confirm clean shutdown with no leaked tasks.

## Files touched

- `crates/runie-core/build.rs`
- `crates/runie-tui/src/bootstrap.rs`
- `crates/runie-tui/src/ui_actor/mod.rs`
- `crates/runie-tui/src/ui_actor/effects.rs`
- `crates/runie-cli/src/server.rs`
- `crates/runie-core/src/shell.rs`
- `crates/runie-agent/src/subagent.rs`

## Notes

- Existing `SPAWN_EXEMPTIONS` should shrink as violations are fixed rather than expanded.
- If extending `build.rs` to all crates is impractical, promote the lint to a workspace xtask and invoke it in CI.
