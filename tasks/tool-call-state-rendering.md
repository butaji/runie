# Tool Call State Machine in UI

**Status**: todo
**Milestone**: R3
**Category**: UI / Feed
**Priority**: P1

**Depends on**: inline-tool-rendering, tool-registry-trait, streaming-buffer-tail-split

## Description

The existing `inline-tool-rendering.md` task formats tool calls as static
single-line blocks. This task upgrades them to first-class stateful objects:
`Pending` → `Running` → `Completed`/`Error`, with elapsed time, bytes, and
expand/collapse. Research from Goose (boxed tool cards), Codex (`HistoryCell`
trait), Gemini CLI (`ToolDisplay` formats), and thClaws (`ActiveToolDisplay`)
all converge on this pattern.

## Acceptance Criteria

- [ ] `crates/runie-core/src/tool.rs` defines `ToolCallState`:
  ```rust
  pub enum ToolCallState {
      Pending { id: String, name: String, input: Value },
      Running { id: String, name: String, input: Value, started: Instant },
      Completed { id: String, name: String, output: String, duration: Duration, bytes: usize },
      Error { id: String, name: String, error: String, duration: Duration },
  }
  ```
- [ ] `UiActor` maintains an `IndexMap<String, ToolCallState>` updated from
  `AgentEvent::ToolCallStart/Progress/End/Error`.
- [ ] TUI renders each state:
  - Running: spinner + `Run <name> '<args>'` + elapsed seconds.
  - Completed: `✓` + total duration + bytes transferred.
  - Error: `[✗]` + error summary.
- [ ] `Ctrl+O` expands a completed/error tool to show full input/output.
  (Builds on existing `ctrl-o-collapse-expand.md`.)
- [ ] Consecutive identical tool calls are coalesced (`×N`) like thClaws.
- [ ] `cargo build --workspace` succeeds.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `tool_state_transitions_pending_to_running` — `ToolCallStart` advances state.
- [ ] `tool_state_transitions_running_to_completed` — `ToolCallEnd` with success.
- [ ] `tool_state_records_duration` — elapsed time populated.
- [ ] `identical_calls_coalesced` — 3x `Read (src/lib.rs)` shows `×3`.

### Layer 2 — Event Handling
- [ ] `tool_progress_event_updates_bytes` — `ToolCallProgress { bytes }` updates
  state.

### Layer 3 — Rendering
- [ ] `running_tool_shows_spinner` — TestBackend line contains spinner char.
- [ ] `completed_tool_shows_checkmark_and_duration` — `✓` and `5.7s`.
- [ ] `expanded_tool_shows_full_output` — after `Ctrl+O`, output is visible.

## Notes

**Files touched:**
- `crates/runie-core/src/tool.rs`
- `crates/runie-core/src/event.rs` (add `ToolCallProgress`)
- `crates/runie-tui/src/message/mod.rs`
- `crates/runie-tui/src/ui.rs`

**Out of scope:**
- Gantt-style cross-tool timeline.
- Subagent inline tracking.
