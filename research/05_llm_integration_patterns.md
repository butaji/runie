# LLM Integration Patterns from Sebastian Raschka's Coding Agent Article

Source: https://magazine.sebastianraschka.com/p/components-of-a-coding-agent

## 1. Prompting Strategies

### Stable + Dynamic Prompt Decomposition
- **Stable prompt prefix**: General instructions, tool descriptions, workspace summary (rebuilt only when needed)
- **Dynamic session state**: Short-term memory, recent transcript, newest user request (updated each turn)
- **Rationale**: Coding sessions are repetitive; avoid re-processing unchanged info

### Workspace-First Context
- Gather repo facts upfront before any task execution
- Include: git branch/status, project docs (README, AGENTS.md), repo layout
- Prevents "starting from zero" on every prompt

### Structured Action Emission
- Model emits structured actions (not free-form prose)
- Harness validates, optionally gates for approval, executes, feeds bounded result back
- Example tools: list files, read file, search, shell command, write file

---

## 2. Model Selection

### Layered Architecture
- **LLM**: Core next-token model (engine)
- **Reasoning Model**: LLM trained/prompted to output intermediate reasoning traces + self-verification (beefed-up engine)
- **Agent**: Control loop wrapping model + tools + memory + environment feedback
- **Coding Harness**: Task-specific scaffold for software engineering

### Harness vs Raw Model
- Vanilla LLMs have similar capabilities; harness often the differentiator
- Example: GLM-5 in a Claude Code-style harness could match GPT-5.4 in Codex
- Some harness-specific post-training helps (e.g., separate Codex variants)

---

## 3. Context Management

### Live Repo Context (Section 1)
- Collect workspace summary: git info, branch, project structure, relevant docs
- Model needs to know WHERE to look, not just WHAT to do
- "Fix the tests" is incomplete without knowing test commands, project layout

### Two-Layer Memory Architecture
- **Working memory**: Small, distilled, explicitly maintained summary of current task, important files, recent notes
- **Full transcript**: Complete history of user requests, tool outputs, LLM responses (durable, JSON on disk)
- **Compact transcript**: Compressed view for prompt reconstruction (vs. working memory for task continuity)

---

## 4. Token Optimization

### Context Compaction Strategies
1. **Clipping**: Shorten long document snippets, tool outputs, memory notes, transcript entries
2. **Transcript reduction/summarization**: Turn full history into smaller promptable summary
3. **Deduplication**: Older file reads kept once; model doesn't see same content repeatedly
4. **Recency bias**: Recent events = richer detail; older events = aggressive compression

### Stable Prefix Caching
- Workspace summary, tool descriptions, agent rules don't change often
- Runtime reuses stable prefix across turns
- Only dynamic parts rebuilt each interaction

### Architecture Insight
> "A lot of apparent 'model quality' is really context quality."

---

## 5. Multi-Turn Conversation Handling

### Session Persistence
- Full transcript stored as JSON files on disk
- Enables session resumption after close
- New events appended to transcript + summarized into working memory

### Progressive Distillation
- Each turn: latest user request + LLM response + tool output → recorded as "new event"
- Event goes into both full transcript (complete) and working memory (distilled)

### Bounded Subagents
- Main agent can delegate side tasks to subagents for parallelization
- Subagent inherits enough context to be useful
- Constrained: read-only, restricted recursion depth
- Use case: symbol lookup, config reading, test failure investigation without blocking main task

---

## 6. Reasoning Approaches

### Agent Loop Pattern
- **Observe**: Collect information from environment
- **Inspect**: Analyze information
- **Choose**: Select next step
- **Act**: Execute it
- Repeat until goal met or stop condition reached

### Reasoning Models (vs Plain LLMs)
- Spend more inference-time compute on intermediate reasoning, verification, search over candidates
- Still LLMs underneath; additional training/prompting
- Harness gets more out of reasoning models via context management

### Self-Verification
- Reasoning models trained to verify themselves more
- Part of why they outperform vanilla LLMs on complex tasks

---

## Key Takeaways

1. **Harness > Raw Model**: Most apparent "model quality" differences are actually context/harness differences
2. **Context is Bottleneck**: Token optimization (caching, clipping, deduplication, compression) is critical for coding agents
3. **Two-Layer Memory**: Separate working memory (small, distilled) from full transcript (durable, complete)
4. **Structured > Free-form**: Pre-defined tools with named inputs > model improvising commands
5. **Stable Prefix Reuse**: Rebuild only what changes; cache everything else
6. **Recency Bias**: Compress older info aggressively; keep recent context rich
7. **Subagent Delegation**: Parallelize bounded subtasks without overwhelming main agent context
