# Agent Patterns

Runie supports three orchestration patterns for multi-agent workflows. Patterns are sold as first-class features — discoverable via `/mode`, configurable per session, and composable from existing Runie primitives.

## Patterns

| Pattern | Use Case | Leader Role | Workers Role |
|---------|----------|-------------|--------------|
| **single** | 80% prototyping tasks | Direct execution | — |
| **swarm** | Coordinated multi-agent work | Orchestrates workers | Fan-out, delegate, or form DAG |
| **improve** | Critical review loops | Evaluates output | Revises based on feedback |

## Config

Minimal TOML config — models come from existing `/model` and `/provider` UX:

```toml
[mode]
active = "single"     # single | swarm | improve
workers = 3           # max parallel workers
max_rounds = 5        # max iterations (improve, swarm)
timeout_ms = 120000   # per-task timeout (2 minutes)
max_retries = 2       # retries per task on failure
circuit_breaker = 3  # consecutive failures before fail-fast
```

### Defaults

| Setting | Default | Description |
|---------|---------|-------------|
| `active` | `single` | Current pattern |
| `workers` | `3` | Max parallel workers |
| `max_rounds` | `5` | Max iterations |
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
/mode                        # show current pattern + config
/mode list                   # show available patterns
/mode swarm                  # switch to swarm (uses /model list)
/mode swarm workers 5       # switch + override workers
/mode improve         # switch to improve
/mode single                 # back to single (default)
```

### Swarm Variants

Swarm has three execution modes, set per session:

```bash
/mode swarm parallel "process these 10 files"        # fan-out workers
/mode swarm delegation "code review with reviewer"   # leader delegates
/mode swarm dag "research async Rust patterns"      # DAG with dependencies
```

| Variant | Description | When to use |
|---------|-------------|-------------|
| **parallel** | Fan-out to all workers | Independent subtasks, bulk work |
| **delegation** | Leader assigns tasks | Known roles, clear workflow |
| **dag** | Workers form dependency graph | Discovery, research, waves |

## Architecture

### Crate: `runie-patterns`

New crate under `crates/runie-patterns/`:

```
crates/runie-patterns/
├── src/
│   ├── lib.rs              # Pattern trait + registry
│   ├── primitives/          # Core primitives (phase-gated)
│   │   ├── mod.rs
│   │   └── dag.rs          # Phase 3: DAG building + cycle detection
│   ├── single.rs           # Phase 1: Single agent (pass-through)
│   ├── swarm.rs            # Phase 2: All variants (parallel, delegation, dag)
│   └── improve.rs          # Phase 3: Review + revise loop
└── Cargo.toml
```

### Reuses Existing Primitives

| Primitive | Location | Purpose |
|-----------|----------|---------|
| `WorkerRunner` | `runie-patterns` | Executes one agent turn; implemented in runie-tui over `runie_agent::subagent::run_subagent` |
| `SubagentType` | `runie-core` | Declarative agent configs |
| Event bus | `runie-core` | Coordination between agents |
| `tokio::sync::Semaphore` | tokio | Concurrency control |

### Core Primitives (NEW)

```rust
/// DAG for swarm dag variant (Phase 3)
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
    pub config: PatternConfig,      // mirrors [mode] section
    pub models: Vec<(String, String)>, // (provider, model) priority list
    pub semaphore: Arc<tokio::sync::Semaphore>,
    pub trace_tx: TraceSender,      // mpsc::UnboundedSender<AgentTrace>
    pub abort: CancellationToken,
    pub runner: Arc<dyn WorkerRunner>,
}

/// Executes one agent turn and returns the final assistant text.
/// Implemented in runie-tui over runie_agent::subagent::run_subagent;
/// mocked in tests. (Deviation from the original LeaderHandle design:
/// LeaderAgentHandle::run is fire-and-forget and cannot return worker
/// text, so patterns go through this trait instead.)
#[async_trait]
pub trait WorkerRunner: Send + Sync {
    async fn run(&self, task: WorkerTask) -> anyhow::Result<String>;
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
| **swarm** | Shared + worker-scoped | Leader holds context, workers scoped; dag variant checkpoints per wave |
| **improve** | Accumulates revisions | Full history for evaluator to assess progress |

## Error Handling

| Setting | Default | Description |
|---------|---------|-------------|
| `max_retries` | `2` | Retries per task on failure |
| `circuit_breaker` | `3` | Consecutive failures before fail-fast |

- **Fail-fast**: Circuit opens after `circuit_breaker` consecutive failures
- **Partial results**: Completed workers return their output even if pattern fails
- **Retries**: Attempt 1 + 2 retries = 3 total attempts per task
- **Cancellation**: Abort signal propagated to all workers; clean shutdown within 5s

## Discovery

Pattern selection is discoverable via:

1. `/mode list` — shows all patterns with descriptions
2. `/mode <pattern> [workers N]` — switch with optional worker override
3. `/mode swarm <parallel|delegation|dag>` — switch + set session variant

## Implementation Status (shipped)

1. **Phase 1** ✅ — Pattern trait + single pattern + `[mode]` config + `/mode` command
2. **Phase 2** ✅ — Swarm parallel + delegation; TUI routes non-single turns
   through `runie-patterns`; per-worker lifecycle feed rows
   (`Event::PatternWorkerSpawned/Finished` → `Element::SubagentRow`,
   grok-style `✓ Subagent completed in Xs: "<task>"`, expandable to the
   worker transcript)
3. **Phase 3** ✅ — Swarm dag variant (topological waves, dependency context,
   cycle detection, skip-on-failed-dependency) + improve
   (generate → review → revise loop, `Approved`/`MaxRoundsReached`)

Notes on the shipped design:

- **Turn hook**: `UiActor` intercepts `TurnStarted` when `mode.active !=
  "single"` and spawns the pattern; it publishes the agent actor's exact
  terminal contract (`Response` + `TurnComplete` + `Done`, or `Error` +
  `Done`), so TurnActor/follow-up queues need no changes. Abort cancels the
  pattern's `CancellationToken` without double-finalizing.
- **Feed rows**: traces are published post-hoc (patterns emit traces on
  completion), so lifecycle rows appear with the final state; live
  in-flight rows would need per-event streaming.
- **Mock provider markers** (deterministic, for black-box tests):
  `[swarm-plan parallel|delegation]` → JSON task array; `[swarm-plan dag]`
  → JSON objects with `deps`; `[swarm-synthesize]` → summary;
  `[improve-generate]` → draft; `[improve-revise]` → revision; `[improve-review]` →
  `APPROVED`.
- **Descoped**: tab completion for `/mode <pattern>` (no subcommand
  completion infrastructure exists) and `/mode <pattern> help`.

## Anti-Goals

- **No pattern nesting** — flat patterns only; complexity stays tractable
- **No unbounded concurrency** — always bounded by `workers`
- **No infinite loops** — always bounded by `max_rounds`
- **No explicit model config in patterns** — use `/model`
- **No config-based approval gates** — approval is runtime decision by evaluator agent

## References

- oh-my-pi: tokio Semaphore + mapWithConcurrencyLimit for parallel execution
- oh-my-pi swarm: DAG with topological sort for wave execution
- crewAI hierarchical process: delegation tools injected into manager agent
- AutoGen: typed messages with explicit approval signal
- LangGraph: checkpointing for state persistence
