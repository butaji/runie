# Multi-Agent Multi-Model Orchestration — Conceptual Vision

> **Status**: Brainstorming / Conceptual. Not yet a task or implementation plan.

---

## TL;DR

The multi-agent multi-model research across 20+ frameworks reveals a clear consensus:

- **Orchestration is the hard part**, model routing is the easy part.
- The dominant pattern is a **central supervisor/coordinator** that decomposes tasks, delegates to **specialized sub-agents**, and synthesizes results — not peer-to-peer negotiation.
- **6 core patterns** dominate: Sequential, Supervisor, Fan-out, Debate, Dynamic Handoff, Graph (DAG). The right one depends on task predictability and coordination cost.
- **Context isolation per sub-agent** (each sees only task+shared context, not full conversation) prevents context pollution and reduces token waste — this is the most critical design decision.
- **MCP (Model Context Protocol)** handles agent→tool communication (tools as resources). **A2A (Agent2Agent)** handles agent→agent communication. These are complementary, not competing. Runie already has MCP integration; A2A is a future direction.
- **Multi-model routing** is well-solved by LiteLLM-style cascades (fast→slow based on confidence), cost-aware Pareto routing, or task-specific static assignment. Runie's `runie-provider` crate already has model selection; the question is how to make it agent-aware.
- **Human-in-the-loop** is not optional at scale — it must be designed in, not bolted on. Checkpoint interrupts, approval gates, and escalation tiers are the standard approaches.
- **The worst failure modes** are: infinite loops (handoff loops, debate loops), context loss across transfers, and token duplication (72-86% in naive multi-agent systems).
- Runie's existing `/spawn` and `Orchestrator` actor are a solid foundation. The gap is: **sequential/parallel task orchestration**, **agent-aware model routing**, and **structured inter-agent result passing**.

---

## 1. What the Research Landscape Looks Like

### 1.1 The Frameworks and Their Philosophies

| Framework | Core Metaphor | State Model | Best For | Runie's Closest Analog |
|-----------|-------------|-------------|----------|------------------------|
| **LangGraph** | StateGraph (DAG) | Explicit shared state | Complex workflows, cycles | `runie-core` event bus |
| **CrewAI** | Role-based Crew | Task output → next agent | Business automation | `/spawn` subagent |
| **OpenAI Agents SDK** | Handoff chain | Conversation history | Lightweight routing | `Orchestrator` actor |
| **AutoGen** | Pub/sub + Topic | CloudEvents | Research, negotiation | `EventBus` |
| **Claude Code** | Orchestrator + subagents | Isolated per-agent context | Coding agents | `runie-agent` |
| **Kimi Code** | Main→subagent swarm | Isolated, merged results | Parallel tasks | `/spawn` |
| **OpenCode** | Session + System Context | Per-session context isolation | TUI coding | `runie-agent` |
| **gptme** | File leases + message bus | File-based claiming | Multi-developer agents | — |
| **CowAgent** | Channel routing | Memory + knowledge base | Modular harness | Skills |
| **SuperAGI** | Toolkit + concurrent agents | Agent memory | Parallel execution | `ToolActor` |
| **LiteLLM** | LLM gateway | Per-request routing | Multi-model proxy | `runie-provider` |
| **Factory/Droid** | Coordinator + droids | Per-droid + shared | Enterprise dev | — |
| **Microsoft Agent Framework** | A2A + MCP | CloudEvents + topics | Production multi-agent | Future |

### 1.2 The Six Orchestration Patterns

Every framework uses some combination of these:

```
Pattern 1: Sequential Pipeline
  A → B → C → D
  Linear, deterministic, easy to debug. Good for well-defined multi-step tasks.
  Used by: CrewAI sequential, Claude Code sequential workflows

Pattern 2: Supervisor / Orchestrator
           ┌──────────────────┐
           │    Supervisor    │
           │  (central coord) │
           └────────┬─────────┘
       ┌───────────┼───────────┐
       ▼           ▼           ▼
    Worker A    Worker B    Worker C
  Central coordinator decomposes tasks, assigns to specialists, aggregates results.
  Used by: LangGraph supervisor, OpenAI Agents SDK, Factory/droids, Claude Code operator

Pattern 3: Fan-out / Parallel
         ┌──────────┐
         │ Splitter │
         └────┬─────┘
       ┌──────┼──────┐
       ▼      ▼      ▼
    Worker  Worker  Worker
       └──────┼──────┘
              ▼
         ┌────────┐
         │ Merge  │
         └────────┘
  Parallel execution of independent tasks, results aggregated.
  Used by: LangGraph parallel nodes, Claude Code split-and-merge

Pattern 4: Multi-Agent Debate / Maker-Checker
         Agent A ←→ Agent B ←→ Agent C
         (generator)  (critic)   (evaluator)
  Agents challenge each other, iterate toward consensus. Research shows 70% hallucination
  reduction vs single-agent but sycophancy cascading is the hard failure mode.
  Used by: AutoGen GroupChat, Microsoft Agent Framework

Pattern 5: Dynamic Handoff
    A → B → C → A (loop risk!)
  Each agent decides whether to handle or transfer. Emergent routing.
  40% faster resolution in customer support (HCLTech). Failure: infinite loops.
  Used by: OpenAI Agents SDK handoffs, Kimi Code AgentSwarm

Pattern 6: Graph Composition (DAG/Cycles)
         ┌───────┐
         │ Node  │ ← Agent or tool
         └───┬───┘
       ┌─────┴─────┐
       ▼           ▼
    ┌─────┐     ┌─────┐
    │     │     │     │
    └──┬──┘     └──┬──┘
       └─────┬─────┘
             ▼
         ┌───────┐
  Cycles allowed for revision loops. State is explicit and typed.
  Used by: LangGraph StateGraph, Semantic Kernel
```

### 1.3 The Two Protocols: MCP + A2A

These are **complementary**, not competing:

```
MCP (Model Context Protocol) — Anthropic, 2024
  Purpose: Agent → Tool/Data
  Scope: How an agent connects to external tools, databases, files
  Analogy: USB-C for AI — standardized tool access
  Status: Widely adopted (Google, OpenAI, Microsoft, Anthropic all support)
  Runie: Already has MCP integration (tasks/adopt-pulldown-cmark.md)

A2A (Agent2Agent Protocol) — Google, April 2025
  Purpose: Agent → Agent
  Scope: How agents discover each other, negotiate tasks, share results
  Key concepts:
    - Agent Cards: JSON metadata for capability discovery
    - Task Management: structured task lifecycle
    - Long-running support: hours/days tasks with SSE streaming
    - Built on: HTTP + JSON-RPC + SSE (no new infrastructure)
    - Backers: 50+ companies including Microsoft, Salesforce, SAP, ServiceNow
  Status: Growing adoption; Microsoft adopted it for Azure AI Foundry + Copilot Studio
  Relevance: Foundation for future Runie↔external agent interop
```

### 1.4 Multi-Model Routing Strategies

Multi-model is **orthogonal** to multi-agent — every framework handles it differently:

```
Strategy 1: Cascade (fast→slow on low confidence)
  Query → SmallModel → confidence < threshold? → LargeModel
  Cheapest tokens first, escalate only when needed.

Strategy 2: Task-Specific Static Assignment
  | Task Type          | Recommended Model           |
  |--------------------|------------------------------|
  | Planning/Architecture| Claude Opus, Gemini 2M ctx   |
  | Code generation     | Claude Sonnet, GPT-4.1        |
  | Research/search     | Perplexity Sonar, GPT-4       |
  | Testing             | GPT-4, Claude Sonnet           |
  | Documentation       | Claude models                 |
  | Cost-sensitive ops | DeepSeek, Kimi, MiniMax       |

Strategy 3: Cost-Aware Pareto Routing (MoMA paper, 2025)
  Balance cost and performance under budget constraint.
  Route to best model available within budget.

Strategy 4: Model Fallback Chains
  "claude-opus-4-6" → "kimi-k2.5" → "gpt-5.4" → "glm-5"
  Provider-agnostic chains with automatic failover.

Strategy 5: LiteLLM-style Routing Groups
  Routing groups with different strategies per group:
    latency-sensitive: latency-based-routing
    cost-optimized: cost-based-routing
    balanced: simple-shuffle
```

### 1.5 Key Failure Modes (from production deployments)

| Failure Mode | Cause | Mitigation |
|-------------|-------|-----------|
| **Infinite handoff loops** | No central ownership; everyone keeps deferring | Max transfer count, escalation at N loops |
| **Context loss on transfer** | Full context too large; summarization degrades quality | Structured context passing with schema |
| **Token duplication** | 72-86% redundancy in naive multi-agent | Shared context projection, selective admission |
| **Sycophancy cascading** | Agents agree with majority even when wrong | Limit debate rounds, use critic model |
| **Single point of failure** | Central supervisor dies | Checkpoint + resume; distributed orchestration |
| **Non-deterministic routing** | Same input → different agent chain | Deterministic routing rules; log all decisions |

---

## 2. Conceptual Vision for Runie

### 2.1 Where Runie Stands Today

From `docs/SPEC.md` and `docs/CONTEXT.md`:

- ✅ **`/spawn`** — linear subagent, runs one at a time after parent completes
- ✅ **`Orchestrator` actor** — designed for "future sequential and parallel flow orchestration"
- ✅ **`runie-provider`** — multi-model support via provider abstraction
- ✅ **MCP integration** — tool access standardized
- ✅ **`EventBus`** — pub/sub foundation for actor communication
- ✅ **Session JSONL persistence** — durable state across restarts
- ✅ **ToolActor** — async tool execution model
- ❌ **No task decomposition** — spawn is a flat nested session
- ❌ **No model routing per task** — model selection is global
- ❌ **No structured result passing** — subagent output is raw text
- ❌ **No fan-out / parallel** — sequential only
- ❌ **No A2A** — no agent-to-agent protocol
- ❌ **No checkpoint/HITL** — no human interrupt points

### 2.2 The Vision: Layered Multi-Agent Architecture

The vision is NOT to become LangGraph or CrewAI. It's to extend Runie's existing **Orchestrator** + **EventBus** + **Session** foundations into a principled multi-agent system:

```
┌─────────────────────────────────────────────────────────────────────┐
│                          USER INTERFACE                               │
│         TUI (interactive), CLI (headless), JSON (API)               │
└────────────────────────────┬────────────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────────────┐
│                      ORCHESTRATOR ACTOR                              │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │  Task Board: explicit task queue with dependencies            │  │
│  │  Routing: deterministic rule → agent model + toolset          │  │
│  │  Lifecycle: spawn → monitor → aggregate → complete            │  │
│  │  Checkpoints: human interrupt at configurable boundaries       │  │
│  │  Failure recovery: retry with exponential backoff             │  │
│  └──────────────────────────────────────────────────────────────┘  │
└──────┬─────────────────┬──────────────────┬────────────────────────┘
       │                 │                  │
  ┌────▼────┐     ┌──────▼─────┐    ┌──────▼──────┐
  │ MAIN    │     │  SUBAGENT  │    │  SUBAGENT    │
  │ SESSION │     │  SESSION A │    │  SESSION B   │
  │         │     │  (parallel)│    │  (parallel)  │
  └─────────┘     └────────────┘    └──────────────┘
```

### 2.3 Three Implementation Phases

#### Phase 1: Structured Subagent Orchestration (Next, Low Risk)
Extend `/spawn` from flat nesting to **sequential + fan-out**:

```rust
// Vision: /spawn supports structured input
/spawn [mode: sequential|parallel] <task description>

// Example: fan-out research
/spawn parallel
Research the Rust async ecosystem
  - Task 1: tokio docs
  - Task 2: async-std comparison
  - Task 3: smol + embassy

// Orchestrator aggregates results into a structured report
```

**Key changes:**
- `Orchestrator` actor gains a `TaskBoard` — explicit tasks with status
- Subagent sessions get **structured context injection** (task schema, not raw prompt)
- Fan-out uses `tokio::join!` or structured concurrency
- Results are **typed**, not raw text — JSON schema for task output
- Session JSONL records task lineage (parent → child relationships)

**Why this first**: Minimal new concepts. Reuses existing Orchestrator, EventBus, Session. No protocol changes.

#### Phase 2: Agent-Aware Model Routing (Medium Complexity)
Make model selection **task-aware**, not just global:

```toml
# config.toml
[orchestration]
default_model = "claude-sonnet-4-7"

[orchestration.model_routing]
planning = "claude-opus-4-6"
code_generation = "claude-sonnet-4-7"
research = "openai/gpt-4.1"
fast_lookup = "openai/gpt-4o-mini"
cost_ceiling = 0.10  # per-task budget

[orchestration.task_model_overrides]
"/research" = "perplexity/sonar"
"/docs" = "anthropic/claude-3-5-sonnet"
```

**Key changes:**
- `Orchestrator` resolves model per task type before spawning
- Provider routing in `runie-provider` becomes task-scoped
- Cascade fallback within task (small → large on low confidence)
- Cost tracking per task, not just per session

**Why this second**: Provider abstraction already exists. Just need task-type → model mapping.

#### Phase 3: A2A and External Agent Interop (Future)
Enable Runie to collaborate with external agents:

```rust
// A2A Agent Card (discovered at /.well-known/agent.json)
{
  "name": "runie",
  "description": "Terminal coding agent for Rust/TypeScript projects",
  "capabilities": ["code-generation", "testing", "refactoring"],
  "a2a_url": "http://localhost:7890/a2a",
  "skills": ["rust", "typescript", "tui", "ratatui"]
}

// Runie as A2A server: external agents can delegate tasks to Runie
// Runie as A2A client: can delegate to specialized agents (e.g., Claude Code for reasoning)
```

**Key changes:**
- `runie-server` crate becomes an A2A server
- Agent Card registry for capability discovery
- Task delegation protocol over HTTP + JSON-RPC
- Long-running task support via SSE streaming
- MCP remains for tool access; A2A handles agent→agent

**Why this last**: Requires protocol design, security model, and is lower priority than core orchestration.

### 2.4 Key Design Decisions

#### Context Isolation Strategy
**Decision**: Each subagent sees only:
1. The task description (structured schema)
2. Shared project context (CLAUDE.md-equivalent, via System Context)
3. Permission set (inherited from parent, possibly narrowed)

Each subagent does **NOT** see:
- Full parent conversation history
- Other subagent conversations
- Unrelated task context

This matches Claude Code's isolated subagent boundaries and Kimi Code's AgentSwarm. It's the most token-efficient and prevents cross-task contamination.

#### Result Passing Strategy
**Decision**: Structured JSON schema, not raw text.

```rust
// Subagent outputs a typed result
struct SubagentResult {
    task_id: Uuid,
    status: TaskStatus,       // Completed, Failed, Escalated
    output: serde_json::Value, // Typed output, not raw text
    confidence: f32,           // 0.0-1.0 for routing decisions
    cost_tokens: u64,
    duration_ms: u64,
}

// Orchestrator aggregates with a synthesis prompt
let synthesis = orchestrator.synthesize(results: Vec<SubagentResult>, goal: &str);
```

This prevents the 72-86% token duplication seen in systems that pass raw conversation history between agents.

#### Human-in-the-Loop Strategy
**Decision**: Checkpoint-based, not approval-gate-based.

```
Task lifecycle with checkpoints:

  /spawn parallel research
         │
         ▼
    ┌─────────┐
    │ Phase 1 │ ← auto, no checkpoint
    └────┬────┘
         │ Checkpoint: results ready
         ▼
    ┌─────────────────────────┐
    │  Human review (TUI)     │  ← interrupt here
    │  - Accept results       │
    │  - Ask subagent to redo │
    │  - Escalate to human    │
    └─────────┬────────────────┘
              │
         ┌────▼────┐
         │ Phase 2 │
         └────┬────┘
              │ Checkpoint
              ▼
         Final synthesis
```

Checkpoints are:
- Configurable per task type (`[orchestration.checkpoints]
research = "always"
code_generation = "on_failure_only"
fast_lookup = "never"`)
- Surface in TUI as a blocking panel
- Emitted as `CheckpointReached` events on the bus
- Resumed via `CheckpointApproved` / `CheckpointRejected` events

This matches LangGraph's `interrupt()` pattern but integrated into the existing EventBus.

#### Failure Recovery Strategy
**Decision**: Exponential backoff with task-level retry budget.

```rust
struct TaskConfig {
    max_retries: u8,         // default: 2
    retry_backoff_ms: u64,   // default: 1000
    escalation_threshold: u8, // escalate after N failures
    escalate_to: EscalationTarget, // Human, or fallback model
}
```

Task failures are:
- Emitted as `TaskFailed` events with `error: TaskError` (typed, not string)
- Logged in session JSONL with full lineage
- Aggregated by Orchestrator for synthesis ("3 of 5 research tasks failed")

### 2.5 What NOT to Build

Based on the research, these are premature or wrong for Runie:

| Don't Build | Reason |
|-------------|--------|
| Peer-to-peer agent negotiation | Coordination overhead too high for CLI tool |
| Persistent agent memory (long-term) | Session is the unit; cross-session memory is a separate problem |
| Full LangGraph-style DAG with cycles | Overkill for terminal use; sequential + fan-out covers 80% of cases |
| Full A2A server (Phase 3) | Only needed when Runie collaborates with external agents |
| CrewAI-style role-based crews | Runie's `/spawn` with structured tasks is sufficient |
| Prompt versioning | Not relevant for CLI tool; session is the unit |
| Built-in guardrails | Can be added as skills later |

---

## 3. Reference Architecture (from Research)

### 3.1 Orchestrator Pattern (most applicable to Runie)

```
┌──────────────────────────────────────────────────────┐
│                    Orchestrator                        │
│  Responsibilities:                                     │
│  1. Decompose user goal into tasks                    │
│  2. Assign model + toolset per task                   │
│  3. Monitor progress via EventBus                     │
│  4. Aggregate results                                 │
│  5. Handle failures + retries                        │
│  6. Surface checkpoints to user                      │
└───────────────────────┬──────────────────────────────┘
                        │
         ┌──────────────┼──────────────┐
         ▼              ▼              ▼
    ┌─────────┐    ┌─────────┐    ┌─────────┐
    │ Task A  │    │ Task B  │    │ Task C  │
    │ (spawn) │    │ (spawn) │    │ (spawn) │
    └────┬────┘    └────┬────┘    └────┬────┘
         │              │              │
         └──────────────┼──────────────┘
                        ▼
              ┌─────────────────┐
              │   Aggregator    │
              │ (synthesis LLM) │
              └────────┬────────┘
                       │
                       ▼
              ┌─────────────────┐
              │ Final Response  │
              └─────────────────┘
```

### 3.2 Session Lineage Model

Each session tracks parent-child relationships:

```
Session Root (user session)
  │
  ├── Subagent Session A (task_id: uuid, parent: root)
  │     ├── Sub-subagent A1 (task_id: uuid, parent: A)
  │     └── Sub-subagent A2 (task_id: uuid, parent: A)
  │
  ├── Subagent Session B (task_id: uuid, parent: root, parallel_with: [A])
  │
  └── Subagent Session C (task_id: uuid, parent: root, sequential_after: [B])
```

Session JSONL records:
- `TaskSpawned { task_id, parent_id, task_schema, model, tools }`
- `TaskProgress { task_id, phase, tokens_spent }`
- `TaskCompleted { task_id, result: JSON, cost, duration }`
- `TaskFailed { task_id, error, retries }`

### 3.3 Model Routing Cascade

```rust
// In Orchestrator::resolve_model
fn resolve_model(task: &TaskSpec) -> ModelRef {
    // 1. Check explicit override (e.g., /spawn --model claude-opus)
    if let Some(m) = task.model_override { return m; }

    // 2. Check task-type routing table
    if let Some(m) = routing_table.get(&task.task_type) { return m; }

    // 3. Check cost ceiling — use cheapest that fits budget
    let candidates = model_catalog.query_by_cost_ceiling(task.budget);
    if !candidates.is_empty() { return candidates[0]; }

    // 4. Fallback to default
    return config.default_model
}

// Cascade: try small first, escalate on failure
async fn execute_with_cascade(task: Task, mut model: ModelRef) -> Result<TaskResult> {
    loop {
        match execute(task, model).await {
            Ok(r) if r.confidence >= 0.7 => return Ok(r),
            Ok(r) => {
                // Low confidence — escalate to better model if available
                model = model_catalog.upgrade(model)?;
                if model == current { return Ok(r); }
            }
            Err(e) if retries < MAX_RETRIES => {
                retries += 1;
                sleep(backoff_ms * 2_u64.pow(retries)).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

---

## 4. Open Questions

1. **Should subagent sessions share the same EventBus or have isolated buses?**  
   Shared bus enables cross-agent observation; isolated prevents noise.  
   Most frameworks use shared state (LangGraph, CrewAI) but isolated context (Claude Code, Kimi Code).

2. **Should the Orchestrator be an LLM-driven router or a deterministic rule engine?**  
   LLM router is more flexible but non-deterministic and harder to debug.  
   Rule engine is predictable but requires explicit rules for every task type.  
   Hybrid (rules for common paths, LLM for edge cases) seems right.

3. **How should A2A discovery work?**  
   Standard A2A uses `.well-known/agent.json` for capability discovery.  
   For CLI tool, a local registry file (`~/.runie/agents.json`) may be simpler.

4. **Should checkpoint interrupts block the EventBus or use async yields?**  
   Blocking the bus is simpler but stops all progress.  
   Async yields allow other tasks to continue while waiting for human.  
   Most frameworks use interrupt-with-yield.

5. **Should task output schema be enforced or flexible?**  
   Strict JSON schema ensures reliable aggregation.  
   Flexible (any JSON) allows simpler subagent prompts.  
   Best approach: optional schema, fallback to raw text extraction.

---

## 5. Key Reading

| Source | URL |
|--------|-----|
| OpenAI Agents SDK Multi-Agent | https://openai.github.io/openai-agents-python/multi_agent/ |
| LangGraph Multi-Agent Patterns | https://latenode.com/blog/langgraph-multi-agent-orchestration |
| CrewAI Multi-Agent Guide | https://www.crewai.com |
| AutoGen Architecture | https://microsoft.github.io/autogen/ |
| Google A2A Protocol | https://developers.googleblog.com/en/a2a-a-new-era-of-agent-interoperability/ |
| MCP vs A2A | https://www.blott.studio/blog/post/mcp-vs-a2a-which-protocol-is-better-for-ai-agents |
| Claude Code Workflow Patterns | https://www.mindstudio.ai/blog/claude-code-agentic-workflow-patterns |
| Zylos Research: Multi-Agent 2026 | https://zylos.ai/research/multi-agent-orchestration-2025 |
| OmniParser Multi-Agent Survey | https://arxiv.org/pdf/2602.08009 |
| OpenCode Docs | https://opencode.ai/docs/agents/ |
| LiteLLM Routing | https://docs.litellm.ai/docs/routing |
| Factory Missions | https://docs.factory.ai/cli/features/missions |
| Kimi Code AGENTS.md | `/Users/admin/Code/agents/kimi-code/AGENTS.md` |
| OpenCode CONTEXT.md | `/Users/admin/Code/agents/opencode/CONTEXT.md` |
| LangGraph AGENTS.md | `/Users/admin/Code/agents/langgraph/AGENTS.md` |
