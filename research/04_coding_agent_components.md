# Coding Agent Components — Architectural Breakdown

Based on: Sebastian Raschka, "Components of a Coding Agent"
Source: https://magazine.sebastianraschka.com/p/components-of-a-coding-agent

---

## Conceptual Hierarchy

| Concept | Role |
|---------|------|
| **LLM** | Raw next-token model (engine) |
| **Reasoning Model** | LLM + intermediate reasoning traces (beefed-up engine) |
| **Agent** | Control loop around model — decides what to inspect, which tools to call, when to stop |
| **Agent Harness** | Software scaffold managing context, tool use, prompts, state, control flow |
| **Coding Harness** | Task-specific harness for software engineering |

Analogy: LLM = engine, reasoning model = more powerful engine, agent harness = transmission/chassis that helps use the engine effectively.

---

## Six Core Components

### 1. Live Repo Context

**Purpose:** Ground each session in actual project state rather than starting blind.

**What it captures:**
- Git repo root, current branch, status, recent commits
- Project layout and key files (README, AGENTS.md, config files)
- Which test commands to run, project-specific conventions

**Why it matters:** "Fix the tests" is not self-contained. The agent needs to know WHERE the tests are, WHAT test command to run, WHAT project conventions apply.

**Data flow:** Workspace facts gathered upfront → summarized as "stable facts" → combined with user request → feeds into prompt.

---

### 2. Prompt Shape and Cache Reuse

**Purpose:** Avoid re-processing static/repeated content on every turn.

**Split of prompt content:**

| Component | Stability | Updates |
|-----------|-----------|---------|
| Stable prompt prefix | High | Tool descriptions, general instructions, workspace summary |
| Session state | Low | Latest user request, recent transcript, short-term memory |

**Why it matters:** Coding sessions are repetitive. Rebuilding everything from scratch each turn wastes compute and introduces inconsistency.

**Key insight:** Cache the prefix; only update the dynamic parts per turn.

---

### 3. Structured Tools, Validation, and Permissions

**Purpose:** Move beyond chat-with-code-suggestions into actual execution capability.

**Tool-use flow:**
1. Model emits structured action (named tool + arguments)
2. Harness validates: known tool? valid args? needs approval? path inside workspace?
3. Optional user approval gate
4. Execute bounded tool
5. Feed result back into loop

**Key constraints enforced:**
- Pre-defined tool list (vs. arbitrary improvisation)
- Path validation (stay inside repo)
- Argument shape validation
- Permission gating for destructive/large operations

**Why it matters:** Without bounds, agents can execute harmful or malformed commands. The harness gives the model less freedom but more reliability.

---

### 4. Context Minimization (Bloat Control)

**Purpose:** Prevent context from growing unbounded across multi-turn sessions.

**Problem:** Repeated file reads, long tool outputs, logs accumulate fast → context exhaustion.

**Strategies:**

| Strategy | Mechanism |
|----------|-----------|
| **Clipping** | Truncate long documents, tool outputs, memory notes, transcript entries |
| **Deduplication** | Older file reads deduplicated so model doesn't see same content repeatedly |
| **Recency weighting** | Recent events kept rich; older events compressed aggressively |
| **Transcript summarization** | Full session history compressed into promptable summary |

**Key insight:** "A lot of apparent model quality is really context quality."

---

### 5. Structured Session Memory

**Purpose:** Durable state management across session resumption.

**Two-layer architecture:**

| Layer | Purpose | Form |
|-------|---------|------|
| **Full transcript** | Complete history for resumption | JSON files on disk |
| **Working memory** | Distilled, task-critical state | Smaller, actively maintained summary |

**Working memory tracks:**
- Current task
- Important files in play
- Recent notes and decisions

**Compact transcript role:** Reconstructs prompt view of recent history for model continuity.

**Working memory role:** Task continuity across turns — explicit maintenance of what matters.

**Both layers:** Updated when new events occur (user request + LLM response + tool output).

---

### 6. Delegation with Bounded Subagents

**Purpose:** Parallelize work by splitting subtasks into separate agent instances.

**Use case:** Main agent needs side answer (e.g., "what file defines this symbol?") without interrupting main flow.

**Delegation challenges:**
- Subagent needs enough context to be useful
- Must avoid duplicate work, file conflicts, unbounded recursion

**Bounding mechanisms:**
- Read-only mode (or restricted permissions)
- Limited recursion depth
- Inherited sandbox and approval setup from main agent

**Key insight:** The tricky design problem is not just how to spawn a subagent but how to bind one.

---

## Data Flow Summary

```
User Request
    │
    ▼
┌─────────────────────────┐
│ 1. Live Repo Context     │ ← Gathers workspace facts upfront
└──────────┬──────────────┘
           │
           ▼
┌─────────────────────────┐
│ 2. Prompt Construction   │ ← Stable prefix + session state + request
│    (with caching)        │
└──────────┬──────────────┘
           │
           ▼
┌─────────────────────────┐
│ Model Inference          │
└──────────┬──────────────┘
           │
           ▼
┌─────────────────────────┐
│ 3. Tool Validation       │ ← Structured action → validation → execution
└──────────┬──────────────┘
           │
           ▼
┌─────────────────────────┐
│ 4. Context Minimization  │ ← Clip, deduplicate, compress older history
└──────────┬──────────────┘
           │
           ▼
┌─────────────────────────┐
│ 5. Memory Update         │ ← Full transcript + working memory updated
└──────────┬──────────────┘
           │
           ▼
    Loop continues or stops
```

---

## Component Interactions

- **Repo Context** feeds into **Prompt Construction** as stable prefix content
- **Prompt Construction** output goes to **Model Inference**
- **Tool Validation** gates **Tool Execution**; results flow back to **Context Minimization**
- **Context Minimization** output becomes part of next **Prompt Construction**
- **Session Memory** is read by **Prompt Construction** and updated by **Context Minimization**
- **Delegation** can spawn parallel loops that each go through context/memory steps

---

## Key Takeaways

1. The harness often matters more than the base model — similar models in different harnesses perform differently
2. Context quality = model quality (underrated)
3. Bounded agents with tool validation are more reliable than unbounded chat
4. Multi-layer memory (working + full transcript) enables both session continuity and prompt efficiency
5. Subagent delegation requires careful bounding to avoid chaos
