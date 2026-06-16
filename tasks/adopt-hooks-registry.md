# Adopt Hooks Registry System

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: (none)
**Blocks**: (none)

## Description

Implement an event hooks system for extensibility, inspired by Codex's 10 hook events:

| Hook Event | Purpose |
|------------|---------|
| `PreToolUse` | Modify/deny tool calls before execution |
| `PostToolUse` | Log/transform tool output |
| `PermissionRequest` | Handle approval prompts |
| `PreCompact` | Modify context before compaction |
| `PostCompact` | Handle compacted context |
| `SessionStart` | Initialize session state |
| `UserPromptSubmit` | Transform user input |
| `SubagentStart` | Notify when subagent spawns |
| `SubagentStop` | Cleanup when subagent exits |
| `Stop` | Graceful shutdown |

Each hook receives JSON on stdin and returns a decision (`allow`, `deny`, or modified input).

Reference: `~/Code/agents/codex-rs/core/src/hooks/`

## Acceptance Criteria

- [ ] `HookRegistry` with `register(event, handler)` and `emit(event, payload)`.
- [ ] Hooks can modify tool input/output.
- [ ] Hooks can deny/allow actions.
- [ ] Config loading from `hooks` section of config.
- [ ] Built-in hooks: logging, permission, compaction.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `hook_registry_calls_handler_on_event` — event triggers handler.
- [ ] `hook_can_modify_input` — handler transforms payload.
- [ ] `hook_can_deny_action` — deny decision returned correctly.

### Layer 2 — Event Handling
- [ ] `pre_tool_hook_intercepts_tool_call` — hook called before tool execution.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-core/src/hooks.rs` (new)

## Notes

Hooks enable powerful extensions: rate limiting, custom permission policies, logging backends.
