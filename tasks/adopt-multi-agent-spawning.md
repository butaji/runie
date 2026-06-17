# Adopt Multi-Agent Spawning

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: (none)
**Blocks**: (none)

## Description

Implement multi-agent spawning for Team mode with the following design decisions:

### Architecture
- **Depth = 1**: Orchestrator (depth 0) spawns Subagents (depth 1). No subagent spawning subagents.
- **Communication**: Bidirectional sync via EventBus
- **Execution**: Subagents run to completion with explicit `done` tool signal

### Config Inheritance
| Config | Behavior |
|--------|----------|
| Provider/API key | Inherit |
| **Model** | NOT inherited — explicitly specified per subagent via `select_model(trait)` tool |
| Tools | Filtered (allowed list from orchestrator) |
| Working directory | Inherit |
| Environment vars | Inherit (redacted secrets) |
| Permission mode | Inherit |

### Subagent Lifecycle
1. Orchestrator dispatches with explicit model (from `select_model(trait)`)
2. Subagent executes with token budget
3. Subagent signals completion via `done(result)` tool
4. Orchestrator synthesizes results

### Retry Strategy
```
Task fails
    ├── Retry 1 (same model)
    ├── Retry 2 (same model)
    ├── Retry 3 (same model)
    └── Fallback: Try another model with same ModelTrait
            ├── Success → continue
            └── All failed → Escalate to user
```

### Naming Convention
`{Role}-{3 alphanumeric}` — e.g., `researcher-A1B`, `coder-X2Y`

### Messaging
- **Orchestrator → Subagent**: `/steer <agent> <message>`, `/cancel <agent>`
- **Subagent → Orchestrator**: Results, questions (via `AskUserTool`), status updates
- **Approval requests**: Routed to orchestrator (not user directly)

### Orchestrator Tools
| Tool | Description |
|------|-------------|
| `list_subagents` | List all active subagents with status |
| `get_subagent_status` | Get detailed status of specific subagent |
| `get_subagent_output` | Get partial/final output from subagent |
| `steer_subagent` | Send message to running subagent |
| `cancel_subagent` | Cancel a running subagent |
| `select_model` | Built-in tool that resolves ModelTrait to concrete model |

### Synthesis Options
- **LLM synthesis** (default): Orchestrator uses LLM to combine results
- **Template-based**: Fixed templates for predictable tasks
- **Custom prompt**: `/team <task> --synthesize "<prompt>"`

## Acceptance Criteria

- [x] `AgentRegistry` with `spawn()`, `send()`, `wait()`, `close()` methods.
- [x] Depth limit = 1 (orchestrator + subagents only).
- [x] Config inheritance: provider, tools (filtered), cwd, env (redacted), permissions.
- [x] Model NOT inherited — specified per subagent via `select_model` tool.
- [x] Subagent naming: `{Role}-{3 alphanumeric}`.
- [x] `done` tool for explicit completion signal.
- [x] Token budget per subagent.
- [x] 3 retries → same-trait fallback → user escalation.
- [x] Steer (`/steer`) and cancel (`/cancel`) commands.
- [x] Orchestrator tools: list/get_status/get_output/steer/cancel.
- [x] Approval requests routed to orchestrator.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `subagent_naming_format` — `researcher-A1B` matches pattern.
- [x] `retry_strategy_3_retries` — exactly 3 retries before fallback.
- [x] `model_not_inherited` — subagent must have explicit model.
- [x] `config_inheritance_excludes_model` — other configs inherited.
- [x] `depth_limit_one_level` — subagent cannot spawn subagent.

### Layer 2 — Event Handling
- [x] `steer_command_delivers_message` — `/steer` reaches subagent.
- [x] `cancel_command_stops_subagent` — `/cancel` terminates subagent.
- [x] `done_tool_signals_completion` — `done` triggers completion event.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
- [x] Smoke test: spawn team, verify subagent responds.

## Files touched

- `crates/runie-core/src/agent/` (new or existing)
- `crates/runie-core/src/multi_agent.rs` (new)
- `crates/runie-core/src/tools/` — `select_model`, `done` tools

## Notes

Aligns with existing `r4-orchestrator-*` and `r4-subagent-*` tasks.
