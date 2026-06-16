# Team Mode Orchestration

## Context

Runie started as a single-agent terminal harness: one model, one conversation,
one turn loop. As users connect more providers and models, they want the system
to use the right model for the right job and to parallelize work when it makes
sense. Research across `~/Code/agents` (Claude Code, Kimi Code, OpenCode,
LangGraph, CrewAI, OpenAI Agents SDK, Factory/Droid) shows that the dominant
pattern is not a single god-agent, but a central orchestrator that decomposes a
goal into specialized roles and executes them.

We evaluated several coordination patterns:

- **Peer-to-peer handoffs** (OpenAI Agents SDK, Swarm): flexible but prone to
  infinite loops and unclear ownership.
- **Role-based crews** (CrewAI): great for business automation, but fixed roles
  add friction for a terminal coding tool.
- **Graph workflows** (LangGraph): powerful for complex branching and cycles,
  but overkill for the 80% case of sequential + parallel execution.
- **Supervisor + isolated subagents** (Claude Code, Kimi Code): gives the
  orchestrator clear ownership while keeping each subagent focused and
  token-efficient.

## Decision

### 1. Two Execution Modes: Solo and Team

- **Solo** is the default: one agent turn with the configured model.
- **Team** is toggled via command (`/team`) or settings (config file).
- The user controls the mode; the machine handles orchestration.

### 2. Depth = 1 (Orchestrator + Subagents Only)

- Depth 0: Orchestrator (coordinator)
- Depth 1: Subagents (workers spawned by orchestrator)
- No subagent spawning subagents (prevents infinite loops)

### 3. Bidirectional Sync Communication

```
Orchestrator → Subagent: dispatch task, steer, cancel
Subagent → Orchestrator: status updates, questions, results
Orchestrator → User: synthesis, approval requests
Subagent → User: (routed through orchestrator)
```

### 4. Config Inheritance Model

| Config | Inheritance |
|--------|-------------|
| Provider/API key | Inherit ✓ |
| **Model** | **NOT inherited** — explicitly specified per subagent |
| Tools | Filtered (allowed list from orchestrator) ✓ |
| Working directory | Inherit ✓ |
| Environment vars | Inherit (redacted secrets) ✓ |
| Permission mode | Inherit ✓ |

### 5. Model Selection: Explicit + Dynamic

- Subagents get explicit model names (e.g., `claude-sonnet-4-20250514`)
- `select_model(trait)` tool resolves `ModelTrait` → concrete model dynamically
- Orchestrator plans with traits, resolves to models at dispatch time

### 6. Subagent Lifecycle

1. Orchestrator dispatches with explicit model (via `select_model`)
2. Subagent executes with token budget
3. Subagent signals completion via `done(result)` tool
4. Orchestrator synthesizes results

### 7. Retry Strategy

```
Task fails
    ├── Retry 1 (same model)
    ├── Retry 2 (same model)
    ├── Retry 3 (same model)
    └── Fallback: Try another model with same ModelTrait
            ├── Success → continue
            └── All failed → Escalate to user
```

### 8. Subagent Naming

Format: `{Role}-{3 alphanumeric}` — e.g., `researcher-A1B`, `coder-X2Y`

User can override: `/team "Research X" as researcher-A1B`

### 9. Message Visibility

- All messages/events visible in feed
- Subagent streams collapsible/expandable
- User sees everything, can steer any subagent

### 10. Steering and Cancellation

- `/steer <agent> <message>` — redirect subagent mid-task
- `/cancel <agent>` — cancel and reschedule

### 11. Approval Routing

Subagent approval requests go to **orchestrator**, not directly to user.
Orchestrator can auto-approve safe tools or escalate to user.

### 12. Orchestrator Tools

| Tool | Description |
|------|-------------|
| `list_subagents` | List all active subagents with status |
| `get_subagent_status` | Get detailed status of specific subagent |
| `get_subagent_output` | Get partial/final output from subagent |
| `steer_subagent` | Send message to running subagent |
| `cancel_subagent` | Cancel a running subagent |
| `reschedule_task` | Cancel and requeue with modified task |

### 13. Synthesis Options

- **LLM synthesis** (default): Orchestrator uses LLM to combine results
- **Template-based**: Fixed templates for predictable tasks
- **Custom prompt**: `/team <task> --synthesize "<prompt>"`

## Consequences

- **Positive:** Higher-quality results via model specialization and parallel execution
- **Positive:** Token-efficient with isolated subagent contexts
- **Positive:** Clear ownership — orchestrator coordinates, subagents execute
- **Positive:** Users control mode via `/team` or settings
- **Trade-off:** Team mode adds latency and cost (multiple subagents + planning)
- **Trade-off:** Debugging multi-agent workflows requires per-agent feeds and logging
