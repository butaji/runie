# Feature Parity: runie vs pi

## Legend
- ✓ = Implemented
- ✗ = Not implemented
- ~ = Partial / different implementation

---

## Architecture

| Feature | pi | runie | Notes |
|---------|:--:|:-----:|-------|
| Event-driven MVU | ✓ | ✓ | Both single-threaded async loops |
| Batched event processing | ✓ | ✓ | pi: message queue; runie: BATCH_SIZE=10 |
| Lazy cache / diff render | ✓ | ✓ | pi: differential TUI; runie: LazyCache + sort-by-update |
| **Extensions / plugins** | ✓ | ✗ | pi: npm-style extensions, skills, themes, packages |
| **SDK / embedding** | ✓ | ✗ | pi: RPC, SDK, print/JSON modes |
| **External editor integration** | ✓ | ✗ | pi: Ctrl+G opens $EDITOR |

---

## Providers & Models

| Feature | pi | runie | Notes |
|---------|:--:|:-----:|-------|
| Provider count | 35 | 35 | Same catalog derived from pi |
| Model count | ~968 | ~130 | runie keeps curated headline subset |
| Runtime model switch | ✓ | ✓ | Both `/model` command |
| **Model cycling (Ctrl+P)** | ✓ | ✗ | pi: Ctrl+P / Shift+Ctrl+P cycles scoped models |
| **Scoped model filtering** | ✓ | ✗ | pi: `/scoped-models` enables/disables models for cycling |
| **Model selector UI** | ✓ | ✗ | pi: Ctrl+L opens interactive model picker |
| **Provider attribution** | ✓ | ✗ | pi: shows which provider served the response |
| **OAuth authentication** | ✓ | ✗ | pi: `/login`, `/logout` per provider |
| **Dynamic provider config** | ✓ | ✗ | pi: resolves config from env, files, CLI flags |

---

## Sessions

| Feature | pi | runie | Notes |
|---------|:--:|:-----:|-------|
| Save / load | ✓ | ✓ | Both JSON-based |
| List / delete | ✓ | ✓ | |
| **Session branching (/tree)** | ✓ | ✗ | pi: `/fork`, `/clone`, `/tree` — fork from any message |
| **Session naming** | ✓ | ✗ | pi: `/name` sets display name |
| **Session info/stats** | ✓ | ✗ | pi: `/session` shows metadata |
| Context compaction | ✓ | ✓ | runie: `/compact [prompt]` — truncates old messages |
| **Export to HTML/JSONL** | ✓ | ✗ | pi: `/export`, `/share` as GitHub gist |
| **Import from JSONL** | ✓ | ✗ | pi: `/import` resumes a session |
| **Session tree navigation** | ✓ | ✗ | pi: visual tree with fold/unfold, labels, filters |
| **Session filters** | ✓ | ✗ | pi: no-tools, user-only, labeled-only, all |

---

## TUI / Rendering

| Feature | pi | runie | Notes |
|---------|:--:|:-----:|-------|
| Streaming response merge | ✓ | ✓ | Both merge chunks by request ID |
| Sort by last update | ✓ | ✓ | Elements float to bottom on update |
| Token count in footer | ✓ | ✓ | Shows total tokens |
| Queue count in footer | ✓ | ✓ | Shows queued messages |
| **Tool output collapse** | ✓ | ✓ | Both Ctrl+O toggles tool visibility |
| **Thinking block collapse** | ✓ | ✓ | Both Ctrl+T toggles thinking visibility |
| **Thinking levels** | ✓ | ✗ | pi: Shift+Tab cycles low/medium/high |
| **Thinking level cycle** | ✓ | ✗ | pi: Shift+Tab |
| File references (@) | ✓ | ✓ | runie: `@` detection in input title |
| **Path completion** | ✓ | ✗ | pi: Tab completion for paths |
| **Multi-line input** | ✓ | ✗ | pi: Shift+Enter for newlines |
| **Image paste** | ✓ | ✗ | pi: Ctrl+V / drag from clipboard |
| Token / cost tracking | ✓ | ✓ | TokenTracker with $/1M token costs |
| **Read-only tool mode** | ✓ | ✗ | pi: can restrict to read/grep/find/ls only |
| **Tool output truncation** | ✓ | ✓ | Both: head for reads/grep/find/ls, tail for bash |
| **Output accumulator / guard** | ✓ | ~ | pi: output-accumulator.ts; runie: truncate.rs module |

---

## Tools

| Feature | pi | runie | Notes |
|---------|:--:|:-----:|-------|
| bash | ✓ | ✓ | Both with safety guards |
| read / view | ✓ | ✓ | |
| write | ✓ | ✓ | |
| edit (search/replace) | ✓ | ✓ | Both validate unique match |
| ls / list_dir | ✓ | ✓ | |
| grep | ✓ | ✓ | ripgrep fallback to grep; regex/literal/glob/limit |
| find / glob | ✓ | ✓ | fd fallback to find; glob patterns; .gitignore respect |
| Structured JSON tools | ✓ | ✓ | JSON + legacy `TOOL:` fallback |
| **File mutation queue** | ✓ | ✗ | pi: serializes file edits to avoid conflicts |
| **Edit diff preview** | ✓ | ✗ | pi: shows diff before applying edit |
| **Path utils / cwd resolution** | ✓ | ~ | runie: relative paths; pi: full cwd resolution |

---

## Input & Commands

| Feature | pi | runie | Notes |
|---------|:--:|:-----:|-------|
| Slash commands | ✓ | ✓ | `/model`, `/save`, `/load`, `/sessions`, `/delete`, `/reset`, `/help`, `/compact` |
| **Additional slash commands** | ✓ | ✗ | pi: `/export`, `/import`, `/share`, `/copy`, `/name`, `/session`, `/fork`, `/clone`, `/tree`, `/trust`, `/login`, `/logout`, `/new`, `/resume`, `/reload`, `/changelog`, `/hotkeys` |
| Message queue | ✓ | ✓ | Steering (Enter) + Follow-up (Alt+Enter) + Abort (Esc) |
| **Dequeue (restore queued)** | ✓ | ✗ | pi: Alt+Up restores queued messages |
| **Bash prefix (!)** | ✓ | ✗ | pi: `!command` runs + sends, `!!command` runs only |
| **Skills system** | ✓ | ✗ | pi: loads SKILL.md files from user/project dirs |
| **Custom prompt templates** | ✓ | ✗ | pi: user-defined system prompt overrides |

---

## Safety & Trust

| Feature | pi | runie | Notes |
|---------|:--:|:-----:|-------|
| Bash blacklist | ✓ | ✓ | Both block rm -rf /, dd, mkfs, fork bombs |
| **Trust system** | ✓ | ✗ | pi: `/trust` per-project decision |
| **Output guard** | ✓ | ✗ | pi: output-accumulator.ts limits tool output size |

---

## Keybindings

| Feature | pi | runie | Notes |
|---------|:--:|:-----:|-------|
| **Configurable keybindings** | ✓ | ✗ | pi: `keybindings.json` in agent dir |
| **Model cycling** | ✓ | ✗ | pi: Ctrl+P / Shift+Ctrl+P |
| **Model selector** | ✓ | ✗ | pi: Ctrl+L |
| **Thinking level cycle** | ✓ | ✗ | pi: Shift+Tab |
| **Tool expand toggle** | ✓ | ✓ | pi: Ctrl+O; runie: Ctrl+O |
| **Thinking toggle** | ✓ | ✓ | pi: Ctrl+T; runie: Ctrl+T |
| **External editor** | ✓ | ✗ | pi: Ctrl+G |
| **Paste image** | ✓ | ✗ | pi: Ctrl+V (Alt+V on Win) |
| **Suspend to background** | ✓ | ✗ | pi: Ctrl+Z |
| Basic shortcuts (quit, scroll) | ✓ | ✓ | Ctrl+C/Q/D, Up/Down |

---

## Configuration & Settings

| Feature | pi | runie | Notes |
|---------|:--:|:-----:|-------|
| TOML config | ✓ | ✓ | runie: `~/.runie/config.toml` |
| **Settings UI/menu** | ✓ | ✗ | pi: `/settings` interactive menu |
| **Theme system** | ✓ | ✗ | pi: customizable themes |
| **Migrations** | ✓ | ✗ | pi: config migration system |
| **Telemetry** | ✓ | ✗ | pi: opt-in telemetry |
| **Diagnostics** | ✓ | ✗ | pi: resource loading diagnostics |

---

## Modes

| Feature | pi | runie | Notes |
|---------|:--:|:-----:|-------|
| Interactive TUI | ✓ | ✓ | |
| **Print mode** | ✓ | ✗ | pi: non-interactive CLI output |
| **JSON mode** | ✓ | ✗ | pi: structured JSON output |
| **RPC / server mode** | ✓ | ✗ | pi: exposes SDK over RPC |

---

## Summary

**Implemented in runie:** Core architecture, provider support, basic TUI, tool suite, session persistence, message queue, safety guards, token tracking, @-file references, word wrapping, hot reload.

**Major gaps vs pi:**
1. **Extensions ecosystem** — no plugins, skills, themes, or packages
2. **Session tree** — no branching, forking, or visual tree navigation
3. **Keybindings** — all hardcoded, no user customization
4. **Advanced TUI** — thinking/tool collapse ✓; no thinking levels, model selector, multi-line input
5. **Export/import** — no HTML, JSONL, gist sharing
6. **Authentication** — no OAuth/login flow
7. **Modes** — no print, JSON, or RPC modes
8. **Configuration** — no settings UI, theme system, or migrations
9. **Output management** — truncation ✓; no accumulation or diff preview
