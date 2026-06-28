# Delete or merge the inspector tool pipeline

**Status**: todo
**Milestone**: R4
**Category**: Tools
**Priority**: P1

**Depends on**: centralize-built-in-tool-names
**Blocks**: none

## Description

The `inspect` command and its associated tool pipeline duplicate the rendering and execution path used by the main agent tool loop. Either delete `inspect` if it is unused, or merge it so it shares the same tool execution and display code.

## Acceptance Criteria

- [ ] `inspect` either is removed or delegates to the shared tool execution path.
- [ ] No separate inspector-specific rendering module remains unless it is a thin wrapper.
- [ ] Tool-call display uses one formatter across TUI, CLI, and inspector.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `tool_result_format_is_shared` — formatter output matches for agent and inspector inputs.

### Layer 2 — Event Handling
- [ ] N/A — command dispatch is unchanged.

### Layer 3 — Rendering
- [ ] `tool_output_renders_consistently` — a `TestBackend` buffer matches for both render paths after unification.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `inspect_delegates_to_tool_loop` — if kept, `inspect` produces the same events as a normal tool turn.

## Files touched

- `crates/runie-cli/src/inspect.rs`
- `crates/runie-cli/src/commands.rs`
- `crates/runie-tui/src/ui/tool.rs`
- `crates/runie-core/src/tool/display.rs`

## Notes

- If `inspect` is required for headless debugging, keep it as a CLI command that drives the same `ToolSkill` harness the agent uses.
- Do not maintain two formatters for markdown/plaintext output.
