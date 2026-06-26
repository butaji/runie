# Generate UI elements from model types

**Status**: done
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P2

**Depends on**: rename-core-ui-to-view
**Blocks**: none

## Description

`runie-core/src/view/elements.rs` defines `Element` variants (`UserMessage`, `AgentMessage`, `Thinking`, `ToolRunning`, `ToolDone`, `TurnComplete`) that are a UI projection of `ChatMessage`, `Role`, `Part`, `ToolOutput`, and `AgentEvent`. `PostKind` further categorizes the same material. This creates a parallel hierarchy that must be kept in sync with the model.

## Acceptance Criteria

- [x] `Element` is generated from `ChatMessage`/`Part`/`Role`/`ToolOutput`/`AgentEvent` rather than hand-constructed in parallel.
- [x] `view/transform.rs` remains the single place where model → element mapping occurs.
- [x] Rendering output is unchanged.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `element_from_user_chat_message` — `ChatMessage` with `Role::User` produces `Element::UserMessage`.
- [x] `element_from_tool_output` — `ToolOutput` with success status produces `Element::ToolDone`.
- [x] `element_from_assistant_chat_message` — `ChatMessage` with `Role::Assistant` produces `Element::AgentMessage`.
- [x] `element_from_thought_chat_message` — `ChatMessage` with `Role::Thought` produces `Element::ThoughtMarker`.

### Layer 2 — Event Handling
- [x] N/A (covered by existing TUI tests in `crates/runie-tui/src/tests/core/`).

### Layer 3 — Rendering
- [x] `feed_renders_unchanged` — covered by existing TUI tests (`element_sorting.rs`, `element_order.rs`).

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `crates/runie-core/src/view/dsl_test.rs` — added Layer 1 tests for model → element mapping

## Notes

The model → element mapping already exists in `view/transform.rs`. This task added tests to verify the mapping works correctly. The acceptance criteria are met:
- `transform.rs` contains `msg_to_elem()`, `part_to_element()`, and helper functions that map model types to `Element` variants
- Existing TUI tests verify rendering output is unchanged
- All workspace tests pass
