# Subagents (`/spawn`)

**Status**: done
**Milestone**: R5
**Category**: Input & Commands

## Description

Adds a `/spawn <prompt>` command that runs a nested agent turn synchronously
and returns the final assistant response as a system message. The subagent
inherits the parent's provider, model, thinking level, and read-only flag.

Use cases:
- "Find all TODOs in this codebase and list them" (delegated research)
- "Explain what this function does" (delegated analysis)
- "Generate a commit message from these diffs" (delegated summarization)

## Architecture

```
/spawn <prompt>
  → runie-core: handle_spawn emits Event::SpawnAgent { prompt }
  → runie-term: catches the event, runs the subagent
      → runie-agent::subagent::run_subagent(prompt, provider, model, ...)
      → spawn_blocking (subagent builds its own tokio runtime)
      → run_agent_turn synchronously, captures AgentResponse events
  → result is sent back as Event::SystemMessage
  → user sees: Subagent "..." → <result snippet>
```

The split is intentional: `runie-core` already depends on `runie-agent`
(through `runie-provider`), but the agent has a runtime dependency on the
binary's tokio context, so the dispatch lives in `runie-term`.

## Acceptance Criteria

- [x] `/spawn <prompt>` runs a nested agent turn
- [x] Empty prompt shows usage
- [x] Subagent inherits parent's provider/model/thinking/read-only
- [x] Subagent has its own empty message buffer (no parent history leaks)
- [x] Progress message: `Subagent "..." [running…]`
- [x] Result message: `Subagent "..." → <snippet>`
- [x] Failure: `Subagent "..." failed: <error>`

## Out of scope (deferred)

- Parallel subagents (DAG orchestration like langgraph/claude-agent-sdk)
- Subagent streaming progress (only the final result is surfaced)
- Per-subagent model override (`/spawn --model=...`)
- Subagent cancellation (Ctrl+C cancels the active turn; subagents are
  not yet interruptible)

## Files

- `crates/runie-agent/src/subagent.rs` — `run_subagent()` + `SubagentError`
- `crates/runie-core/src/commands/handlers/subagent.rs` — `/spawn` command
- `crates/runie-core/src/event.rs` — `Event::SpawnAgent { prompt: String }`
  replaces the old empty `SpawnAgent` variant
- `crates/runie-term/src/main.rs` — event_loop catches `SpawnAgent` and
  dispatches the subagent in `spawn_blocking`
