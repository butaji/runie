# Subagent Isolation

**Status**: done
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

## What was implemented

- `crates/runie-core/src/actors/mod.rs` (new) — actors module
- `crates/runie-core/src/actors/subagent.rs` (new, ~480 lines)
  - `SubagentContext` — isolated context derived from `SubagentTask`; contains only
    `task_id`, `role_prompt`, `task_description`, `allowed_tools`, `model_trait`,
    and a sanitized `ProjectSnapshot`. The `OrchestratorPlan` is **not** stored.
  - `SubagentStatus` — `Pending | Running | Done { output } | Failed { error }`
  - `SubagentCommand` — `Run | ReportStatus | Cancel | UpdateStatus`
  - `SubagentEvent` — `Started | StatusChanged | Completed | Failed | Cancelled`
  - `SubagentActor` — implements `Actor` trait, owns `SubagentContext`,
    drives execution loop, emits typed events to the shared bus.
  - `ProjectSnapshot` — `from_env()` constructs with `is_sensitive_env_key`
    redaction of env vars.
  - `is_sensitive_env_key()` — matches any key containing `SECRET`, `PASSWORD`,
    `PRIVATE_KEY`, `AWS_ACCESS_KEY`, `GITHUB_TOKEN`, `API_KEY`, `DATABASE_URL`,
    or prefixed with `RUNIE_`.
- `crates/runie-core/src/tool/mod.rs` — `pub(crate) tools` field on
  `ToolRegistry` to allow `filtered()` in actors module.
- `crates/runie-core/src/lib.rs` — `pub mod actors`

## Acceptance Criteria

- [x] `SubagentActor` is spawned with a `SubagentContext` derived from the
  parent `OrchestratorPlan` task.
- [x] Context includes only the assigned role prompt, task, filtered tool list,
  and a sanitized project snapshot.
- [x] Tool filter restricts the tool registry to the subset named in the task.
- [x] Subagent cannot access `OrchestratorActor` state directly.
- [x] Each subagent writes events to the shared event bus with its `task_id`.
- [x] `cargo test -p runie-core` passes (8 new tests).

## Tests

### Layer 1 — State / Logic

- `subagent_context_hides_orchestrator_plan` — task_id, description, role_prompt
  extracted; no full plan exposed.
- `subagent_context_respects_tool_filter` — `tool_filter` on task propagates to
  `allowed_tools`.
- `tool_filter_limits_registry` — only named tools present after filtering.
- `tool_filter_empty_allows_all` — empty filter means no restriction.
- `tool_filter_unknown_names_skipped` — unknown tool names silently ignored.
- `subagent_status_serialization` — JSON round-trip of `SubagentStatus`.
- `subagent_context_serialization` — JSON round-trip of `SubagentContext`.
- `project_snapshot_from_env` — no sensitive keys in env vars.
- `sensitive_env_keys_identified` — sensitive keys detected correctly.

## Files touched

- `crates/runie-core/src/actors/mod.rs` (new)
- `crates/runie-core/src/actors/subagent.rs` (new)
- `crates/runie-core/src/tool/mod.rs` — `tools` field made `pub(crate)`
- `crates/runie-core/src/lib.rs` — `pub mod actors`

## Out of scope

- Process-level sandboxing (separate OS processes per subagent).
- Network isolation.
- r4-subagent-sidebar (separate task).
- r4-team-mode-integration (separate task).
