# Ask-User Tool for Orchestrator Alignment

**Status**: todo
**Milestone**: R4
**Category**: Tools / State
**Priority**: P0

**Depends on**: r3-tool-registry-trait, r4-orchestrator-domain-types
**Blocks**: r4-one-shot-orchestrator-llm

## Description

Implement a built-in tool that lets the Orchestrator pause planning and ask the
user clarifying questions before generating the final OHP plan. The answer is
fed back into the Orchestrator context so it can refine the plan in one shot.

## Acceptance Criteria

- [ ] `AskUserTool` registered in the tool registry.
- [ ] Tool accepts a JSON payload `{ "question": "..." }` and returns a
  structured response.
- [ ] When invoked during Team mode planning, the runtime pauses and shows the
  question in the feed with a pending state.
- [ ] User reply is captured as an event and appended to the Orchestrator's
  working memory.
- [ ] A subagent plan cannot be submitted until all `AskUserTool` calls are
  resolved.

## Tests

### Layer 1 — State / Logic

```rust
#[test]
fn ask_user_tool_requires_question() {
    let tool = AskUserTool;
    let result = tool.execute(json!({}));
    assert!(result.is_err());
}

#[test]
fn ask_user_tool_returns_pending_marker() {
    let tool = AskUserTool;
    let result = tool.execute(json!({"question": "Which file?"})).unwrap();
    assert_eq!(result.status, ToolStatus::AwaitingUser);
}
```

### Layer 2 — Event Handling

```rust
#[test]
fn user_answer_appends_to_orchestrator_context() {
    let mut orch = OrchestratorContext::new();
    orch.record_question("Which file?");
    orch.record_answer("src/lib.rs");
    assert_eq!(orch.dialogue.len(), 2);
}
```

## Files touched

- `crates/runie-core/src/tools/ask_user.rs` (new)
- `crates/runie-core/src/tools/mod.rs`
- `crates/runie-core/src/orchestrator.rs`

## Out of scope

- General chat-style multi-turn in Team mode.
- UI rendering of the question prompt.
