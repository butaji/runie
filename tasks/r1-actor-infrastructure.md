# Actor infrastructure: Tool, Queue, Session, Config agents

**Status**: todo
**Milestone**: R1
**Category**: Core Architecture

## Description

Implement the 4 core actors that currently exist as stubs (ActorId defined,
Orchestrator spawn methods exist, but no actual actor implementations).

## Scope

Replaces these 4 separate tasks:
- `r1-actor-tool-actors`
- `r1-actor-queue-agent`
- `r1-actor-session-manager`
- `r1-actor-config-agent`

## Acceptance Criteria

- [ ] **ToolActor**: Spawn per tool invocation, execute asynchronously, emit ToolEnd
- [ ] **QueueAgent**: Hold message queue, emit SpawnAgent when agent idle
- [ ] **SessionManager**: Handle save/load/list/delete domain events
- [ ] **ConfigAgent**: Watch config file, emit ConfigChanged on change

## Tests

- [ ] Layer 1 — `tool_actor_executes_bash` — spawn + result
- [ ] Layer 1 — `queue_agent_batches_messages` — queue then emit
- [ ] Layer 1 — `session_manager_save_load` — event log roundtrip
- [ ] Layer 2 — `config_agent_emits_on_file_change` — notify integration

## Notes

- These are infrastructure tasks with no direct user-facing value.
- They unblock user-facing features: keybindings (needs ConfigAgent), queue delivery modes (needs QueueAgent).
- Do NOT over-engineer. Each actor should be <200 lines.
