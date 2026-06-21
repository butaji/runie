# Generate UI elements from model types

**Status**: todo
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P2

**Depends on**: rename-core-ui-to-view
**Blocks**: none

## Description

`runie-core/src/ui/elements.rs` defines `Element` variants (`UserMessage`, `AgentMessage`, `Thinking`, `ToolRunning`, `ToolDone`, `TurnComplete`) that are a UI projection of `ChatMessage`, `Role`, `Part`, `ToolOutput`, and `AgentEvent`. `PostKind` further categorizes the same material. This creates a parallel hierarchy that must be kept in sync with the model.

## Acceptance Criteria

- [ ] `Element` is generated from `ChatMessage`/`Part`/`Role`/`ToolOutput`/`AgentEvent` rather than hand-constructed in parallel.
- [ ] `ui/transform.rs` remains the single place where model → element mapping occurs.
- [ ] Rendering output is unchanged.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `element_from_user_chat_message` — `ChatMessage` with `Role::User` produces `Element::UserMessage`.
- [ ] `element_from_tool_output` — `ToolOutput` with success status produces `Element::ToolDone`.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] `feed_renders_unchanged` — a sample feed produces the same `Buffer` before and after the refactor.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/ui/elements.rs`
- `crates/runie-core/src/ui/transform.rs`
- `crates/runie-core/src/message/mod.rs`
- `crates/runie-core/src/tool/context.rs`

## Notes

Run after `rename-core-ui-to-view` so the view module boundary is clear.
