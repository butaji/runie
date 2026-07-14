# Agent Patterns

Runie supports five orchestration patterns for multi-agent workflows. Patterns are sold as first-class features — discoverable via `/mode`, configurable per session, and composable from existing Runie primitives.

## Patterns

| Pattern | Use Case | Leader Role | Workers Role |
|---------|----------|-------------|--------------|
| **single** | 80% prototyping tasks | Direct execution | — |
| **hierarchical** | Cross-functional teams with clear roles | Orchestrates + delegates | Execute specific roles |
| **parallel** | Bulk independent subtasks | Splits + fans out | Process in parallel |
| **eval-optimizer** | Critical review loops | Evaluates output | Revises based on feedback |
| **swarm** | Discovery and research | Coordinates discovery | Explore + synthesize |

## Config

Minimal TOML config — models come from existing `/model` and `/provider` UX:

```toml
[mode]
active = "single"        # single | hierarchical | parallel | eval-optimizer | swarm
workers = 3              # max parallel agents (ignored for single)
max_rounds = 5           # max iterations (eval-optimizer, swarm)
timeout_ms = 120000      # per-task timeout (2 minutes)
max_retries = 2          # retries per task on failure
circuit_breaker = 3      # consecutive failures before fail-fast
```

### Defaults

| Setting | Default | Description |
|---------|---------|-------------|
| `active` | `single` | Current pattern |
| `workers` | `3` | Max parallel agents for non-single patterns |
| `max_rounds` | `5` | Max iterations (eval-optimizer, swarm) |
| `timeout_ms` | `120000` | Per-task timeout (2 minutes) |
| `max_retries` | `2` | Retries per task on failure |
| `circuit_breaker` | `3` | Consecutive failures before fail-fast |

### Model Fallback

If configured models < `workers`:
- **Leader** always uses first model in `/model` list
- **Workers** reuse leader model if insufficient models configured
- No error; pattern proceeds with available models

## UX

All interaction via `/mode`:

```bash
/mode                       # show current pattern + config
/mode list                  # show available patterns
/mode swarm                 # switch to swarm (uses /model list)
/mode swarm workers 5      # switch + override workers
/mode eval-optimizer max_rounds 5  # override max rounds
/mode single                # back to single (default)
/mode swarm "Research async Rust patterns"  # set goal (swarm only)
```

### Pattern-specific Behavior

- **swarm**: Goal set per session via `/mode swarm "goal description"`
- **hierarchical**: Roles inferred from `workers` list, LLM picks agent for each role
- **eval-optimizer**: Review loop until `max_rounds` reached or evaluator approves
- **parallel**: Task list derived from input context, aggregated at end
- **single**: Uses `/model` directly, no pattern overhead

## Architecture

### Crate: `runie-patterns`

New crate under `crates/runie-patterns/`:

```
crates/runie-patterns/
├── src/
│   ├── lib.rs              # Pattern trait + registry
│   ├── primitives/          # Core primitives (phase-gated)
│   │   ├── mod.rs
│   │   ├── dag.rs          # Phase 5: DAG building + cycle detection
│   │   └── termination.rs  # Termination conditions
│   ├── single.rs           # Phase 1: Single agent (pass-through)
│   ├── parallel.rs         # Phase 2: Fan-out + fan-in
│   ├── hierarchical.rs     # Phase 3: Manager + role workers
│   ├── eval_optimizer.rs   # Phase 4: Review + revise loop
│   └── swarm.rs            # Phase 5: DAG-based discovery
└── Cargo.toml
```

### Reuses Existing Primitives

| Primitive | Location | Purpose |
|-----------|----------|---------|
| `LeaderHandle` | `runie-core` | Coordinates all agents |
| `AgentActorFactory` | `runie-core` | Spawns worker agents |
| `LeaderAgentHandle` | `runie-core` | Sends commands to agents |
| `SubagentType` | `runie-core` | Declarative agent configs |
| Event bus | `runie-core` | Coordination between agents |
| `tokio::sync::Semaphore` | tokio | Concurrency control |

### Core Primitives (NEW)

```rust
/// DAG for swarm pattern (Phase 5 only)
pub struct Dag {
    nodes: HashMap<AgentId, AgentConfig>,
    edges: Vec<(AgentId, AgentId)>,  // (from, to) = "from waits for to"
}

impl Dag {
    pub fn add_node(&mut self, id: AgentId, config: AgentConfig);
    pub fn add_edge(&mut self, from: AgentId, to: AgentId);
    pub fn topological_sort(&self) -> Result<Vec<Vec<AgentId>>>;  // waves
    pub fn detect_cycles(&self) -> Result<(), CycleError>;
}

/// Termination conditions
pub enum TerminationReason {
    Completed,
    MaxRoundsReached,
    Timeout,
    Error(String),
    Approved,
}
```

### Pattern Trait

```rust
pub trait Pattern: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &str;
    fn execute(&self, ctx: &Context, input: &str) -> impl Future<Output = Result<PatternOutput>>;
}

pub struct PatternOutput {
    pub result: String,
    pub termination: TerminationReason,
    pub traces: Vec<AgentTrace>,
}

pub struct AgentTrace {
    pub agent_id: String,
    pub start_time: DateTime<Utc>,
    pub duration_ms: u64,
    pub events: Vec<TraceEvent>,
}

pub enum TraceEvent {
    Handoff { from: String, to: String },
    ToolCall { tool: String, duration_ms: u64 },
    Error { error: String },
    Termination { reason: TerminationReason },
}
```

### Context

```rust
pub struct Context {
    pub leader: LeaderHandle,
    pub session: SessionState,
    pub config: ConfigState,
    pub semaphore: Arc<tokio::sync::Semaphore>,
    pub trace_tx: TraceSender,
    pub abort: CancellationToken,
}
```

### Cancellation

All patterns must honor `Context.abort`:
- On abort signal, all in-flight agents receive abort
- Partial results from completed workers are preserved
- Pattern returns `TerminationReason::Error("aborted")`
- No zombie tasks; clean shutdown within 5 seconds

## Model Selection

Models are configured via existing `/model` and `/provider` commands:

- **Leader** = current `/model` setting (first in priority list)
- **Workers** = next N models in priority list, up to `workers` config

No separate model config in patterns — keeps it simple.

## State Isolation

| Pattern | State Scope | Recommendation |
|---------|-------------|----------------|
| **single** | N/A | Direct execution, full context |
| **hierarchical** | Leader holds shared, workers scoped | Aggregate worker outputs before next delegation |
| **parallel** | Shared read, isolated writes | Fan-out with aggregation, merge at end |
| **eval-optimizer** | Accumulates revisions | Full history for evaluator to assess progress |
| **swarm** | Distributed, loosely coupled | DAG state with checkpoints per wave |

## Error Handling

| Setting | Default | Description |
|---------|---------|-------------|
| `max_retries` | `2` | Retries per task on failure (not per worker) |
| `circuit_breaker` | `3` | Consecutive failures before fail-fast |

- **Fail-fast**: Circuit opens after `circuit_breaker` consecutive failures
- **Partial results**: Completed workers return their output even if pattern fails
- **Retries**: Attempt 1 + 2 retries = 3 total attempts per task
- **Cancellation**: Abort signal propagated to all workers; clean shutdown within 5s

## Discovery

Pattern selection is discoverable via:

1. `/mode list` — shows all patterns with descriptions
2. Tab completion for `/mode <pattern>`
3. Help text per pattern via `/mode <pattern> help`

## Implementation Phases

1. **Phase 1**: Pattern trait + single pattern (baseline)
2. **Phase 2**: Parallel pattern (fan-out workers, tokio Semaphore)
3. **Phase 3**: Hierarchical pattern (role-based delegation)
4. **Phase 4**: Eval-optimizer pattern (review loop + termination)
5. **Phase 5**: Swarm pattern (DAG building, wave execution)

## Anti-Goals

- **No pattern nesting** — flat patterns only; complexity stays tractable
- **No unbounded concurrency** — always bounded by `workers`
- **No infinite loops** — always bounded by `max_rounds`
- **No explicit model config in patterns** — use `/model`
- **No config-based approval gates** — approval is runtime decision by evaluator agent (eval-optimizer)

## References

- crewAI hierarchical process: delegation tools injected into manager agent
- oh-my-pi: tokio Semaphore + mapWithConcurrencyLimit for parallel execution
- oh-my-pi swarm: DAG with topological sort for wave execution
- AutoGen: typed messages with explicit approval signal
- LangGraph: checkpointing for state persistence
