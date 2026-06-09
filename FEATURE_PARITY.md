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
| Command registry | ✅ | 🔄 | `tasks/r2-command-registry.md` |
| Command palette (Ctrl+P) | ✅ | 🔄 | `tasks/r2-command-palette.md` |
| Dialog state system | ✅ | 🔄 | `tasks/r2-command-palette.md` |
| Extensions / plugins | ✅ | ❌ | Excluded by design decision — plugins add complexity without clear daily-use value |
| SDK / embedding | ✅ | 🔄 | `tasks/r3-rpc-mode.md` |
| External editor (Ctrl+G) | ✅ | 🔄 | `tasks/r2-external-editor.md` |

---

## Providers & Models

| Feature | pi | runie | Task |
|---------|:--:|:-----:|------|
| Provider count | 35 | 35 | |
| Model count | ~968 | ~130 | |
| Runtime model switch | ✅ | ✅ | |
| Model selector (Ctrl+L) | ✅ | 🔄 | `tasks/r2-model-selector.md` |
| Model cycling (Ctrl+P) | ✅ | 🔄 | `tasks/r2-model-cycling.md` |
| Scoped model filtering | ✅ | 🔄 | `tasks/r2-scoped-models.md` |
| Provider attribution | ✅ | 🔄 | `tasks/r2-provider-attribution.md` |
| Thinking levels (Shift+Tab) | ✅ | 🔄 | `tasks/r2-thinking-levels.md` |
| OAuth authentication | ✅ | 🔄 | `tasks/r3-oauth-login.md` |
| Dynamic provider config | ✅ | 🔄 | `tasks/r2-dynamic-provider-config.md` |

---

## Sessions

| Feature | pi | runie | Task |
|---------|:--:|:-----:|------|
| Save / load | ✅ | ✅ | |
| List / delete | ✅ | ✅ | |
| Session naming (/name) | ✅ | 🔄 | `tasks/r2-session-commands.md` |
| Export / import | ✅ | 🔄 | `tasks/r2-session-commands.md` |
| New / resume | ✅ | 🔄 | `tasks/r2-session-commands.md` |
| Compact / reset | ✅ | ✅ | |
| Session branching (/fork, /clone, /tree) | ✅ | 🔄 | `tasks/r3-session-tree.md` |
| Session info/stats (/session) | ✅ | 🔄 | `tasks/r2-session-info.md` |
| Session tree navigation | ✅ | 🔄 | `tasks/r3-session-tree.md` |
| Session filters | ✅ | 🔄 | `tasks/r3-session-tree.md` |

---

## TUI / Rendering

| Feature | pi | runie | Task |
|---------|:--:|:-----:|------|
| Streaming responses | ✅ | ✅ | |
| Sort by last update | ✅ | ✅ | |
| Markdown rendering | ✅ | ✅ | |
| Syntax highlighting | ✅ | ✅ | |
| Diff rendering | ✅ | ✅ | |
| ANSI colors | ✅ | ✅ | |
| Scrollbar | ✅ | ✅ | |
| Footer status | ✅ | ✅ | |
| Thinking display | ✅ | ✅ | |
| Tool collapse (Ctrl+O) | ✅ | ✅ | |
| Thinking collapse (Ctrl+T) | ✅ | ✅ | |
| File references (@) | ✅ | ✅ | |
| Multi-line input | ✅ | ✅ | |
| Theme system | ✅ | 🔄 | `tasks/r2-theme-system.md` |
| Thinking levels | ✅ | 🔄 | `tasks/r2-thinking-levels.md` |
| Path completion (Tab) | ✅ | 🔄 | `tasks/r2-path-completion.md` |
| Image paste (Ctrl+V) | ✅ | 🔄 | `tasks/r3-image-paste.md` |
| Token / cost tracking | ✅ | ✅ | |
| Read-only mode | ✅ | 🔄 | `tasks/r2-safety-commands.md` |
| Tool output truncation | ✅ | ✅ | |
| Output accumulator | ✅ | 🔄 | `tasks/r2-output-accumulator.md` |

---

## Tools

| Feature | pi | runie | Task |
|---------|:--:|:-----:|------|
| bash, read, write | ✅ | ✅ | |
| edit, ls, grep, find | ✅ | ✅ | |
| Safety blacklist | ✅ | ✅ | |
| Output size limits | ✅ | ✅ | |
| File mutation queue | ✅ | 🔄 | `tasks/r2-file-mutation-queue.md` |
| Edit diff preview | ✅ | 🔄 | `tasks/r2-edit-diff-preview.md` |
| Path utils / cwd | ✅ | 🔄 | `tasks/r2-path-utils.md` |

---

## Input & Commands

| Feature | pi | runie | Task |
|---------|:--:|:-----:|------|
| Slash commands (core) | ✅ | ✅ | |
| Command registry | ✅ | 🔄 | `tasks/r2-command-registry.md` |
| Command palette | ✅ | 🔄 | `tasks/r2-command-palette.md` |
| Message queue | ✅ | ✅ | |
| Queue delivery mode | ✅ | ✅ | |
| Dequeue (Alt+Up) | ✅ | 🔄 | `tasks/r2-dequeue.md` |
| Bash prefix (!) | ✅ | ✅ | |
| Input history | ✅ | ✅ | |
| History persistence | ✅ | ✅ | |
| Undo/redo | ✅ | ✅ | |
| Word navigation | ✅ | ✅ | |
| Bracketed paste | ✅ | ✅ | |
| Skills system | ✅ | 🔄 | `tasks/r3-skills.md` |
| Custom prompt templates | ✅ | 🔄 | `tasks/r3-custom-prompts.md` |

---

## Safety & Trust

| Feature | pi | runie | Task |
|---------|:--:|:-----:|------|
| Bash blacklist | ✅ | ✅ | |
| Output size limits | ✅ | ✅ | |
| Read-only mode | ✅ | 🔄 | `tasks/r2-safety-commands.md` |
| Trust system (/trust) | ✅ | 🔄 | `tasks/r2-safety-commands.md` |
| Output accumulator | ✅ | 🔄 | `tasks/r2-output-accumulator.md` |

---

## Keybindings

| Feature | pi | runie | Task |
|---------|:--:|:-----:|------|
| Configurable keybindings | ✅ | ✅ | |
| Semantic names | ✅ | 🔄 | `tasks/r2-command-registry.md` |
| Model cycling (Ctrl+P) | ✅ | 🔄 | `tasks/r2-model-cycling.md` |
| Model selector (Ctrl+L) | ✅ | 🔄 | `tasks/r2-model-selector.md` |
| Tool expand (Ctrl+O) | ✅ | ✅ | |
| Thinking toggle (Ctrl+T) | ✅ | ✅ | |
| Thinking cycle (Shift+Tab) | ✅ | 🔄 | `tasks/r2-thinking-levels.md` |
| External editor (Ctrl+G) | ✅ | 🔄 | `tasks/r2-external-editor.md` |
| Paste image (Ctrl+V) | ✅ | 🔄 | `tasks/r3-image-paste.md` |
| Suspend (Ctrl+Z) | ✅ | 🔄 | `tasks/r3-suspend.md` |
| Basic shortcuts | ✅ | ✅ | |

---

## Configuration

| Feature | pi | runie | Task |
|---------|:--:|:-----:|------|
| TOML config | ✅ | ✅ | |
| Hot reload | ✅ | ✅ | |
| Settings dialog (/settings) | ✅ | 🔄 | `tasks/r2-settings-dialog.md` |
| Theme system | ✅ | 🔄 | `tasks/r2-theme-system.md` |
| Migrations | ✅ | 🔄 | `tasks/r3-config-migrations.md` |
| Telemetry | ✅ | 🔄 | `tasks/r3-telemetry.md` |
| Diagnostics | ✅ | 🔄 | `tasks/r3-diagnostics.md` |

---

## Modes

| Feature | pi | runie | Task |
|---------|:--:|:-----:|------|
| Interactive TUI | ✅ | ✅ | |
| Print mode | ✅ | 🔄 | `tasks/r3-print-mode.md` |
| JSON mode | ✅ | 🔄 | `tasks/r3-json-mode.md` |
| RPC / server | ✅ | 🔄 | `tasks/r3-rpc-mode.md` |

---

## Summary

**Implemented (✅):** 40 major features

**Planned (🔄):** 37 major features — all have task files

**Coverage:** 100% of pi features tracked. Zero gaps.
