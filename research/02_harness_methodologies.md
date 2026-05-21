# Harness Engineering Methodologies & Patterns

Source: [awesome-harness-engineering](https://github.com/ai-boost/awesome-harness-engineering)

---

## Core Definition

**Harness engineering** = discipline of designing scaffolding (context delivery, tool interfaces, planning artifacts, verification loops, memory systems, sandboxes) that surrounds an AI agent.

Key insight (Martin Fowler): every harness component exists because the model can't do it alone — and best harnesses are designed knowing those components will become unnecessary as models improve.

---

## Common Architectural Patterns

### 1. Agent Loop Pattern
- **ReAct** (Thought/Action/Observation) — foundational loop structure
- Components: observe → plan → act → verify
- Extended thinking: `budget_tokens` controls reasoning depth; thinking blocks **must be preserved** when passing tool results
- 60% of open-source LLM agent projects adopt agent loop pattern (scheduler analysis of 70 projects)

### 2. Plan-and-Execute Separation
- **Planner LLM** generates step list once
- **Executor agent** works through steps, replanning only on failure
- Enables specialized models per layer (different sizes, tool access, reasoning budgets)
- Microsoft TaskWeaver: code-first decomposition with planner/executor split + plugin system

### 3. Multi-Agent Orchestration Topologies
- **Subagents** — context isolation prevents cross-domain bloat; 67% fewer tokens vs skills in multi-domain
- **Skills** — shared context, higher token cost
- **Handoffs** — typed schemas required at every boundary
- **Router** — single agent directing to specialized handlers
- Topologies: parallel, sequential, hierarchical, hybrid (AdaptOrch dynamically selects based on task dependency graphs)

### 4. Hierarchical Context Management
- **Five-stage progressive compaction**: budget reduction → snip → microcompact → context collapse → auto-compact
- Claude Code example: subagent isolation with rebuilt permission contexts
- ByteRover: LLM-curated hierarchical context with learned relevance filtering

---

## Evaluation Pipeline Patterns

### Harness-as-Performance-Lever
- LangChain case study: harness-only changes moved coding agent from rank 30 → top 5 on Terminal Bench 2.0 (no model swap)
- Infrastructure configuration swings benchmarks 5+ percentage points (Anthropic 2026 Trends Report)
- "Intent Met" score: 45% → 75% on novel incidents via filesystem-based context engineering (Azure SRE)

### Eval Harness Components
1. **Evaluation gates** — block deployment on failure
2. **Observability instrumentation** — tracks all agent decisions
3. **CI integration** — catch regressions before users

### Key Metrics Tracked
- Resolve@1 (SWE-Bench-Pro)
- Intent Met score
- Token efficiency (context reduction %)
- Task completion rates across context windows

---

## Key Design Principles

### 1. Context Engineering
- Treat context as **finite, curated resource** — not unlimited prompt space
- Three context categories: system prompts, tools/MCP, message history
- Proactive compaction > reactive-at-limit (interrupts mid-subtask, corrupts reasoning state)
- Agent-controlled compression via dedicated tool > harness-controlled threshold

### 2. Tool Design = Agent UX
- Naming, schemas, error surfaces all affect agent performance
- Tool design is harness design — same principles as API design
- Schema-filtered planning subagents enforce behavioral constraints via tool schema

### 3. Entropy Management
- Periodic agents that repair documentation drift
- Verification loops (computational: linters, tests; inferential: LLM-as-judge)
- Loop-detection middleware

### 4. Persistence Semantics
- Interpreter state persistence is **learned semantics** — must match training-time expectations
- Mismatch = 80% missing-variable errors OR 3.5x token overhead
- Checkpointing for multi-day tasks (Meta REA: hibernate-and-wake for 6-hour ML pipelines)

### 5. Defense-in-Depth Safety
- 5-layer defense for terminal-native agents
- Structured permission systems > natural-language permission text
- Input validation → output filtering → tool-risk ratings → human-intervention triggers

### 6. Middleware Pattern
- Six composable hooks: `before_agent`, `before_model`, `wrap_model_call`, `wrap_tool_call`, `after_model`, `after_agent`
- Enables cross-cutting concerns without modifying core agent logic
- Deterministic policy enforcement (PII redaction), dynamic tool injection, mid-task model swapping

---

## Memory & State Patterns

- **Filesystem-as-context**: expose source code, runbooks, schemas, past notes as files; agent uses `read_file`, `grep`, `find`, `shell`
- **Persistent note-taking**: cross-session learning
- **Progressive spec systems**: load only standards/PRDs/session journals relevant to current step
- **Context database**: unified memory + resources + skills through filesystem paradigm

---

## Verification & CI Integration

- **LLM-as-judge**: inferential verification alongside computational checks
- **Test gates**: cross-session state (feature lists, git commits, test results)
- **PR-gated assessment**: harness changes as first-class code review criteria
- **Human-in-the-loop**: governance layers, intervention triggers

---

## Context Compaction Techniques

| Technique | Token Reduction | Notes |
|-----------|-----------------|-------|
| Server-side compaction (Anthropic) | 84% in 100-turn eval | Automatic at window limit |
| LLMLingua (Microsoft) | up to 20x | 3-6x speed gains with LLMLingua-2 |
| Symbol indexing (Token Savior) | 77% | Navigate by pointer vs read whole files |
| Agent-controlled compression | 22.7% | "Focus Agent" decides what to preserve |
| Context-mode MCP server | variable | Sandboxes bulky data outside window |

---

## Reference Architecture (Claude Code Analysis)

Five-stage progressive compaction:
1. Budget reduction
2. Snip
3. Microcompact
4. Context collapse
5. Auto-compact

27-event-type hook pipeline. Subagent isolation with rebuilt permission contexts.

---

## Production Case Studies

| System | Key Pattern | Results |
|--------|-------------|---------|
| Azure SRE Agent | Filesystem-based context engineering | 35k+ incidents, 40.5hrs → 3min MTTR |
| Meta REA | Hibernate-and-wake checkpointing | Multi-day ML pipeline automation |
| Claude Code | Progressive compaction + hook pipeline | Full agent loop portability |
| LangChain coding agent | Structured verification + context injection | Rank 30 → top 5 |

---

## Key Papers/References

- [ReAct (arxiv:2210.03629)](https://arxiv.org/abs/2210.03629) — foundational loop
- [LATS (arxiv:2310.04406)](https://arxiv.org/abs/2310.04406) — MCTS over agent trajectories
- [A Scheduler-Theoretic Framework (arxiv:2604.11378)](https://arxiv.org/abs/2604.11378) — 70 OSS projects analyzed
- [AdaptOrch (arxiv:2602.16873)](https://arxiv.org/abs/2602.16873) — dynamic topology selection
- [Agent Learn Runtime (arxiv:2603.01209)](https://arxiv.org/abs/2603.01209) — persistence semantics
