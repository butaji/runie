# Subagent Isolation

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Security
**Priority**: P0

**Depends on**: r4-orchestrator-actor, permission-rulesets
**Blocks**: r4-team-mode-integration

## Description

Ensure each subagent runs with its own isolated context: only the role prompt,
task description, allowed tools, and project context are visible. Subagents must
not see each other's outputs, the Orchestrator's internal plan, or the user's
full session history.

## Acceptance Criteria

- [ ] `SubagentActor` is spawned with a `SubagentContext` derived from the
  parent `OrchestratorPlan` task.
- [ ] Context includes only the assigned role prompt, task, filtered tool list,
    and a sanitized project snapshot.
- [ ] Tool filter restricts the tool registry to the subset named in the task.
- [ ] Subagent cannot access `OrchestratorActor` state directly.
- [ ] Each subagent writes events to a per-subagent event stream, not the main
  session feed.

## Tests

### Layer 1 — State / Logic

```rust
#[test]
fn subagent_context_hides_orchestrator_plan() {
    let plan = sample_plan();
    let ctx = SubagentContext::from_task(&plan.tasks[0], &plan);
    assert!(ctx.orchestrator_plan.is_none());
    assert_eq!(ctx.task_description, "Review src/main.rs");
}

#[test]
fn tool_filter_limits_registry() {
    let registry = tool_registry();
    let filtered = registry.filter(&["read_file", "grep"]);
    assert_eq!(filtered.len(), 2);
    assert!(filtered.get("bash").is_none());
}
```

### Layer 2 — Event Handling

```rust
#[test]
fn subagent_events_are_tagged_with_subagent_id() {
    let mut sub = SubagentActor::new("t1".into(), context());
    sub.handle(SubagentCommand::Run);
    let events = sub.drain_events();
    assert!(events.iter().all(|e| e.subagent_id == "t1"));
}
```

## Files touched

- `crates/runie-core/src/actors/subagent.rs` (new)
- `crates/runie-core/src/actors/mod.rs`
- `crates/runie-core/src/tools/mod.rs`
- `crates/runie-core/src/permissions.rs`

## Out of scope

- Process-level sandboxing.
- Network isolation.
