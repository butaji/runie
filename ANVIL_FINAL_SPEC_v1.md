# Anvil v1.0 Specification
## TUI for Agent Swarms — Terminal-Native Coding Harness

**Version**: 1.0  
**Date**: 2026-05-19  
**Style**: Grok Build — terminal-native, keyboard-first, no excess chrome  
**Philosophy**: Write the glue, not the engine. Ship in 6-8 weeks.

---

## 1. Overview

Anvil is a terminal-native coding harness that orchestrates AI agent swarms. It is not a chat interface, not an IDE plugin, not a web dashboard. It is a **mission control** for coding with multiple models, multiple agents, and strict safety guardrails — all from the terminal.

**Core principle**: The user is the commander, not the typist. Agents are the workforce. The plan is the contract. The stream is the log. The cost HUD is the budget.

---

## 2. Design Language (Grok Build Style)

### Visual Identity

```
┌─ xai/main  jasong/folder/xai ────────────────── 4 agents ↓  $12.40/$100 ─┐
│                                                                          │
│  ◇ Thought for 2.5s                                                       │
│                                                                          │
│  ◆ Edit frontend/apps/website/src/app/(main)/cli/page.tsx              │
│     770      <h3 className="...">Better title</h3>                      │
│     773  +   <p className="text-secondary mx-auto mt-4 max-w-xl ...">   │
│     774  +     A terminal-native experience designed for speed and      │
│     775  +     clarity. No bloat, no noise—just agents, right where    │
│     776  +     you need it.                                             │
│     777  +   </p>                                                       │
│                                                                          │
│  ◇ Thought for 1.2s                                                     │
│                                                                          │
│  ┌─ Plan: Design System Improvement ─────────────────────────────────┐   │
│  │  plan.md                                                        │   │
│  │  1. Audit Current State                                         │   │
│  │     • Inventory existing components across frontend apps        │   │
│  │     • Document design tokens (colors, typography, spacing)      │   │
│  │  2. Establish Design Tokens                                     │   │
│  │     • Create centralized token system                           │   │
│  │     • Add OKLCH color space support                              │   │
│  │  [Reviewing plan]                                                │   │
│  └───────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│  ◇ Waiting on answers for 3 questions                                  │
│  ┌─────────────────────────────────────────────────────────────────────┐ │
│  │ What are the core design principles?                               │ │
│  │ 1 (○) Minimal & terminal-native  Clean, keyboard-first           │ │
│  │ 2 (○) Bold & expressive          Strong visuals, gradients         │ │
│  │ 3 (○) Developer-focused          Code-first aesthetic              │ │
│  │ z ( ) Type your answer here                                       │ │
│  │ [1/3] ↑/↓ navigate · ←/→ question              Enter:select       │ │
│  └─────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  ┌─ Skills ──────────────────────────────────────────────────────────┐  │
│  │  Hooks  │  Plugins  │  Marketplace  │  Skills  │  MCP Servers  [x]│  │
│  │  / to search                                                       │  │
│  │  ▸ rust-check        (local)                                       │  │
│  │  ▸ gcloud-auth       (local)                                       │  │
│  │  ▸ pr-babysit        (local)                                       │  │
│  │  ▸ code-review       (local)                                       │  │
│  │  ▸ xai-grafana-mcp   (local)                                       │  │
│  │  ▸ help              (user)                                        │  │
│  │  ▸ oklch-skill       (user)                                        │  │
│  │  ▸ make-interfaces-feel-better  (user)                             │  │
│  │  ▸ xai-eval          (plugin: eval-development)                  │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
│                                                                          │
│  ┌─ 4 agents ─────────────────────────────────────────────────────────┐  │
│  │  • general   Suggest design improvements    grok-build-latest [29s]│  │
│  │  • general   Review CLI page implementation  grok-build-latest [29s]│  │
│  │  • explore   Check Section component         explore         [29s]│  │
│  │  • explore   Explore website design system   explore         [29s]│  │
│  └─────────────────────────────────────────────────────────────────────┘  │
│                                                                          │
│  > /bt                                                                 │
│    /btw          Ask a side question without interrupting              │
│    /make-interfaces-feel-better  Design engineering principles...        │
│                                                                          │
│  Enter send │ Shift-Tab normal │ ^h home │ ^q quit                     │
└──────────────────────────────────────────────────────────────────────────┘
```

### Design Rules

1. **No decorative panels** — Every pixel serves agent orchestration
2. **Diamond markers** — `◇` for thoughts/reasoning, `◆` for actions/edits
3. **Embedded viewers** — Plans, diffs, and questions render inline, not in separate windows
4. **Keyboard-first** — Every action has a shortcut. Mouse is optional.
5. **Information density** — Like `htop` for coding agents. No whitespace for whitespace's sake.
6. **Status in header** — Repo, branch, agent count, cost — always visible
7. **Command palette** — `>` or `/` triggers the universal command interface
8. **Live timestamps** — Elapsed time updates every second for running agents

---

## 3. Architecture

### The Two-Layer Model

**Layer 1: Agent Task Execution (Structured Concurrency)**
- DAG-based task graphs with explicit join points
- Deterministic cancellation propagation
- Static, inspectable execution graphs
- Built on `dagx` + `tokio::task::JoinSet`

**Layer 2: Model Infrastructure (Actor Model)**
- One actor per model provider (Claude, GPT-4o, Ollama, Gemini, DeepSeek)
- Rate-limited mailboxes, circuit breakers, health metrics
- Independent failure domains
- Built on `tokio` + `tower` middleware

### Why This Split

DAGs coordinate work. Actors manage resources. Agent tasks need coordination (join points, cancellation, debuggable causality). Model APIs need resource management (rate limits, cost tracking, failover). **DAGs for agents, actors for models.**

### System Diagram

```
User Input
    ↓
┌─────────────────────────────────────────────────────────────┐
│  TUI (ratatui) — Mission Control                              │
│  Header · Stream · Panels · Input                             │
└─────────────────────────────────────────────────────────────┘
    ↓
┌─────────────────────────────────────────────────────────────┐
│  Intent Parser — NL → typed Intent                            │
│  "fix auth" → { target: "src/auth.rs", type: "bug", ... }    │
└─────────────────────────────────────────────────────────────┘
    ↓
┌─────────────────────────────────────────────────────────────┐
│  Plan Generator — Static DAG                                  │
│  [User reviews — Ctrl+Enter to approve]                       │
└─────────────────────────────────────────────────────────────┘
    ↓
┌─────────────────────────────────────────────────────────────┐
│  DAG Executor (dagx + tokio)                                  │
│  read_context → generate_code → cargo_check → cargo_test     │
│  Explicit join points. Cancellation propagation.              │
└─────────────────────────────────────────────────────────────┘
    ↓
┌─────────────────────────────────────────────────────────────┐
│  Safety Envelope — Guaranteed enforcement                      │
│  Cost caps · Protected paths · Required tests · Max retries   │
└─────────────────────────────────────────────────────────────┘
    ↓
┌─────────────────────────────────────────────────────────────┐
│  OODA Router (Actor Model)                                    │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐            │
│  │ Claude  │ │ GPT-4o  │ │ Ollama  │ │ Gemini  │            │
│  │ Actor   │ │ Actor   │ │ Actor   │ │ Actor   │            │
│  │ $3/Mtok │ │ $5/Mtok │ │ $0      │ │ $1/Mtok │            │
│  │ 200Kctx │ │ 128Kctx │ │ 8Kctx   │ │ 1Mctx   │            │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘            │
│  Observe → Orient → Decide → Act                            │
└─────────────────────────────────────────────────────────────┘
    ↓
┌─────────────────────────────────────────────────────────────┐
│  models.dev — Canonical Model Database                        │
│  247 models · 18 providers · Live pricing · Capabilities     │
│  https://models.dev/api.json                                  │
└─────────────────────────────────────────────────────────────┘
    ↓
LLM API Response
    ↓
┌─────────────────────────────────────────────────────────────┐
│  rquickjs Runtime — Agent Script Execution                    │
│  anvil.js · Hooks · Skills · Plugins                          │
│  ES2020 · Async/await · Modules · Rust↔JS bridge            │
└─────────────────────────────────────────────────────────────┘
    ↓
Result → Git Commit → Session Log
```

---

## 4. The TUI Layout

### 4.1 Header Bar

Always visible. Always informative.

```
┌─ {repo}/{branch}  {user}/{folder}/{repo} ───────── {N} agents ↓  ${spent}/${budget} ─┐
```

| Element | Display | Action |
|---|---|---|
| `repo/branch` | Current git context | Click/dropdown to switch branches |
| `user/folder/repo` | Working directory | Breadcrumb navigation |
| `N agents ↓` | Active agent count | Dropdown shows swarm status |
| `${spent}/${budget}` | Real-time cost | Red when >80%, pulse when >95% |

**Keyboard**: `^a` opens agent dropdown. `^$` opens cost breakdown.

### 4.2 Agent Stream

The main content area. An **execution log**, not a chat.

**Entry Types**:

| Marker | Type | Behavior |
|---|---|---|
| `◇` | Thought | Reasoning, not action. Gray. Collapsible. Shows elapsed time. |
| `◆` | Edit | File modification. Orange. Always expanded. Unified diff with line numbers. |
| `┌─┐` | Plan | Structured plan viewer. Embedded, scrollable. User can edit inline. |
| `┌─┐` | Question | Human-in-the-loop. Blocks execution until answered. |
| `┌─┐` | Subagent | Parallel agent spawn. Live list with elapsed time. |

**Stream Behavior**:
- Auto-scrolls to latest entry
- `j/k` navigates entries (vim-style)
- `Space` expands/collapses
- Click filename opens in `$EDITOR`
- Entries are immutable once complete (append-only log)

### 4.3 Command Palette

Triggered by `>` or `/`.

```
> /{command}
  /btw              Ask a side question without interrupting
  /plan             Enter plan mode
  /agents           Show agent swarm status
  /models           Open model selector
  /cost             Show cost breakdown
  /pause            Pause all agents
  /resume           Resume agents
  /cancel           Cancel current task
  /spawn            Spawn subagent
```

**Programmable**: Agents register new commands via `anvil.js`:

```javascript
export const commands = {
  "deploy": async (args, ctx) => {
    await ctx.run("cargo build --release");
    return { status: "built", cost: ctx.session.cost };
  }
};
```

### 4.4 Skills Panel

Modal overlay. `Tab` cycles tabs. `Esc` closes. `/` searches.

**Tabs**:
- **Hooks** — lifecycle scripts (pre-task, post-edit, pre-commit)
- **Plugins** — rquickjs extensions from git packs
- **Marketplace** — browse agent packs from git remotes
- **Skills** — prompt fragments and expertise modules
- **MCP Servers** — external tool connections

**Shortcuts**:
- `r` reload
- `Space` expand
- `/` search
- `Tab` cycle tabs
- `Esc` close

### 4.5 Agent Swarm View

Dropdown from header or `/agents`.

```
┌─ 4 agents ────────────────────────────────────────────────────────────┐
│                                                                       │
│  • general   Suggest design improvements    grok-build-latest [29s]  │
│    └─ Waiting on: user approval                                        │
│                                                                       │
│  • explore   Check Section component         explore         [29s]   │
│    └─ Reading: frontend/components/Section.tsx                         │
│                                                                       │
│  [+ Spawn subagent]  [▼ View worktree]  [✕ Cancel]                   │
│                                                                       │
└───────────────────────────────────────────────────────────────────────┘
```

**Each agent**:
- Own git worktree (branch: `anvil/{agent-id}`)
- Own model selection (from OODA router)
- Own cost tracking
- Own safety envelope

**Spawn command**:
```
> /spawn {role} {task} [--model {model}] [--worktree {name}]
```

**Pre-flight check**: Tree-sitter file overlap analysis. If conflict: "These agents would conflict on src/auth.rs. Run sequentially? [Y/n]"

### 4.6 Model Selector

`Ctrl+L` or `/models`. Inline dropdown.

```
> /models
  anthropic/claude-sonnet-4    $3.00/$15.00  200K ctx  ●●●●●  [ACTIVE]
  openai/gpt-4o                $2.50/$10.00  128K ctx  ●●●●○  [ACTIVE]
  ollama/llama3.3               $0.00/$0.00     8K ctx  ●●●○○  [STANDBY]
  google/gemini-1.5-pro         $1.25/$5.00      1M ctx  ●●●●○  [RATE_LIMITED]
  deepseek/deepseek-v3          $0.27/$1.10     64K ctx  ●●●○○  [AVAILABLE]

  ↑/↓ select · Enter confirm · Esc cancel
```

**Health dots**:
- `●●●●●` — Healthy, <100ms
- `●●●●○` — Good, <500ms
- `●●●○○` — Degraded, >1s or recent errors
- `●●○○○` — Critical, circuit breaker open

**Auto-selection**: OODA router decides based on task profile, cost, history. User can override.

### 4.7 Cost HUD

Always in header. Expandable with `/cost` or `^$`.

```
$12.40/$100  [████████░░░░░░░░░░░░]  12%

Breakdown:
  Claude Sonnet 4    $8.20   (66%)  ████████████
  GPT-4o             $3.50   (28%)  █████
  Ollama             $0.00    (0%)  
  Gemini 1.5 Pro     $0.70    (6%)  █

This task:        $0.04   (Claude, 2.1s)
Session total:    $12.40
Monthly budget:   $100.00
Remaining:        $87.60

[⚠ Warning: At current rate, budget will exhaust in 3 days]
```

### 4.8 Safety Checkpoint

Invisible until triggered. Then impossible to ignore.

```
┌─ SAFETY CHECKPOINT ───────────────────────────────────────────────────┐
│                                                                       │
│  Agent impl-a wants to delete 147 lines from src/auth.rs             │
│                                                                       │
│  Diff preview:                                                        │
│    - function oldAuth() { ... }                                       │
│    - function legacyLogin() { ... }                                  │
│    + function newOAuth2() { ... }                                     │
│                                                                       │
│  Risk: HIGH (deleting auth functions without tests passing)          │
│  Confidence: 0.72 (below 0.80 threshold)                               │
│                                                                       │
│  [Y] Approve and continue    [n] Reject and pause    [e] Edit plan  │
│                                                                       │
│  This checkpoint was triggered by: ~/.anvil/hooks/safety.js           │
│                                                                       │
└───────────────────────────────────────────────────────────────────────┘
```

---

## 5. Keyboard Shortcuts

| Key | Action |
|---|---|
| `Enter` | Send command / Confirm selection |
| `Shift+Tab` | Switch to normal mode (navigate stream) |
| `^h` | Home (jump to top) |
| `^q` | Quit |
| `^c` | Cancel current task |
| `^l` | Model selector |
| `^p` | Previous model (cycle) |
| `^$` | Cost HUD |
| `^a` | Agent swarm dropdown |
| `Tab` | Cycle focus (input → stream → panel) |
| `Esc` | Close panel / Unselect / Dismiss |
| `j/k` | Navigate stream entries |
| `Space` | Expand/collapse entry |
| `>` or `/` | Command palette |
| `?` | Help overlay |

---

## 6. The anvil.js Contract

Every agent, skill, hook, and plugin is an `anvil.js` file executed by rquickjs.

### 6.1 Agent Pack Structure

```
~/.anvil/agents/{name}/
├── anvil.js          # Config + logic + hooks + commands
├── prompts/
│   ├── system.txt    # System prompt for this agent
│   └── {task}.txt    # Task-specific prompts
└── tests/
    └── validate.js   # Validation script (optional)
```

### 6.2 Minimal anvil.js

```javascript
// ~/.anvil/agents/backend/anvil.js

// Static configuration
export const config = {
  name: "backend",
  version: "1.0.0",
  description: "Backend engineering agent",
  defaultModel: "anthropic/claude-sonnet-4",
  maxCost: 5.00,
};

// Dynamic model selection (overrides OODA router)
export function route(task, state) {
  if (task.type === "architecture") {
    return state.models["anthropic/claude-sonnet-4"];
  }
  if (task.type === "refactor" && state.models["ollama/llama3.3"].isHealthy) {
    return state.models["ollama/llama3.3"]; // Free
  }
  return state.models["openai/gpt-4o"];
}

// Plan generation
export async function plan(intent, ctx) {
  return [
    { id: "read", action: "read_context", files: intent.targetFiles },
    { id: "impl", action: "generate_code", after: ["read"] },
    { id: "test", action: "run_tests", after: ["impl"] },
  ];
}

// Validation gate
export async function validate(result, ctx) {
  const tests = await ctx.run("cargo test");
  return {
    pass: tests.exitCode === 0,
    retry: tests.exitCode !== 0,
    maxRetries: 3,
  };
}

// Lifecycle hooks
export async function onTaskStart(ctx) {
  console.log(`Starting ${ctx.task.id}`);
}

export async function onTaskComplete(result, ctx) {
  await ctx.git.commit({
    message: `[anvil] ${result.task.id} | ${result.model} | $${result.cost}`,
  });
}

// Slash commands
export const commands = {
  "deploy": async (args, ctx) => {
    await ctx.run("cargo build --release");
    return { status: "built" };
  },
};
```

### 6.3 Context API (Exposed from Rust)

```javascript
// Available in all anvil.js functions
const ctx = {
  // Task info
  task: { id, type, intent, estimatedTokens, severity },

  // Router state (populated from models.dev)
  router: {
    models: { "anthropic/claude-sonnet-4": {...}, ... },
    estimateCost: (task) => Promise<Cost>,
    downgradeModel: (task) => void,
  },

  // Git operations
  git: {
    commit: (options) => Promise<void>,
    changedFiles: () => Promise<string[]>,
    worktree: { path, branch },
  },

  // Code intelligence
  memory: {
    query: (q) => Promise<Result[]>,
    checkOverlap: (tasks) => Promise<{ conflict, groupA, groupB }>,
  },

  // Execution
  run: (cmd, options) => Promise<{ exitCode, stdout, stderr }>,

  // UI
  ui: {
    showError: (title, detail) => void,
    showInfo: (message) => void,
  },

  // Safety
  safety: {
    requireApproval: (reason) => Promise<void>,
    requireReview: (reason) => void,
    pause: (reason) => void,
  },

  // Human interaction
  human: {
    confirm: (options) => Promise<boolean>,
    select: (options) => Promise<string>,
    input: (prompt) => Promise<string>,
  },

  // Session
  session: { cost, budget, elapsed, modelUsage },
};
```

### 6.4 Hook Scripts

```javascript
// ~/.anvil/hooks/safety.js
export async function preEdit(ctx) {
  const { file, changes } = ctx;

  if (file.match(/\.(env|key|pem)$/)) {
    throw new SafetyViolation("Cannot edit secret files");
  }

  if (changes.deletions > 100) {
    await ctx.human.confirm({
      message: `Delete ${changes.deletions} lines from ${file}?`,
      diff: changes.preview,
    });
  }
}

export function onCostThreshold(ctx) {
  if (ctx.current > ctx.limit * 0.8) {
    ctx.router.downgradeModel(ctx.task);
  }
}
```

### 6.5 Skill Scripts

```javascript
// ~/.anvil/skills/rust-check/anvil.js
export const config = {
  name: "rust-check",
  trigger: "post-edit",
  filePattern: "*.rs",
};

export async function run(ctx) {
  const result = await ctx.run("cargo check");
  if (result.exitCode !== 0) {
    ctx.ui.showError("cargo check failed", result.stderr);
    return { action: "block", reason: "compilation errors" };
  }
  return { action: "allow" };
}
```

---

## 7. Execution Model

### 7.1 Single-Agent Default (v1)

```
User Input
    ↓
Intent Parser (NL → typed Intent)
    ↓
Plan Generator (static DAG)
    ↓
[User reviews plan — Ctrl+Enter to approve]
    ↓
DAG Executor (structured concurrency)
    ↓
Safety Hook (pre-execution check)
    ↓
OODA Router (actor-per-model)
    ↓
LLM API Call
    ↓
Result → Safety Hook → Test Gate
    ↓
[Pass] Next DAG node
[Fail] Pause → Ask user (retry/replan/skip)
    ↓
Task Complete → Git commit → Session log
```

### 7.2 Multi-Agent Swarm (v1.5)

```
User Input
    ↓
Intent Parser
    ↓
Plan Generator detects parallelizable steps
    ↓
Pre-flight overlap check (tree-sitter)
    ↓
[No conflict] Spawn N agents in parallel worktrees
              Each: own model, own cost, own safety
    ↓
Join Gate: all must pass tests before merge
    ↓
Semantic merge (tree-sitter AST-based)
    ↓
Git merge commit with full attribution
```

---

## 8. The OODA Router

### 8.1 Decision Loop

```
OBSERVE:  Task needs → model states → quota → history
ORIENT:   Match profile → tradeoffs → fallback chain
DECIDE:   Route to model → set timeout → attach breaker
ACT:      Execute → stream → track → update health
          If failure → trigger OODA again with degraded state
```

### 8.2 Cost-Quality-Speed Surface

| Task Type | Context | Reasoning | Best Model | Cost |
|---|---|---|---|---|
| Simple refactor | 1K | Low | ollama/llama3.3 | $0 |
| Architecture design | 50K | High | anthropic/claude-sonnet-4 | $3/Mtok |
| Test generation | 10K | Pattern | openai/gpt-4o-mini | $0.60/Mtok |
| Context-heavy analysis | 500K | Medium | google/gemini-1.5-pro | $1/Mtok |
| Emergency fix (quota exhausted) | 5K | Medium | deepseek/deepseek-v3 | $0.50/Mtok |

### 8.3 Learning

Every routing decision generates training data:
```
Task type X, context Y, model Z → 95% success, $0.02 cost, 2.1s latency
```

Improves routing over time. Local to user's machine (privacy-preserving).

---

## 9. models.dev Integration

### 9.1 Sync Strategy

| Mode | Behavior |
|---|---|
| **Online** | Fetch `https://models.dev/api.json` at startup, cache to `~/.anvil/cache/models.dev.json` |
| **Offline** | Use cached version, warn if >7 days old |
| **Bundled** | Ship snapshot with binary for instant startup (`--offline` flag) |
| **Custom** | `ANVIL_MODELS_DEV_URL` env var for enterprise mirrors |

### 9.2 Usage in Anvil

```rust
// Router discovers models from env + models.dev
let models_db = models_dev::fetch_or_cache().await?;

// Filter by capability and availability
let available = models_db.models.iter()
    .filter(|m| std::env::var(&m.provider.env[0]).is_ok())  // API key present
    .filter(|m| m.capabilities.tool_call)                  // Supports tools
    .filter(|m| task.estimated_tokens < m.limit.context)   // Fits context
    .collect();

// Estimate cost using live pricing
let cost = (task.tokens as f64 / 1_000_000.0) * model.cost.input;
```

### 9.3 TUI Display

Canonical IDs: `anthropic/claude-sonnet-4`, `openai/gpt-4o`, `google/gemini-1.5-pro` — same as AI SDK ecosystem.

---

## 10. Safety Envelope

### 10.1 Configuration

```toml
# ~/.anvil/safety.toml
max_cost_per_task = 5.00
max_cost_per_session = 50.00
protected_paths = [".env", "secrets/", ".ssh/"]
required_tests = true
max_retries = 3
```

### 10.2 Enforcement

| Rule | Enforcement |
|---|---|
| Max cost per task | Router refuses to route if estimate exceeds budget |
| Max cost per session | Hard stop, human escalation |
| Protected paths | Filesystem layer blocks writes (Landlock on Linux) |
| Required tests | Test Gate runs `cargo test` + `cargo clippy`. Max 3 retries. |
| Human checkpoint | High-risk actions (git push, >100 line deletions) pause for approval |

### 10.3 Hook Integration

Safety rules are executable JS, not static config:

```javascript
export async function preEdit(ctx) {
  if (ctx.changes.deletions > 100) {
    await ctx.human.confirm({
      message: `Delete ${ctx.changes.deletions} lines?`,
      diff: ctx.changes.preview,
    });
  }
}
```

---

## 11. Technology Stack

### 11.1 Rust Dependencies

```toml
[dependencies]
# Core Async
tokio = { version = "1", features = ["full"] }
futures = "0.3"

# TUI
ratatui = "0.29"
crossterm = "0.28"

# JS Scripting
rquickjs = { version = "0.11", features = ["full-async", "loader", "futures", "chrono", "allocator"] }

# DAG Execution
dagx = "0.1"
futures-concurrency = "7"

# Git
git2 = "0.19"

# Code Parsing
tree-sitter = "0.24"
tree-sitter-rust = "0.23"

# HTTP + Router
reqwest = { version = "0.12", features = ["json"] }
tower = "0.5"
backoff = "0.4"

# Config
toml = "0.8"
toml_edit = "0.22"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Progress + Interaction
indicatif = "0.17"
dialoguer = "0.11"

# File Watch
notify = "6"
notify-debouncer-mini = "0.4"

# Cache
moka = { version = "0.12", features = ["future"] }

# CLI
clap = { version = "4", features = ["derive", "env"] }

# Errors
anyhow = "1"
thiserror = "2"
color-eyre = "0.6"

# Observability
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# Data Structures
dashmap = "6"
petgraph = "0.6"

# Text
regex = "1"
globset = "0.4"
syntect = "5"
unicode-width = "0.2"

# Crypto + Versioning
sha2 = "0.10"
blake3 = "1"
semver = "1"

# Time
chrono = "0.4"

# Process
tempfile = "3"
which = "6"
```

### 11.2 External Services

| Service | Purpose | Fallback |
|---|---|---|
| `models.dev/api.json` | Model database | Bundled snapshot |
| LLM APIs (Claude, OpenAI, etc.) | Agent execution | Ollama local |
| Git remotes | Agent pack distribution | Local filesystem |

---

## 12. Project Structure

```
anvil/
├── Cargo.toml
├── src/
│   ├── main.rs              # CLI entry (clap)
│   ├── tui/
│   │   ├── app.rs           # Ratatui event loop + layout
│   │   ├── stream.rs        # Agent stream rendering (thoughts, edits, plans)
│   │   ├── panel.rs         # Modal panels (skills, agents, questions)
│   │   ├── header.rs        # Status bar (agents, cost, repo)
│   │   └── input.rs         # Command palette + text input
│   ├── core/
│   │   ├── dag.rs           # DAG executor (dagx wrapper)
│   │   ├── intent.rs        # NL → typed intent parser
│   │   ├── safety.rs        # Safety envelope enforcement
│   │   └── git.rs           # Worktree ops (git2)
│   ├── router/
│   │   ├── ooda.rs          # OODA decision logic
│   │   ├── models.rs        # Model actor implementations
│   │   └── health.rs        # Circuit breakers + metrics
│   └── script/
│       ├── runtime.rs       # rquickjs AsyncRuntime
│       ├── bridge.rs        # Rust ↔ JS type conversions
│       ├── loader.rs        # Module resolver (git packs, bytecode)
│       └── api.rs           # Exposed ctx API
├── scripts/
│   ├── default-router.js    # Default OODA routing logic
│   ├── default-safety.js    # Default safety hooks
│   └── default-team.js      # Default team topology builder
└── tests/
    └── integration_tests.rs
```

---

## 13. Commands

```bash
# Interactive TUI mode
anvil run

# Headless single-shot (scriptable, CI/CD)
anvil run --print "refactor auth module to use OAuth2"

# Force specific model
anvil run --model anthropic/claude-sonnet-4

# Install agent pack from git
anvil agent install https://github.com/alice/backend-agent

# List installed packs
anvil agent list

# Show model selector
anvil models

# Show cost breakdown
anvil cost

# Show agent swarm status
anvil agents

# Offline mode (use bundled models.dev snapshot)
anvil run --offline
```

---

## 14. What v1 Is NOT

| Feature | Status | Reason |
|---|---|---|
| Web store frontend | v2 | CLI-only install via git clone |
| WASM plugins | v3 | rquickjs is the plugin system |
| RPC / IDE integration | v2 | Terminal-only |
| Firecracker microVMs | v3 | Worktree + Landlock is enough |
| rustdoc memory graph | v2 | Tree-sitter only |
| Graphical DAG visualization | v2 | Text-based status list |
| A2A agent protocol | v2 | Agents communicate via git commits |
| Formal verification | v3 | cargo test + clippy only |
| Network firewall rules | v2 | Subprocess isolation |
| FSM workflow modes | v2 | One execution model |
| CRDT session merging | v2 | Simple JSON log |
| Micro-commits per action | v2 | One commit per task |
| Multi-agent default | v1.5 | Single-agent default, swarm opt-in |
| 11-crate stack | Never | ONE binary, internal modules |

---

## 15. Honest Gaps

1. **macOS isolation**: Landlock is Linux-only. macOS needs sandbox-exec or no kernel isolation.
2. **Tree-sitter graph viz**: Complex DAGs may need WebView fallback for ratatui.
3. **Git-native store adoption**: Unproven vs centralized registries.
4. **rquickjs single-threaded**: One runtime per agent task. Not a bottleneck for v1.
5. **Context window economics**: 500K tasks on Gemini still cost $0.50-1.00.
6. **Plan mode latency**: 10-40s upfront planning. Users may skip for quick fixes.
7. **models.dev freshness**: Stale pricing if offline >7 days.

---

## 16. The Core Insight

> **Anvil is not a chatbot. It is a terminal-native operating system for coding with AI swarms.**
>
> The stream is the log. The plan is the contract. The swarm is the workforce. The safety envelope is the constitution. The cost HUD is the budget. The models.dev integration is the intelligence layer. And the user is the commander — not the typist.
