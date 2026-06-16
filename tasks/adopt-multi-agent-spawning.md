# Adopt Multi-Agent Spawning with Depth Limits and Config Inheritance

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: (none)
**Blocks**: (none)

## Description

Implement multi-agent spawning patterns from codex-rs:

1. **AgentRegistry** — global registry with automatic nickname generation ("Alice the 1st")
2. **Depth limiting** — `exceeds_thread_spawn_depth_limit(depth, max_depth)` prevents infinite recursion
3. **Config inheritance** — child agents inherit provider, approval policy, sandbox, cwd from parent
4. **Spawn options** — `SpawnAgentOptions` with fork_mode, parent_thread_id, environments

Reference: `~/Code/agents/codex-rs/core/src/tools/handlers/multi_agents/`

## Acceptance Criteria

- [ ] `AgentRegistry` with `spawn()`, `send()`, `wait()`, `close()` methods.
- [ ] Depth limit configurable (default 3).
- [ ] Child agents inherit config from parent.
- [ ] Nickname auto-generation for agents.
- [ ] Graceful termination with `close_agent()`.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `agent_registry_creates_with_nickname` — automatic naming.
- [ ] `depth_limit_prevents_excessive_spawning` — respects max depth.
- [ ] `config_inheritance_preserves_parent_settings` — provider/approval/sandbox/cwd copied.

### Layer 2 — Event Handling
- [ ] `spawn_agent_event_creates_thread` — event creates new agent.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
- [ ] Smoke test: spawn 3 nested agents, verify each responds.

## Files touched

- `crates/runie-core/src/agent/` (new or existing)
- `crates/runie-core/src/multi_agent.rs` (new)

## Notes

Aligns with existing `r4-subagent-*` tasks. Use those as foundation.
