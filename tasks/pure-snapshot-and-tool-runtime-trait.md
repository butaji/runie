# Make snapshot pure and inject ToolRuntime

**Status**: done  
**Milestone**: R4  
**Category**: TUI / Rendering  
**Priority**: P0  

**Depends on**: simplify-event-vocabulary  
**Blocks**: consolidate-binary-setup  

## Description

Rendering is currently entangled with cache maintenance: `snapshot()` calls `ensure_fresh(&mut self)`. Tool execution is hardcoded to `runie_engine::tool::builtin_registry()`. This task separates cache invalidation from snapshot building, makes rendering a pure function of `Snapshot`, and introduces an injectable `ToolRuntime` trait so agent turns can run in tests without real tools.

## Acceptance Criteria

- [x] (4a) `ensure_fresh(&mut self)` is called by `UiActor` after applying events; `snapshot(&self) -> Snapshot` is pure.
- [x] (4b) `ToolRuntime` trait is defined in `runie-core` and injected into `run_agent_turn` and `run_headless_turn`.
- [x] (4b) Production uses an adapter over `runie_engine::tool::builtin_registry()`; tests use `MockToolRuntime`.
- [x] (4b) Unused `ToolPipeline`/`Inspector` abstraction is deleted or unified with `execute_tool_call`.
- [x] (4a) `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `snapshot_does_not_mutate_state` — calling `snapshot` twice produces the same result.
- [x] `mock_tool_runtime_returns_expected_output` — mock executor returns configured results.

### Layer 3 — Rendering
- [x] `draw_snapshot_renders_from_snapshot_only` — `draw_snapshot` takes only `Frame` and `Snapshot`.
- [x] `pure_view_helper_renders_without_mutation` — replacement for `view(&mut AppState)` is pure.

### Layer 4 — Smoke / Crash
- [x] `minimax_m3_multi_tool_turn_with_mock_runtime` — full turn runs with replay provider and mock tool runtime.

## Files touched

- `crates/runie-core/src/model/cache.rs`
- `crates/runie-core/src/model/state/app_state.rs`
- `crates/runie-tui/src/ui.rs`
- `crates/runie-tui/src/ui_actor.rs`
- `crates/runie-agent/src/turn.rs`
- `crates/runie-agent/src/headless.rs`
- `crates/runie-agent/src/inspector.rs` (deleted)
- `crates/runie-agent/src/actor.rs`
- `crates/runie-agent/src/subagent.rs`
- `crates/runie-agent/src/tool_runner.rs`
- `crates/runie-agent/src/lib.rs`
- `crates/runie-engine/src/tool/runtime_adapter.rs` (new)
- `crates/runie-engine/src/tool/mod.rs`
- `crates/runie-core/src/tool_runtime.rs` (new)
- `crates/runie-core/src/lib.rs`
- `crates/runie-testing/src/mock_tool_runtime.rs` (new)
- `crates/runie-testing/src/lib.rs`
- `crates/runie-testing/src/runner.rs`
- `crates/runie-tui/src/main.rs`
- `crates/runie-print/src/main.rs`
- `crates/runie-json/src/main.rs`
- `crates/runie-server/src/main.rs`
- `tasks/pure-snapshot-and-tool-runtime-trait.md`

## Notes

- The `ToolRuntime` trait is `Send + Sync` and takes owned `ToolCall` inputs to fit the async turn loop.
- Tool execution no longer calls `runie_engine::tool::builtin_registry()` directly from `turn.rs` or `headless.rs`; the registry is only used to build OpenAI function schemas and by the `EngineToolRuntime` adapter.
- The existing `runie_core::tool::runtime::ToolRuntime` trait (per-tool runtime with approval/sandbox hooks) remains untouched; it is a separate abstraction that may be wired in a future phase.
