# Collapse two ToolRuntime traits into one

**Status**: todo
**Milestone**: R4
**Category**: Tools
**Priority**: P1

**Depends on**: delete-dead-tool-runtime-trait
**Blocks**: none

## Description

After `delete-dead-tool-runtime-trait` removes the unused rich `tool::ToolRuntime`, two layers remain:

- `tool::Tool` (concrete, `tool/registry.rs:13`) — implemented by every engine tool, has `name()`, `description()`, `input_schema()`, `is_read_only()`, `requires_approval()`, `async fn call(input, ctx) -> Result<ToolOutput>`.
- `tool_runtime::ToolRuntime` (injectable, `tool_runtime.rs:43`) — `async fn execute(ToolCall) -> Result<ToolResult, ToolError>`, used by the agent turn for test injection.
- `engine/tool/runtime_adapter.rs::EngineToolRuntime` — the *only* production `ToolRuntime` impl, a ~20-line bridge that looks up the tool in `builtin_registry()` and calls `tool.call(input, ctx)`.
- `runie-testing/src/mock_tool_runtime.rs` — a mock `ToolRuntime` for Layer 4 tests.

With a single production impl, the indirection exists purely for test injection. But `tool::Tool` is already a trait with `call()` — tests can provide fake `Tool` impls and a `ToolRegistry::from(vec![Arc::new(FakeTool)])`. Collapsing to one trait removes: `tool_runtime.rs` (46 LOC), `engine/tool/runtime_adapter.rs` (~30 LOC), `runie-testing/src/mock_tool_runtime.rs`, the `ToolCall`/`ToolResult`/`ToolError` triple, and one trait indirection in `run_agent_turn`.

Supersedes the "kept both" outcome of the done task `pure-snapshot-and-tool-runtime-trait`.

## Acceptance Criteria

- [ ] Decision made: EITHER
  - (a) **Collapse** — agent turn takes `&ToolRegistry`; tests inject fake `Tool` impls; `tool_runtime.rs`, `runtime_adapter.rs`, `mock_tool_runtime.rs` deleted; `ToolCall`/`ToolResult`/`ToolError` removed; OR
  - (b) **Keep + document** — a concrete orphan-rule / test-injection blocker is written into `tool_runtime.rs` justifying the second trait (not a hand-wave).
- [ ] If (a): `rg "pub trait ToolRuntime" crates/` returns zero hits (the `tool::Tool` trait remains).
- [ ] If (a): `runie_agent::run_agent_turn` / `run_headless_turn` signatures take `&ToolRegistry` (or `Arc<ToolRegistry>`) instead of `&dyn ToolRuntime`.
- [ ] If (a): `runie-testing` Layer 4 fixtures build a `ToolRegistry` with fake `Tool` impls.
- [ ] No `async-trait` use remains in `tool_runtime.rs` / `runtime_adapter.rs` (deleted) — audit `async-trait` dep after.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `fake_tool_implements_call` — a `struct EchoTool; impl Tool for EchoTool { ... }` returns a fixed `ToolOutput` via `call()`.
- [ ] `registry_lookup_by_name` — `registry.get("echo")` returns the registered fake tool.

### Layer 2 — Event Handling
- N/A — trait collapse, not event flow.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `agent_turn_with_fake_tool_completes` — the existing `runie-testing` Layer 4 fixture (`minimax_m3_multi_tool_turn` style) still reaches `TurnComplete` after the collapse, with fake `Tool` impls standing in for `list_dir` / `read_file`.
- [ ] `mock_provider_turn_still_passes` — every existing Layer 4 fixture under `crates/runie-agent/tests/` and `crates/runie-testing/` still passes.

## Files touched

- `crates/runie-core/src/tool_runtime.rs` (deleted if option a)
- `crates/runie-engine/src/tool/runtime_adapter.rs` (deleted if option a)
- `crates/runie-testing/src/mock_tool_runtime.rs` (deleted if option a)
- `crates/runie-agent/src/turn.rs` / `headless.rs` / `tool_runner.rs` (signatures changed if option a)
- `crates/runie-testing/src/runner.rs` / `fixtures.rs` (build fake `ToolRegistry` if option a)
- `crates/runie-agent/src/tests/tool_runtime.rs` (rewritten or deleted)

## Notes

The done task `pure-snapshot-and-tool-runtime-trait` deliberately kept both traits to preserve the test-injection seam. This task reopens that decision under the YAGNI posture: one trait is enough if tests can use `Tool` fakes. If (b) is chosen, link the documented blocker back to `pure-snapshot-and-tool-runtime-trait` notes. Run after `delete-dead-tool-runtime-trait` so only the two live layers remain to reason about.
