# Delete dead inspector module

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-agent/src/inspector.rs` (315 LOC) defines `Inspector` trait, `ToolPipeline`, and 3 impls (`CallCounter`, `LatencyTracker`, `AfterCallSpy`). The only non-test reference is `lib.rs:6: pub mod inspector;`. `run_agent_turn` does not use `ToolPipeline` (it uses `ToolRuntime`). Additionally `impl Inspector for LatencyTracker {}` (inspector.rs:162) is an empty impl — `after_call` is never overridden, so `LatencyTracker::total()` always returns `Duration::ZERO`. Broken even within the dead module.

## Acceptance Criteria

- [ ] `crates/runie-agent/src/inspector.rs` deleted.
- [ ] `pub mod inspector;` removed from `crates/runie-agent/src/lib.rs`.
- [ ] `rg "Inspector|ToolPipeline|CallCounter|LatencyTracker|AfterCallSpy" crates/` returns zero hits outside `tasks/`.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- N/A — module deletion.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_agent_turn_without_inspector` — a turn still runs end-to-end after deletion.

## Files touched

- `crates/runie-agent/src/inspector.rs`
- `crates/runie-agent/src/lib.rs`

## Notes

If tool-call instrumentation is wanted later, build it as a decorator around `ToolRuntime` with real wiring, not an unused parallel trait.
