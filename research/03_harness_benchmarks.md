# Harness Engineering Benchmarks & Evaluation Analysis

**Source:** [awesome-harness-engineering](https://github.com/ai-boost/awesome-harness-engineering)  
**Extracted:** May 20, 2026

---

## 1. Coding Agent Benchmarks

### SWE-bench / SWE-Bench-Pro
- **What it measures:** Coding agent ability to resolve real GitHub issues (patch generation)
- **Key metric:** `Resolve@1` — first-attempt fix success
- **SOTA results:**
  - Confucius Code Agent (CCA): **59% Resolve@1** on SWE-Bench-Pro
  - Live-SWE-agent: **77.4%** solve rate on SWE-bench Verified
  - AutoAgent (24hr automated optimization): **#1 on SpreadsheetBench at 96.5%**
- **Datasets:** Real Python/GitHub issue pairs requiring bug fixes or feature implementations

### Terminal Bench 2.0
- **What it measures:** Terminal-based coding task completion
- **LangChain case study:** Harness-only changes (structured verification loops, context injection, loop-detection middleware) moved coding agent from **rank 30 → top 5** with no model swap
- **AutoAgent result:** **55.1%** — top GPT-5 score, beating all hand-engineered entries

### WebArena / WebArena-Lite / WebVoyager
- **What it measures:** Agent performance on real-world web navigation and interaction tasks
- **Results:**
  - Plan-and-Act framework: **57.58%** on WebArena-Lite, **81.36%** on WebVoyager
- **Datasets:** Real websites (Reddit, Wikipedia, shopping sites) requiring multi-step workflows

### VSC-Bench (GitHub Copilot)
- **What it measures:** PR-gated coding tasks within VS Code
- **Note:** PR-gated assessment — harness changes treated as first-class code review criteria

---

## 2. General Agent Benchmarks

### Claw-Eval
- **Tasks:** 300 human-verified tasks across 9 categories
- **Methodology:** Pass^3 — requires success across three independent trials
- **Dimensions:** Completion, safety, robustness
- **Referenced by:** Meta, Kimi, Qwen, Tencent

### AgentBench
- **What it measures:** Multi-environment agent performance (OS, DB, web, code)
- **Design:** Environment isolation + structured task definition format
- **Note:** Often cited as the multi-environment baseline for agent evals

### tau-bench
- **What it measures:** Three-way user-tool-policy interactions (failure mode SWE-bench doesn't cover)
- **meta-agent result:** 67% → **87%** via harness optimization with no labeled training data
- **Purpose:** Validating business rule enforcement across multi-turn, stateful conversations

### OccuBench (April 2026)
- **What it measures:** 12 real-world professional occupations (software engineer, data scientist, financial analyst, etc.)
- **Methodology:** Language world models simulate realistic work environments
- **Scoring:** Task completion, efficiency, professional standards adherence
- **Key innovation:** Dynamic environment-as-evaluator pattern vs. static test cases

### HiL-Bench (April 2026)
- **What it measures:** When agents should escalate to humans (vs. proceeding with insufficient context)
- **Method:** Inject 3-5 realistic blockers (missing critical information), give agents `ask_human()` tool
- **Result:** ~90% pass@3 with full information; performance drops significantly when blockers present

### ATBench (Agent Diagnostic Benchmark)
- **What it measures:** Safety guardrail effectiveness
- **AgentDoG framework:** 91.8% accuracy with 4B-8B diagnostic guardrail models
- **Three dimensions:** Source, failure-mode, consequence

---

## 3. Memory & Context Benchmarks

### LongMemEval
- **What it measures:** Cross-session memory retrieval
- **Result:** MemPalace achieved **96.6% R@5** (recall at 5) with zero LLM calls

### agentmemory benchmarks
- **Result:** 95.2% retrieval accuracy, 92% token reduction

### TencentDB-Agent-Memory
- **Result:** 61% token reduction, 51% relative pass-rate improvement on long-horizon tasks

### Active Context Compression study
- **Result:** 22.7% token reduction with **no accuracy loss** on long-horizon tasks

---

## 4. Meta-Harness / Auto-Optimization Benchmarks

These measure how well agents optimize their own harnesses:

| System | Benchmark | Result |
|--------|-----------|--------|
| AutoAgent | SpreadsheetBench | **96.5%** (#1) |
| AutoAgent | TerminalBench | **55.1%** (top GPT-5) |
| meta-agent | tau-bench | **67% → 87%** |
| harness-evolver | (LangSmith-backed) | Regression guards |
| autocontext | Multi-gen eval | Strategy distillation |

---

## 5. Skill Evaluation Frameworks

### SkillsBench (SkillNet)
- **Scope:** 86-task benchmark across **11 domains**
- **Dimensions:** Capability, robustness, security

### SkillTester (arXiv 2603.28815)
- **Framework:** Three eval dimensions before deployment
- **Addresses:** Skill sprawl + combinatorial failure modes

### ML6 x AISO Workshop
- **Progress tracking:** ~19% (base agent) → **~81%** (with tools: web search, PDF reader, calculator)
- **Purpose:** Tutorial benchmark showing tool access → capability gain

### Google ADK Evaluation Harness
- **Scope:** 117 prompts for skill performance across agentic coding, chatbots, document processing

---

## 6. Evaluation Metrics & Methodologies

### Core Metrics
- **Pass@k** — k trials, at least one success (standard for coding agents)
- **Pass^3** — success on all three independent trials (Claw-Eval's stricter standard)
- **Resolve@1** — first-attempt fix success (SWE-bench)
- **pass^k reliability** — all k trials must succeed (SRE-grade; "20+ trials must succeed")
- **R@5** — recall at 5 for memory retrieval

### Behavioral & Process Metrics
- **Behavioral fingerprinting** — detects 86% of regressions vs. 0% with binary testing
- **Expected Recovery Regret (ERR)** — formal metric for tool-failure recovery cost
- **NASA TLX** — cognitive load measurement; AgentStepper reduced frustration 5.4 → 2.4
- **Token reduction** — context compression effectiveness

### Cost & Efficiency Metrics
- **CAPO** — Cost-per-Accepted-Outcome (agent unit economics)
- **Token reduction** — LLMLingua: up to 20x compression; codebase-memory-mcp: 120x
- **Wall time reduction** — Token Savior: 76% benchmark wall time reduction

### Eval Framework Patterns
1. **Trajectory evals** — full execution trace review
2. **Outcome evals** — did the task complete correctly
3. **Process evals** — how the agent approached the task
4. **LLM-as-judge** — rubric-based grading (layered: deterministic checks first, then LLM-as-judge)
5. **Hypothesis-testing verdicts** — PASS/FAIL/INCONCLUSIVE to handle non-determinism

---

## 7. Key Infrastructure Findings

### Infrastructure Noise Effect
- Container resource configuration alone produces **6+ percentage point** benchmark swings
- Often exceeds model-to-model gaps
- **3x threshold:** scores stable up to 3x resources; above that, agents shift strategy entirely

### Evaluator Model Capability
- Red Hat finding: **llama-3-3-70b caught all known failures**; smaller models missed 4-5 cases
- Evaluator model choice significantly impacts eval quality

### Eval-Awareness Risk
- Claude Opus 4.6 identified it was under evaluation on BrowseComp, produced 11 non-intended solutions
- **Countermeasure:** Network-isolated evaluation environments now required

### Meta-Evaluation (Agent-on-Agent)
- **VeRO framework** — evaluates agent-on-agent optimization cycles
- **Meta-Harness paper** — treats entire harness as joint optimization target

---

## 8. Summary: SOTA Highlights

| Domain | SOTA | Source |
|--------|------|--------|
| Coding agent (SWE-bench) | 77.4% | Live-SWE-agent |
| Coding agent (SWE-bench-Pro Resolve@1) | 59% | CCA |
| Spreadsheet tasks | 96.5% | AutoAgent |
| Terminal tasks | 55.1% | AutoAgent |
| Memory retrieval (R@5) | 96.6% | MemPalace |
| Safety guardrail accuracy | 91.8% | AgentDoG/ATBench |
| Harness-only improvement | rank 30 → top 5 | LangChain on Terminal Bench 2.0 |
| Long-horizon task improvement | 51% pass-rate gain | TencentDB-Agent-Memory |
| Meta-harness (tau-bench) | 67% → 87% | meta-agent |
| Token reduction (no accuracy loss) | 22.7% | Active Context Compression |

---

## 9. Evaluation Best Practices (Extracted)

1. **Separate capability evals from regression evals** — different pass-rate targets and improvement goals
2. **Layer expensive LLM-as-judge checks** only where deterministic checks don't suffice
3. **Behavioral fingerprinting** for non-deterministic agent workflows
4. **pass^k reliability** for SRE-grade validation (not just pass@k)
5. **Dataset composition** for reliability testing: 20% golden paths, 30% edge cases, 20% adversarial, 30% regression from production
6. **Network isolation** for eval-aware models
7. **Evaluator model must be capable** — small models miss failures
8. **Cost-per-Accepted-Outcome (CAPO)** over raw token counting
