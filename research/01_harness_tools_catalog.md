# Awesome Harness Engineering — Tools & Frameworks Catalog

**Source:** https://github.com/ai-boost/awesome-harness-engineering  
**Cataloged:** May 2026  
**Total Tools:** ~90+ GitHub repos + 30+ articles/papers

---

## Categories Overview

| Category | Count | Description |
|---|---|---|
| Task Runners & Orchestration | ~25 | Frameworks for multi-agent coordination, workflow management |
| Skills & MCP | ~20 | Protocol standards, skill frameworks, tool integration |
| Memory & State | ~12 | Cross-session persistence, memory architectures |
| Demo Harnesses | ~20 | Reference implementations of complete agent harnesses |
| Security, Sandbox & Permissions | ~18 | Isolation, authorization, guardrails |
| Evals & Verification | ~12 | Benchmark harnesses, testing frameworks |
| Context Delivery & Compaction | ~15 | Context management, compression, prompt optimization |
| Observability & Tracing | ~10 | Tracing, monitoring, debugging tools |
| Debugging & Developer Experience | ~10 | Developer tools for agent debugging |
| Human-in-the-Loop | ~8 | HITL patterns, approval workflows |
| Generators & Meta-Harnesses | ~12 | Self-improving harness frameworks |
| Agent Loop | ~8 | Core loop patterns, ReAct implementations |
| Planning & Task Decomposition | ~10 | Task planning, decomposition frameworks |
| Tool Design | ~8 | Tool interface design, structured output |
| Tutorials & Educational | ~10 | Learning resources, workshops |

---

## 🎓 Tutorials & Educational

| Name | Description | Link | Purpose |
|---|---|---|---|
| Learn Harness Engineering | Project-based course on designing Codex/Claude Code environments | https://walkinglabs.github.io/learn-harness-engineering | Learn harness engineering from first principles |
| AISO-workshop | 3-hour hands-on workshop building AI agent with Google ADK | https://github.com/ml6team/AISO-workshop | Understand tool access → capability gains |
| workshop-mastracode | 11-topic curriculum on production coding-agent internals | https://github.com/mastra-ai/workshop-mastracode | Production harness internals walkthrough |
| Building Governed AI Agents | Cookbook: policy-as-code guardrails, observability, eval-driven design | https://developers.openai.com/cookbook/examples/partners/agentic_governance_guide | Build governance into infrastructure |
| claude-cookbooks | Anthropic's official notebooks: orchestrator-worker, parallel tool calling, PTC | https://github.com/anthropics/claude-cookbooks | Reference implementation of orchestration patterns |
| smolagents | Minimal agent library (~1k lines core code), readable in an afternoon | https://github.com/huggingface/smolagents | Alternative loop design (code-agent pattern) |
| mini-coding-agent | Pure-Python harness with 6 core components in one file | https://github.com/rasbt/mini-coding-agent | Understand coding agent loop from scratch |
| learn-claude-code | Step-by-step deconstruction of Claude Code harness | https://github.com/shareAI-lab/learn-claude-code | How agent loop, tools, skills compose |
| awesome-agent-harness | Curated list: Full Lifecycle Platforms, Task Runners, Agent Runtimes | https://github.com/AutoJunjie/awesome-agent-harness | Complementary curated reference |
| Skill Issue: Harness Engineering for Coding Agents | Practitioners' guide: system prompts, MCP, skills, hooks | https://www.humanlayer.dev/blog/skill-issue-harness-engineering-for-coding-agents | Harness configuration for coding agents |

---

## 🔄 Agent Loop

| Name | Description | Link | Purpose |
|---|---|---|---|
| ReAct | Foundational paper: Thought/Action/Observation loop | https://arxiv.org/abs/2210.03629 | Define agent loop structure |
| LangGraph — Low Level Concepts | Directed graph with typed state, conditional edges, checkpointing | https://langchain-ai.github.io/langgraph/concepts/low_level/ | Loop control flow engineering |
| Confucius Code Agent (CCA) | Meta/Harvard coding agent: AX/UX/DX perspectives, 59% Resolve@1 on SWE-Bench-Pro | https://github.com/facebookresearch/cca-swebench | Production-grade coding agent |
| deepclaude | Claude Code loop ported to DeepSeek V4 Pro | https://github.com/aattaran/deepclaude | Backend-agnostic harness evidence |
| The Design Space of Today's and Future AI Agent Systems | Claude Code architecture reverse-engineered: 5-stage compaction, 27-event hook pipeline | https://arxiv.org/abs/2604.14228 | Understand Claude Code internals |

---

## 🗺️ Planning & Task Decomposition

| Name | Description | Link | Purpose |
|---|---|---|---|
| TaskWeaver | Code-first decomposition: planner/executor split, plugin system | https://github.com/microsoft/TaskWeaver | Plan-then-execute with stateful tracking |
| LATS: Language Agent Tree Search | Reasoning/acting/planning via Monte Carlo Tree Search | https://arxiv.org/abs/2310.04406 | Tree search for agent trajectories |
| Agyn | Multi-agent team: planner, coder, reviewer, executor coordination | https://arxiv.org/abs/2602.01465 | Heterogeneous agent teams |
| Plan-and-Act | Planner generates steps once; executor works through it | https://arxiv.org/abs/2503.09572 | Separate planning from execution |
| AdaptOrch | Dynamically selects orchestration topology based on task dependency graphs | https://arxiv.org/abs/2602.16873 | Topology as harness-level lever |
| Task-Decoupled Planning (TDP) | Supervisor decomposes into dependency graph; Planner/Executor solve independently | https://arxiv.org/abs/2601.07577 | Localized replanning without cascade |

---

## 📦 Context Delivery & Compaction

| Name | Description | Link | Purpose |
|---|---|---|---|
| LLMLingua | Microsoft prompt compression (up to 20x, minimal loss) | https://github.com/microsoft/LLMLingua | Context compression preprocessing |
| context-mode | MCP server: intercepts raw tool output, BM25 retrieval | https://github.com/mksglu/context-mode | Sandbox bulky data outside context |
| Token Savior | Indexes codebases by symbol (functions, classes, call graphs) | https://github.com/Mibayy/token-savior | Navigate by pointer, not whole files |
| Trellis | Progressive spec system replacing bloated CLAUDE.md | https://github.com/mindfold-ai/Trellis | Vendor-agnostic harness config |
| OpenViking | ByteDance context database: memory/resources/skills via filesystem paradigm | https://github.com/volcengine/OpenViking | Hierarchical context delivery |
| DESIGN.md | Machine-readable design tokens + markdown for visual identity | https://github.com/google-labs-code/design.md | Structured design constraints for agents |
| codebase-memory-mcp | Tree-sitter AST analysis, persistent knowledge graph, 66 languages | https://github.com/DeusData/codebase-memory-mcp | Sub-millisecond code intelligence |
| Mirage | Mount S3/Slack/Gmail/Redis/GitHub as virtual filesystem | https://github.com/strukto-ai/mirage | Unified backend via bash commands |
| dirac | Hash Anchored edits, massively parallel ops, AST manipulation | https://github.com/dirac-run/dirac | Surgical context curation |
| Semble | Natural-language code search, ~98% token reduction | https://github.com/MinishLab/semble | Replace grep+read with NL retrieval |
| A-RAG | Expose 3 retrieval tools (keyword, semantic, chunk read) as tool calls | https://arxiv.org/abs/2602.03442 | RAG as agent tool, not preprocessing |
| ByteRover | LLM-curated hierarchical context management | https://arxiv.org/abs/2604.01599 | Model-curated active memory |

---

## 🔧 Tool Design

| Name | Description | Link | Purpose |
|---|---|---|---|
| outlines | Constrain token sampling via regex/CFG/JSON Schema at decode time | https://github.com/dottxt-ai/outlines | Structured output without fine-tuning |
| instructor | Pydantic models → structured LLM extraction with retry/validation | https://python.useinstructor.com/ | Type-safe tool output parsing |
| SkillTester | Benchmark agent skills: capability, robustness, security | https://arxiv.org/abs/2603.28815 | Evaluate skill quality before deploy |
| AutoHarness | Auto-synthesize runtime constraint harnesses from tool schemas | https://arxiv.org/abs/2603.03329 | Learned behavioral guardrails |
| EigentSearch-Q+ | Deep-research agents: dedicated reasoning tools (plan, search, extract, analyze) | https://arxiv.org/abs/2604.07927 | Structured cognitive scaffolding |
| tui-use | Programmable TUI interaction for REPLs, debuggers, ncurses | https://github.com/onesuper/tui-use | Interactive CLI tool surface |

---

## 🔌 Skills & MCP

| Name | Description | Link | Purpose |
|---|---|---|---|
| Model Context Protocol | Anthropic's open protocol for agent-tool connectivity | https://modelcontextprotocol.io/introduction | Standardize agent-tool connections |
| MCP servers | Official reference implementations: GitHub, Slack, Postgres, Puppeteer | https://github.com/modelcontextprotocol/servers | Correct MCP server structure |
| playwright-mcp | Browser automation via accessibility tree snapshots | https://github.com/microsoft/playwright-mcp | Structured browser automation |
| A2A Protocol | Google agent-to-agent: JSON-RPC, Agent Card discovery | https://github.com/a2aproject/A2A | Cross-framework agent interoperability |
| MCP Inspector | Interactive debugging UI for MCP servers | https://github.com/modelcontextprotocol/inspector | Validate MCP server responses |
| Composio | Wrap 250+ SaaS APIs as agent-ready actions with OAuth | https://github.com/ComposioHQ/composio | One-line authenticated tool import |
| AG-UI | Event-driven protocol: agent→frontend streaming, HITL interrupts | https://github.com/ag-ui-protocol/ag-ui | Real-time agent-to-UI communication |
| Microsoft Skills Framework | Define, version, distribute agent skills cross-platform | https://github.com/microsoft/skills | Skills as deployment artifacts |
| SkillNet & SkillsBench | 86-task benchmark across 11 domains for skill evaluation | https://github.com/skillmatic-ai/awesome-agent-skills | Standardized skill evaluation |
| Agent Toolkit for AWS | MCP servers/skills for AWS resource provisioning | https://github.com/aws/agent-toolkit-for-aws | Cloud infrastructure as agent primitives |
| agentic-stack | Portable `.agent/` folder: cross-tool harness layer | https://github.com/codejunkie99/agentic-stack | Vendor lock-in prevention |
| mcp-agent | Production framework: composable workflows, observability, provider routing | https://github.com/lastmile-ai/mcp-agent | Coherent MCP-based agent harness |
| vurb.ts | TypeScript MCP servers with PII redaction, workflow-gated visibility | https://github.com/vinkius-labs/vurb.ts | Safe-by-default MCP server authoring |

---

## 🛡️ Permissions & Authorization

| Name | Description | Link | Purpose |
|---|---|---|---|
| OWASP LLM06:2025 | Excessive agency risk: over-provisioned functions, missing approvals | https://genai.owasp.org/llmrisk/llm062025-excessive-agency/ | Permission audit checklist |
| Claude Agent SDK — Configure Permissions | 5-layer eval: hooks → deny → mode → allow → canUseTool | https://platform.claude.com/docs/en/agent-sdk/permissions | Permission architecture reference |
| Authorization and Governance for AI Agents | Microsoft: PEP + PDP with Entra protection | https://techcommunity.microsoft.com/blog/microsoft-security-blog/authorization-and-governance-for-ai-agents-runtime-authorization-beyond-identity/4509161 | Deterministic authorization decisions |
| IETF draft-klrc-aiagent-auth | Standards-track: SPIFFE-style IDs, OAuth 2.0, DPoP | https://datatracker.ietf.org/doc/draft-klrc-aiagent-auth/ | Cross-domain agent auth |
| Nango | Pre-built OAuth for 700+ APIs, auto token refresh | https://nango.dev | Authentication layer for agents |
| AgentDoG | Diagnostic guardrail framework: 3D risk taxonomy, ATBench | https://arxiv.org/abs/2601.18491 | Safety diagnosis, not just safe/unsafe |
| Open Agent Passport (OAP) | Pre-action authorization with cryptographically signed audit | https://arxiv.org/abs/2603.20953 | 53ms enforcement, adversarial tested |

---

## 🧠 Memory & State

| Name | Description | Link | Purpose |
|---|---|---|---|
| Letta (MemGPT) | Three-tier memory: core/archival/recall | https://github.com/letta-ai/letta | Stateful agent architecture |
| mem0 | Universal memory layer, cross-session retention | https://github.com/mem0ai/mem0 | Drop-in persistent memory |
| Stash | 8-stage consolidation pipeline, MCP server built-in | https://github.com/alash3al/stash | Self-hosted memory without cloud |
| TencentDB-Agent-Memory | 4-tier pipeline: Conversation→Atom→Scenario→Persona, 61% token reduction | https://github.com/Tencent/TencentDB-Agent-Memory | Hierarchical memory for coding agents |
| Zep | Auto summarization, entity extraction, semantic search | https://github.com/getzep/zep | Long-session context overflow solution |
| engram | Single Go binary, SQLite+FTS5, 18 MCP tools | https://github.com/Gentleman-Programming/engram | Agent-agnostic zero-dependency memory |
| MemPalace | Palace architecture (wings/rooms/drawers), 96.6% R@5 | https://github.com/MemPalace/mempalace | Local-first semantic retrieval |
| agentmemory | 95.2% retrieval accuracy, 92% token reduction, cross-agent | https://github.com/rohitg00/agentmemory | Portable cross-session memory |
| claude-memory-compiler | Session → self-evolving knowledge base | https://github.com/coleam00/claude-memory-compiler | Trace-driven memory evolution |
| MAGMA | Four orthogonal graphs: semantic/temporal/causal/entity | https://arxiv.org/abs/2601.03236 | Multi-graph memory architecture |
| GAAMA | Graph augmented associative memory for agents | https://arxiv.org/abs/2603.27910 | Hybrid graph+embedding retrieval |

---

## ⚙️ Task Runners & Orchestration

| Name | Description | Link | Purpose |
|---|---|---|---|
| LiteLLM | Unified proxy: 100+ LLM providers, retry/fallback, OTEL | https://github.com/BerriAI/litellm | Provider resilience, model swapping |
| LangGraph | Graph-based state machine, supervisor/subagent, checkpoint persistence | https://github.com/langchain-ai/langgraph | Multi-agent orchestration standard |
| OpenAI Agents SDK | Handoffs and guardrails; Swarm successor | https://github.com/openai/openai-agents-python | Lightweight multi-agent framework |
| Google ADK | Multi-agent orchestration, tool registration, session state, eval | https://github.com/google/adk-python | Code-first agent framework |
| AutoGen | Microsoft: AgentChat, tool integration, termination, HITL | https://github.com/microsoft/autogen | Large-scale multi-agent design |
| CrewAI | Dual-layer: Crew (autonomous) + Flow (deterministic) | https://github.com/crewAIInc/crewAI | Mix autonomous and scripted execution |
| PydanticAI | Type-safe, Pydantic models throughout, dependency injection | https://github.com/pydantic/pydantic-ai | Type-safe agent framework |
| OmniRoute | Intelligent routing, 40-60% token cost reduction | https://github.com/diegosouzapw/OmniRoute | Cost-aware model selection |
| AgentScope Runtime | Secure sandbox execution, durable agent serving | https://github.com/agentscope-ai/agentscope-runtime | Runtime concerns directly addressed |
| Vercel AI SDK | 20M+ downloads, ToolLoopAgent, DevTools, MCP | https://github.com/vercel/ai | TypeScript AI toolkit standard |
| Mastra | 22K+ stars, 40+ providers, workflows, RAG, eval | https://github.com/mastra-ai/mastra | Opinionated TypeScript alternative |
| open-multi-agent | Task DAG decomposition, parallelization, 3 deps | https://github.com/JackChen-me/open-multi-agent | Lightest production-grade harness |
| Symphony | Monitor issue tracker, per-issue workspaces, proof-of-work artifacts | https://github.com/openai/symphony | Work management orchestration |
| Harmonist | Deterministic constraints via mechanical gates, not prompts | https://github.com/GammaLabTechnologies/harmonist | Model-proof constraint enforcement |
| Hive | Deterministic DAGs, state persistence, crash recovery | https://github.com/aden-hive/hive | Production agent workloads |
| thClaws | Native-Rust, four surfaces, sovereign-by-design | https://github.com/thClaws/thClaws | Offline-capable unified harness |
| sandcastle | Docker/Podman/Firecracker isolation as primitive | https://github.com/mattpocock/sandcastle | Lightweight sandboxed orchestration |
| Temporal | Persistent workflows, agentic handshake protocol | https://temporal.io/blog/orchestrating-ambient-agents-with-temporal | Distributed systems for agents |
| Microsoft Agent Framework 1.0 | Unified Semantic Kernel + AutoGen, graph orchestration | https://devblogs.microsoft.com/agent-framework/microsoft-agent-framework-version-1-0/ | Enterprise .NET/Python framework |

---

## ✔️ Verification & CI Integration

| Name | Description | Link | Purpose |
|---|---|---|---|
| promptfoo | YAML-driven LLM testing, LLM-as-judge, assertion DSL, CI integration | https://github.com/promptfoo/promptfoo | Agent output regression tests |
| AgentBench | Multi-environment benchmark: OS, DB, web, code | https://github.com/THUDM/AgentBench | Environment isolation design reference |
| Testing Agent Skills Systematically | Four eval dimensions, JSONL trace capture, rubric grading | https://developers.openai.com/blog/eval-skills | Skill regression testing framework |
| Agent Evaluation Readiness Checklist | 33-item checklist: error taxonomy, grader specialization | https://blog.langchain.com/agent-evaluation-readiness-checklist/ | Eval lifecycle framework |
| Evaluating Skills | Claude Code: 82% with curated skills vs 9% without | https://blog.langchain.com/evaluating-skills/ | Skill coverage testing |
| AgentAssay | Behavioral fingerprinting: 86% regression detection | https://arxiv.org/abs/2603.02601 | Non-deterministic workflow testing |
| Eval-Driven Development | Red Hat: 8-stage maturity, DeepEval, LLM-as-judge | https://developers.redhat.com/articles/2026/03/23/eval-driven-development-build-evaluate-ai-agents | Cost-aware continuous monitoring |
| Agent Evaluation Framework 2026 | Multi-environment baselines, domain benchmarks, NIST standards | https://galileo.ai/blog/agent-evaluation-framework-metrics-rubrics-benchmarks | Standardized eval dimensions |

---

## 👁️ Observability & Tracing

| Name | Description | Link | Purpose |
|---|---|---|---|
| OpenLLMetry | OpenTelemetry instrumentation for LLM calls | https://github.com/traceloop/openllmetry | OTEL ecosystem for agents |
| Arize Phoenix | Self-hostable trace UI, offline replay | https://github.com/Arize-ai/phoenix | Offline agent audit |
| Langfuse | Traces, prompt versions, evals in one tool | https://github.com/langfuse/langfuse | Self-hostable observability |
| Weights & Biases Weave | Call graph capture, dataset versioning, LLM-as-judge | https://github.com/wandb/weave | Experiment tracking + eval |
| Pydantic Logfire | SQL-queryable traces, MCP server for agents | https://github.com/pydantic/logfire | Observable PydanticAI harnesses |
| Helicone | LLM observability proxy, 300+ model pricing database | https://github.com/Helicone/helicone | Cost tracking, session tracing |
| OpenObserve | Infrastructure log/metric unification for agents | https://openobserve.ai/ | Correlate agent decisions with infra |
| Braintrust | Auto-tracing, full-trace search without sampling | https://www.braintrust.dev | Evaluation-first observability |
| Future AGI | Tracing + evals + simulations + guardrails + gateway | https://github.com/future-agi/future-agi | Unified observability plane |

---

## 🐛 Debugging & Developer Experience

| Name | Description | Link | Purpose |
|---|---|---|---|
| AgentOps | Session replay, cost tracking, failure detection, 10+ frameworks | https://github.com/AgentOps-AI/agentops | Multi-agent debugging layer |
| claude-devtools | Per-turn token attribution, subagent trees, cost breakdowns | https://github.com/matt1398/claude-devtools | Claude Code internals visibility |
| Syncause/debug-skill | Runtime evidence, background tracing, fix citation requirements | https://github.com/Syncause/debug-skill | Evidence-based agent repair |
| AgentTrace | Causal graph tracing: 0.12s, 93.6-95.8% accuracy | https://arxiv.org/abs/2603.14688 | Root cause localization |
| TraceCoder | Multi-agent debugging: Instrumentation/Analysis/Repair agents | https://arxiv.org/abs/2602.06875 | Collaborative debugging loop |
| AgentRx | Constraint synthesis, constraint-guided evaluation | https://www.microsoft.com/en-us/research/blog/systematic-debugging-for-ai-agents-introducing-the-agentrx-framework/ | Systematic failure diagnosis |
| AgentPrism | OTEL traces → interactive visualizations (tree, timeline, sequence) | https://github.com/evilmartians/agent-prism | Human-comprehensible trace UIs |
| AgentStepper | Interactive debugger with breakpoints, step-through execution | https://arxiv.org/abs/2602.06593 | Trajectory debugging for agents |

---

## 🧑‍💼 Human-in-the-Loop

| Name | Description | Link | Purpose |
|---|---|---|---|
| aws-samples/sample-human-in-the-loop-patterns | Four HITL patterns: Hook, Tool Context, Step Functions, MCP Elicitation | https://github.com/aws-samples/sample-human-in-the-loop-patterns | Production HITL architecture guide |
| Dify HITL Node | Workflow primitive: suspend, review-edit, approve/reject/escalate | https://github.com/langgenius/dify/discussions/32245 | Native HITL as execution node |
| HITL Protocol | Open standard: HTTP 202 + review URL, 13 end-to-end flows | https://github.com/rotorstar/hitl-protocol | Universal HITL interoperability |
| LangGraph — Human-in-the-Loop | Interrupt, breakpoint, approve patterns | https://langchain-ai.github.io/langgraph/concepts/human_in_the_loop/ | Pause/resume mid-loop |
| AutoGen — Human-in-the-Loop | human_input_mode, UserProxyAgent approval gate | https://microsoft.github.io/autogen/0.2/docs/tutorial/human-in-the-loop/ | Multi-agent approval nodes |
| Claude Agent SDK — Handle Approvals | canUseTool callback, AskUserQuestion, streaming input | https://platform.claude.com/docs/en/agent-sdk/user-input | Complete HITL mechanics |
| HiL-Bench | Benchmark: agents knowing when to ask humans | https://arxiv.org/abs/2604.09408 | When to escalate measurement |
| AutoResearchClaw HITL Co-Pilot | Six intervention modes, SmartPause, Intervention Learning | https://github.com/aiming-lab/AutoResearchClaw | Cost-constrained HITL |

---

## 🔒 Security, Sandbox & Permissions

| Name | Description | Link | Purpose |
|---|---|---|---|
| E2B | Firecracker microVM sandboxes, ~150ms cold start | https://github.com/e2b-dev/E2B | Code execution as harness primitive |
| prompt-injection-defenses | Catalog: input validation, output sanitization, canary tokens | https://github.com/tldrsec/prompt-injection-defenses | Trust boundary hardening |
| OWASP LLM01:2025 | Prompt injection: direct and indirect classification | https://genai.owasp.org/llmrisk/llm01-prompt-injection/ | Threat model definition |
| Daytona | OCI-container sandboxes, sub-90ms startup, LSP support | https://github.com/daytonaio/daytona | Long-lived working directories |
| NeMo Guardrails | Five-layer rails: input, dialog, retrieval, execution, output | https://github.com/NVIDIA-NeMo/Guardrails | Behavioral enforcement |
| LangSmith Sandboxes | MicroVM + kernel isolation + auth proxy + persistent WebSocket | https://blog.langchain.com/introducing-langsmith-sandboxes-secure-code-execution-for-agents/ | Long-running secure execution |
| Cursor Sandbox | Cross-platform: Seatbelt/Landlock/seccomp/WSA2 | https://cursor.com/blog/agent-sandboxing | 40% fewer interruptions via boundary |
| deepsec | Vulnerability scanning as agentic workflow | https://github.com/vercel-labs/deepsec | High-stakes long-running harnesses |
| OpenShell | Policy-driven: Landlock LSM, seccomp BPF, OPA/Rego proxy | https://github.com/NVIDIA/OpenShell | Kernel-level agent enforcement |
| Microsoft Agent Governance Toolkit | Seven packages, 10 OWASP risks, deterministic enforcement | https://github.com/microsoft/agent-governance-toolkit | Enterprise runtime security |
| Cloudflare Dynamic Workers | V8 isolates, milliseconds startup, 100x vs containers | https://blog.cloudflare.com/dynamic-workers/ | Serverless sandbox option |
| Kubernetes Agent Sandbox | K8s-native CRD, gVisor/Kata, "Secure by Default" networking | https://github.com/kubernetes-sigs/agent-sandbox | K8s-integrated isolation |
| Alibaba OpenSandbox | Multi-language SDKs, gVisor/Kata/Firecracker | https://github.com/alibaba/OpenSandbox | Runtime-flexible isolation |
| CubeSandbox | Tencent: sub-60ms via snapshot cloning, eBPF network | https://github.com/TencentCloud/CubeSandbox | Hyperscale cloud sandbox |
| zeroboot | Sub-millisecond VM via COW forking | https://github.com/zerobootdev/zeroboot | Per-action isolation in real-time |
| AI Harness Scorecard | Scores repositories on AI harness safeguards | https://github.com/anthropics/ai-harness-scorecard | Security posture audit |
| The Attack and Defense Landscape of Agentic AI | 128 papers, 51 attacks, 60 defenses, comprehensive survey | https://arxiv.org/abs/2603.11088 | Agent threat modeling reference |

---

## ✅ Evals & Verification

| Name | Description | Link | Purpose |
|---|---|---|---|
| DeepEval | 20+ metrics, pytest integration, CI runner | https://github.com/confident-ai/deepeval | Complete LLM/agent eval framework |
| Claw-Eval | 300 human-verified tasks, Pass^3 methodology, 9 categories | https://github.com/claw-eval/claw-eval | Rigorous community-verified benchmark |
| SWE-bench | Canonical coding agent benchmark | https://www.swebench.com | Verified working standard |
| Inspect AI | UK AI Security Institute eval framework, black-box agent testing | https://github.com/UKGovernmentBEIS/inspect_ai | Safety-grade eval infrastructure |
| tau-bench | User/tool/policy three-way interactions | https://github.com/sierra-research/tau-bench | Business rule enforcement validation |
| Agentic Harness for Real-World Compilers | Specialized: llvm-autofix for compiler bugs | https://arxiv.org/abs/2603.20075 | Domain-specialized harness design |
| VeRO | Evaluate agent-on-agent optimization cycles | https://arxiv.org/abs/2602.22480 | Meta-evaluation infrastructure |
| OccuBench | 12 real-world occupations, language world models | https://arxiv.org/abs/2604.10866 | Professional task evaluation |
| Quantifying Infrastructure Noise | Container config → 6+ percentage point swings | https://www.anthropic.com/engineering/infrastructure-noise | Eval environment variance |

---

## 📋 Templates

| Name | Description | Link | Purpose |
|---|---|---|---|
| AGENTS.md | Project-level agent instructions, conventions, permissions | https://github.com/ai-boost/awesome-harness-engineering/blob/main/templates/AGENTS.md | Agent configuration template |
| PLAN.md | Task planning artifact with milestones and verification gates | https://github.com/ai-boost/awesome-harness-engineering/blob/main/templates/PLAN.md | Planning artifact template |
| IMPLEMENT.md | Implementation log: decisions, deviations, open questions | https://github.com/ai-boost/awesome-harness-engineering/blob/main/templates/IMPLEMENT.md | Tracking template |
| HARNESS_CHECKLIST.md | Pre-production review checklist | https://github.com/ai-boost/awesome-harness-engineering/blob/main/templates/HARNESS_CHECKLIST.md | Production readiness checklist |

---

## 🏭 Generators & Meta-Harnesses

| Name | Description | Link | Purpose |
|---|---|---|---|
| everything-claude-code | 140K+ stars, skills/instincts/memory/continuous learning | https://github.com/affaan-m/everything-claude-code | Production-optimized Claude Code |
| Claude Agent SDK | Programmable API for Claude Code harness | https://platform.claude.com/docs/en/agent-sdk/overview | Programmatic harness control |
| harness-evolver | Multi-agent proposers, LangSmith eval, git worktrees | https://github.com/raphaelchristi/harness-evolver | Autonomous harness improvement |
| auto-harness | Mine failures, optimize iteratively, gate regressions | https://github.com/neosigmaai/auto-harness | Self-improving agentic system |
| Meta-Harness | Treat entire harness as joint optimization target | https://arxiv.org/abs/2603.28052 | Outer-loop harness optimization |
| AutoAgent | Iterates overnight on prompts/configs/routing | https://github.com/kevinrgu/autoagent | Automated harness engineering |
| metaharness | Filesystem-backed search, AGENTS.md as optimizable artifact | https://github.com/SuperagenticAI/metaharness | Improve harness code, not prompts |
| meta-agent | 67%→87% on tau-bench via config rewriting | https://github.com/canvas-org/meta-agent | Lightweight continual optimizer |
| stanford-iris-lab/meta-harness | Stanford IRIS Lab implementation, filesystem search | https://github.com/stanford-iris-lab/meta-harness | Research meta-harness reference |
| autocontext | Multi-generation eval loops, persistent playbooks | https://github.com/greyhaven-ai/autocontext | Recursive self-improving harness |

---

## 🧪 Demo Harnesses

| Name | Description | Link | Purpose |
|---|---|---|---|
| Anthropic Computer Use Demo | Screenshot/bash/text_editor tool interface | https://github.com/anthropics/anthropic-quickstarts/tree/main/computer-use-demo | Screen-based agent reference |
| OpenHands | Runtime/Sandbox, EventStream, Agent Controller three-layer design | https://github.com/OpenHands/OpenHands | Architecturally complete coding agent |
| browser-use | Minimal browser automation: tool registration, DOM injection, error recovery | https://github.com/browser-use/browser-use | Minimal viable harness reference |
| browser-harness | Self-healing: agent writes missing helpers into harness | https://github.com/browser-use/browser-harness | Editable harness that learns |
| SWE-agent | ACI: purpose-built file viewer, search, editor | https://github.com/SWE-agent/SWE-agent | Domain-adapted tool interface |
| Aider | Architect mode: planner/coder split, git-aware tooling | https://github.com/Aider-AI/aider | Multi-file editing reference |
| Open SWE | Composable coding agent, ~15-tool limit, isolated sandbox | https://blog.langchain.com/open-swe-an-open-source-framework-for-internal-coding-agents/ | Production internal coding agent |
| Pipecat | Real-time voice: ASR/LLM/TTS pipeline, sub-800ms | https://github.com/pipecat-ai/pipecat | Voice agent framework |
| AIO Sandbox | Browser + shell + filesystem + MCP + VSCode in Docker | https://github.com/agent-infra/sandbox | All-in-one dev environment |
| langchain-ai/deepagents | Batteries-included: planning, filesystem, shell, sub-agents | https://github.com/langchain-ai/deepagents | General-purpose out-of-box harness |
| OpenCode | 131K+ stars, provider-agnostic, multi-session parallel agents | https://github.com/anomalyco/opencode | Terminal-first coding harness |
| Squad | Multi-agent orchestration built on GitHub Copilot | https://github.com/bradygaster/squad | Repo-native AI team |
| cua | Full-desktop control: macOS/Linux/Windows, background driver | https://github.com/trycua/cua | OS-level computer use |
| desloppify | Score-driven quality improvement workflow | https://github.com/peteromallet/desloppify | Anti-gaming score design |
| HKUDS/OpenHarness | Compact, inspectable: auto-compaction, MCP HTTP transport | https://github.com/HKUDS/OpenHarness | Modular multi-day session harness |
| Live-SWE-agent | 77.4% solve rate via self-evolving scaffold | https://arxiv.org/html/2511.13646v3 | Continuous harness adaptation |

---

## 📚 Related Awesome Lists

| Name | Description | Link | Purpose |
|---|---|---|---|
| Awesome Context Engineering | Context engineering: prompt, RAG, window management | https://github.com/Meirtz/Awesome-Context-Engineering | Context engineering survey |
| awesome-claude-code | Resources specifically for Claude Code users | https://github.com/hesreallyhim/awesome-claude-code | Claude Code ecosystem |
| awesome-mcp-servers | Comprehensive MCP server list | https://github.com/appcypher/awesome-mcp-servers | MCP extensibility |
| awesome-ai-agents | AI agents and frameworks by use case | https://github.com/e2b-dev/awesome-ai-agents | Agent landscape survey |
| awesome-llm-apps | Production LLM apps: RAG, multi-agent, tool-use | https://github.com/Shubhamsaboo/awesome-llm-apps | Application patterns |
| awesome-agent-evolution | Agent evolution, memory, multi-agent, self-improvement | https://github.com/EvoMap/awesome-agent-evolution | Next-generation agent capabilities |
| Picrew/awesome-agent-harness | 150 entries, 84% GitHub projects, 9 categories | https://github.com/Picrew/awesome-agent-harness | Implementation-first reference |
| VoltAgent/awesome-ai-agent-papers | 363+ arXiv papers from 2026 | https://github.com/VoltAgent/awesome-ai-agent-papers | Research tracking |
| bradAGI/awesome-cli-coding-agents | 80+ terminal-native coding agents | https://github.com/bradAGI/awesome-cli-coding-agents | CLI agent catalog |

---

## Key Statistics

- **Total GitHub Repos:** ~90+
- **Papers/Articles:** ~30+
- **Categories:** 15 major categories
- **Top Frameworks by Frequency:** LangGraph, MCP, Claude SDK, AutoGen, CrewAI, Vercel AI SDK
- **Most Referenced Orgs:** Anthropic, Microsoft, Google, OpenAI, LangChain

---

*Generated from awesome-harness-engineering — CC0 Licensed*
