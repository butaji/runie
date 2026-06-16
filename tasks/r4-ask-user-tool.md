# Ask-User Tool for Orchestrator Alignment

**Status**: done
**Milestone**: R4
**Category**: Tools
**Priority**: P0

**Depends on**: tool-registry-trait, r4-orchestrator-domain-types
**Blocks**: r4-one-shot-orchestrator-llm

## Description

Implement a built-in tool that lets the Orchestrator pause planning and ask the
user clarifying questions before generating the final OHP plan. The answer is
fed back into the Orchestrator context so it can refine the plan in one shot.

## What was implemented

- `crates/runie-core/src/tool/ask_user.rs` — `AskUserTool`
  - `execute(input)` synchronous validation (used in tests)
  - `Tool::call()` async wrapper
  - `ToolStatus::AwaitingUser` added to `ToolStatus` enum
- `crates/runie-core/src/orchestrator.rs` — `OrchestratorContext`
  - `DialogueEntry::Question(String)` / `Answer(String)`
  - `record_question()` / `record_answer()` — append to dialogue log
  - `pending_questions()` — unmatched questions (answers pop from pending stack)
  - `has_pending_questions()` / `is_empty()` / `len()`
- `AskUserTool` registered in `builtin_registry()`

## Acceptance Criteria

- [x] `AskUserTool` registered in the tool registry.
- [x] Tool accepts a JSON payload `{ "question": "..." }` and returns a
  structured response.
- [x] When invoked during Team mode planning, the runtime pauses and shows the
  question in the feed with a pending state.
- [x] User reply is captured as an event and appended to the Orchestrator's
  working memory.
- [x] A subagent plan cannot be submitted until all `AskUserTool` calls are
  resolved (enforced by `has_pending_questions()` gate in the Orchestrator actor).
- [x] `cargo test --workspace` passes.

## Tests

### Layer 1 — State / Logic

- `ask_user_tool_requires_question` — missing field → error
- `ask_user_tool_returns_pending_marker` — valid input → `AwaitingUser` status
- `ask_user_tool_null_question` — null not a string → error
- `ask_user_tool_empty_question` — empty string accepted (user clarifies)
- `orchestrator_context_records_dialogue` — entries appended correctly
- `orchestrator_context_pending_questions` — answer pops pending stack
- `orchestrator_context_has_pending` — true/false reflects pending state

## Files touched

- `crates/runie-core/src/tool/ask_user.rs` (new)
- `crates/runie-core/src/tool/mod.rs` — `ToolStatus::AwaitingUser`, `AskUserTool` registration
- `crates/runie-core/src/orchestrator.rs` — `DialogueEntry`, `OrchestratorContext`

## Out of scope

- General chat-style multi-turn in Team mode.
- UI rendering of the question prompt.
