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

1. **Two execution modes: Solo and Team.**
   - **Solo** is the default: one agent turn with the configured model. This is
     today's behavior.
   - **Team** is a per-session toggle. The Orchestrator designs a workflow,
     assigns roles, and executes steps.

2. **User chooses the mode, the machine does the rest.**
   - A UI toggle selects Solo or Team for the session.
   - No manual model tagging or role configuration is required.
   - The Orchestrator aligns with the user via a short Q&A in the Dialog Panel
     before planning.

3. **Orchestrator-Harness Protocol (OHP).**
   - The Orchestrator LLM emits a typed `TeamWorkflow` plan: roles, steps,
     sequential/parallel groups, and model trait preferences.
   - The harness validates the plan, resolves traits to concrete models, and
     executes it.

4. **Model selection by traits.**
   - Traits (`fast`, `capable`, `reasoning`, `cheap`, etc.) are derived by
     relative ranking against connected models.
   - An optional global model priority list lets users override ranking and
     drain underutilized providers.
   - Fallback walks the priority list on rate-limit or quota errors.

5. **Fully isolated subagents with structured JSON results.**
   - Each subagent sees only its own role prompt, task prompt, allowed tools,
     and shared project context.
   - Subagents output JSON matching a per-step schema. Only extracted fields
     flow to downstream steps.
   - This prevents the 72-86% token duplication seen in systems that pass raw
     conversation history between agents.

6. **Subagent sidebar with per-agent feeds.**
   - Active subagents appear in the sidebar next to the main feed.
   - `Ctrl+0` switches to the Orchestrator feed; `Ctrl+1`..`Ctrl+9` switch to
     agent feeds.

## Consequences

- **Positive:** Higher-quality results for complex tasks via specialization and
  parallel research/execution.
- **Positive:** Token-efficient compared to monolithic prompts or raw-history
  passing.
- **Positive:** Users do not need to learn new commands; the mode is a toggle
  and the rest is conversational.
- **Trade-off:** Team mode adds latency and cost because it runs multiple
  subagents and a planning LLM call.
- **Trade-off:** The Orchestrator LLM can produce invalid plans; the harness
  must validate and retry.
- **Trade-off:** Debugging multi-agent workflows is harder than debugging a
  single turn; full plan logging and per-agent feeds are required.
