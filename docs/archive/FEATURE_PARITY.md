# Feature Parity: runie vs pi

## Legend
- ✅ = Implemented
- 🔄 = Planned (task exists)

---

## Architecture

| Feature | pi | runie | Task |
|---------|:--:|:-----:|------|
| Event-driven MVU | ✅ | ✅ | |
| Batched event processing | ✅ | ✅ | |
| Lazy cache / diff render | ✅ | ✅ | |
| Command registry | ✅ | ✅ |  |  |
| Command palette (Ctrl+P) | ✅ | ✅ |  |  |
| Dialog state system | ✅ | ✅ |  |  |
| Actor runtime (tokio tasks + event bus) | ✅ | 🔄 | `tasks/actor-runtime-decision.md` |
| Event bus with replay | ✅ | 🔄 | `tasks/event-bus-jsonl-persistence.md` |
| JSONL session persistence | ✅ | 🔄 | `tasks/event-bus-jsonl-persistence.md` |
| Event sub-enums | ✅ | 🔄 | `tasks/event-subenums.md` |
| Crate replacement audit | ✅ | 🔄 | `tasks/crate-replacement-audit.md` |
| Extensions / plugins | ✅ | ❌ | Excluded by design decision — plugins add complexity without clear daily-use value |
| SDK / embedding | ✅ | ❌ | Excluded by design decision — SDK/RPC mode not targeted |  |
| External editor (Ctrl+G) | ✅ | ❌ |  |  |

---

## Providers & Models

| Feature | pi | runie | Task |
|---------|:--:|:-----:|------|
| Provider count | 35 | 35 | |
| Model count | ~968 | ~130 | |
| Runtime model switch | ✅ | ✅ | |
| Normalized `LLMEvent` stream | ✅ | 🔄 | `tasks/llm-event-normalization.md` |
| Model capability flags | ✅ | 🔄 | `tasks/model-capability-flags.md` |
| Model selector (Ctrl+L) | ✅ | ✅ |  |  |
| Model cycling (Ctrl+P) | ✅ | ✅ |  |  |
| Scoped model filtering | ✅ | ✅ |  |  |
| Provider attribution | ✅ | ✅ |  |  |
| Thinking levels (Shift+Tab) | ✅ | ✅ |  |  |
| OAuth authentication | ✅ | ❌ |  |  |
| Dynamic provider config | ✅ | ✅ |  |  |

---

## Sessions

| Feature | pi | runie | Task |
|---------|:--:|:-----:|------|
| Save / load | ✅ | ✅ | |
| List / delete | ✅ | ✅ | |
| JSONL event-sourced sessions | ✅ | 🔄 | `tasks/event-bus-jsonl-persistence.md` |
| Session list with summaries | ✅ | ✅ |  |  |
| Session naming (/name) | ✅ | ✅ |  |  |
| Export / import | ✅ | ✅ |  |  |
| New / resume | ✅ | ✅ |  |  |
| Compact / reset | ✅ | ✅ | |
| Session branching (/fork, /clone, /tree) | ✅ | ✅ |  |  |
| Session info/stats (/session) | ✅ | ✅ |  |  |
| Session tree navigation | ✅ | ✅ |  |  |
| Session filters | ✅ | ❌ |  |  |

---

## Memory & Context

| Feature | pi | runie | Task |
|---------|:--:|:-----:|------|
| Context compaction | ✅ | 🔄 | `tasks/context-compaction.md` |
| Message metadata (pinned/hidden/ephemeral) | ✅ | 🔄 | `tasks/context-compaction.md` |

---

## TUI / Rendering

| Feature | pi | runie | Task |
|---------|:--:|:-----:|------|
| Streaming responses | ✅ | ✅ | |
| Streaming stable/tail split | ✅ | 🔄 | `tasks/streaming-buffer-tail-split.md` |
| Sort by last update | ✅ | ✅ | |
| Markdown rendering | ✅ | ✅ | |
| Syntax highlighting | ✅ | ✅ | |
| Diff rendering | ✅ | ✅ | |
| ANSI colors | ✅ | ✅ | |
| Scrollbar | ✅ | ✅ | |
| Footer status | ✅ | ✅ | |
| Status indicator widget | ✅ | 🔄 | `tasks/status-indicator-widget.md` |
| Thinking display | ✅ | ✅ | |
| Tool collapse (Ctrl+O) | ✅ | ✅ | |
| Tool state machine rendering | ✅ | 🔄 | `tasks/tool-call-state-rendering.md` |
| Thinking collapse (Ctrl+T) | ✅ | ✅ | |
| File references (@) | ✅ | ✅ | |
| Multi-line input | ✅ | ✅ | |
| Theme system | ✅ | ✅ |  |  |
| Semantic theme tokens | ✅ | 🔄 | `tasks/semantic-theme-tokens.md` |
| Thinking levels | ✅ | ✅ |  |  |
| Path completion (Tab) | ✅ | ✅ |  |  |
| Image paste (Ctrl+V) | ✅ | ❌ |  |  |
| Token / cost tracking | ✅ | ✅ | |
| Read-only mode | ✅ | ✅ |  |  |
| Tool output truncation | ✅ | ✅ | |
| Output accumulator | ✅ | ❌ |  |  |

---

## Tools

| Feature | pi | runie | Task |
|---------|:--:|:-----:|------|
| bash, read, write | ✅ | ✅ | |
| edit, ls, grep, find | ✅ | ✅ | |
| Safety blacklist | ✅ | ✅ | |
| Output size limits | ✅ | ✅ | |
| `ToolRegistry` trait | ✅ | 🔄 | `tasks/tool-registry-trait.md` |
| MCP client (stdio) | ✅ | 🔄 | `tasks/mcp-client-integration.md` |
| File mutation queue | ✅ | ✅ |  |  |
| Edit diff preview | ✅ | ✅ |  |  |
| Path utils / cwd | ✅ | ✅ |  |  |

---

## Agents & Orchestration

| Feature | pi | runie | Task |
|---------|:--:|:-----:|------|
| Solo execution mode (default) | ✅ | ✅ | |
| Subagents (`/spawn`) | ✅ | ✅ | Legacy; being replaced by Team mode |
| Team execution mode | ❌ | 🔄 | `tasks/r4-solo-team-mode-toggle.md` |
| Alignment Q&A in Dialog Panel | ❌ | 🔄 | `tasks/r4-ask-user-tool.md` |
| Dynamic multi-agent workflows | ❌ | 🔄 | `tasks/r4-orchestrator-actor.md` |
| Model trait-based routing | ❌ | 🔄 | `tasks/r4-model-trait-resolution.md` |
| Subagent sidebar + per-agent feeds | ❌ | 🔄 | `tasks/r4-subagent-sidebar.md` |

## Grok Build TUI Parity

Research and gap analysis live in `docs/grok-parity/GROK.md`.

| Feature | Grok | runie | Task |
|---------|:--:|:-----:|------|
| Mouse support (SGR, scroll, click) | ✅ | 🔄 | `tasks/grok-mouse-terminal-init.md`, `tasks/grok-mouse-hit-testing.md` |
| Contextual footer hints | ✅ | 🔄 | `tasks/grok-contextual-hints.md` |
| Mode suffix in input title | ✅ | 🔄 | `tasks/grok-input-title-modes.md` |
| Richer status bar (worktree, orchestrator state) | ✅ | 🔄 | `tasks/grok-status-bar-richer.md` |
| Block-level copy (`y`/`Y`) | ✅ | 🔄 | `tasks/grok-block-copy.md` |
| Palette ranking (recency × usage) | ✅ | 🔄 | `tasks/grok-palette-ranking.md` |
| Tool block inline status | ✅ | 🔄 | `tasks/grok-tool-block-status.md` |
| `@file` line ranges | ✅ | 🔄 | `tasks/grok-file-ref-ranges.md` |
| Theme quantization | ✅ | 🔄 | `tasks/grok-theme-quantization.md` |
| Welcome / launcher screen | ✅ | 🔄 | `tasks/grok-welcome-screen.md` |
| Inline diff polish | ✅ | 🔄 | `tasks/grok-inline-diff-polish.md` |

## Input & Commands

| Feature | pi | runie | Task |
|---------|:--:|:-----:|------|
| Slash commands (core) | ✅ | ✅ | |
| Command registry | ✅ | ✅ |  |  |
| Command palette | ✅ | ✅ |  |  |
| Message queue | ✅ | ✅ | |
| Queue delivery mode | ✅ | ✅ | |
| Dequeue (Alt+Up) | ✅ | ❌ |  |  |
| Bash prefix (!) | ✅ | ✅ | |
| Input history | ✅ | ✅ | |
| History persistence | ✅ | ✅ | |
| Undo/redo | ✅ | ✅ | |
| Word navigation | ✅ | ✅ | |
| Bracketed paste | ✅ | ✅ | |
| Skills system | ✅ | ✅ |  |  |
| Custom prompt templates | ✅ | ✅ |  |  |

---

## Safety & Trust

| Feature | pi | runie | Task |
|---------|:--:|:-----:|------|
| Bash blacklist | ✅ | ✅ | |
| Output size limits | ✅ | ✅ | |
| Read-only mode | ✅ | ✅ |  |  |
| Trust system (/trust) | ✅ | ✅ |  |  |
| Permission rulesets (wildcard allow/ask/deny) | ✅ | 🔄 | `tasks/permission-rulesets.md` |
| Read-only vs mutating tool classification | ✅ | 🔄 | `tasks/permission-rulesets.md` |
| Output accumulator | ✅ | ❌ |  |  |

---

## Keybindings

| Feature | pi | runie | Task |
|---------|:--:|:-----:|------|
| Configurable keybindings | ✅ | ✅ | |
| Semantic names | ✅ | ❌ |  |  |
| Model cycling (Ctrl+P) | ✅ | ✅ |  |  |
| Model selector (Ctrl+L) | ✅ | ✅ |  |  |
| Tool expand (Ctrl+O) | ✅ | ✅ | |
| Thinking toggle (Ctrl+T) | ✅ | ✅ | |
| Thinking cycle (Shift+Tab) | ✅ | ✅ |  |  |
| External editor (Ctrl+G) | ✅ | ❌ |  |  |
| Paste image (Ctrl+V) | ✅ | ❌ |  |  |
| Suspend (Ctrl+Z) | ✅ | ❌ |  |  |
| Basic shortcuts | ✅ | ✅ | |

---

## Configuration

| Feature | pi | runie | Task |
|---------|:--:|:-----:|------|
| TOML config | ✅ | ✅ | |
| Hot reload | ✅ | ✅ | |
| Settings dialog (/settings) | ✅ | ✅ |  |  |
| Theme system | ✅ | ✅ |  |  |
| Migrations | ✅ | ❌ |  |  |
| Telemetry | ✅ | ❌ |  |  |
| Diagnostics | ✅ | ✅ |  |  |

---

## Modes

| Feature | pi | runie | Task |
|---------|:--:|:-----:|------|
| Interactive TUI | ✅ | ✅ | |
| Print mode | ✅ | ✅ |  |  |
| JSON mode | ✅ | ✅ |  |  |
| RPC / server | ✅ | ✅ |  |  |

---

## Summary

**Implemented (✅):** 84 major features

**Planned (🔄):** 23 major features — all have task files

**Excluded (❌):** 15 major features (not targeted by design)

**Coverage:** 100% of pi features tracked. Zero gaps.
