# Feature Parity: runie vs pi

| Feature | pi | runie | Notes |
|---------|:--:|:-----:|-------|
| **Architecture** |
| Event-driven MVU | ✓ | ✓ | Both single-threaded async loops |
| Batched event processing | ✓ | ✓ | pi: message queue; runie: BATCH_SIZE=10 |
| Lazy cache / diff render | ✓ | ✓ | pi: differential TUI; runie: LazyCache + sort-by-update |
| Extensions / plugins | ✓ | ✗ | pi: npm-style extensions, skills, themes, packages |
| SDK / embedding | ✓ | ✗ | pi: RPC, SDK, print/JSON modes |
| **Providers** |
| Provider count | 35 | 35 | Same catalog derived from pi |
| Model count | ~968 | ~130 | runie keeps curated headline subset |
| Runtime model switch | ✓ | ✓ | Both `/model` command |
| Model cycling (Ctrl+P) | ✓ | ✗ | |
| OAuth authentication | ✓ | ✗ | pi: `/login`, `/logout` |
| **Sessions** |
| Save / load | ✓ | ✓ | Both JSON-based |
| List / delete | ✓ | ✓ | |
| Session branching (/tree) | ✓ | ✗ | pi: fork/clone from any message |
| Context compaction | ✓ | ✓ | runie: `/compact [prompt]` — truncates old messages |
| Export to HTML | ✓ | ✗ | pi: `/export`, `/share` as gist |
| **TUI** |
| Streaming response merge | ✓ | ✓ | Both merge chunks by request ID |
| Sort by last update | ✓ | ✓ | Elements float to bottom on update |
| Token count in footer | ✓ | ✓ | Shows total tokens |
| Queue count in footer | ✓ | ✓ | Shows queued messages |
| Tool output collapse | ✓ | ✗ | pi: Ctrl+O |
| Thinking block collapse | ✓ | ✗ | pi: Ctrl+T |
| File references (@) | ✓ | ✓ | runie: `@` detection in input title |
| Path completion | ✓ | ✗ | pi: Tab completion |
| Multi-line input | ✓ | ✗ | pi: Shift+Enter |
| Image paste | ✓ | ✗ | pi: Ctrl+V / drag |
| Thinking levels | ✓ | ✗ | pi: Shift+Tab to cycle |
| Token / cost tracking | ✓ | ✓ | TokenTracker with $/1M token costs |
| **Tools** |
| bash | ✓ | ✓ | Both with safety guards |
| read / view | ✓ | ✓ | |
| write | ✓ | ✓ | |
| edit (search/replace) | ✓ | ✓ | Both validate unique match |
| ls / list_dir | ✓ | ✓ | |
| grep | ✓ | ✓ | ripgrep fallback to grep; regex/literal/glob/limit |
| find / glob | ✓ | ✓ | fd fallback to find; glob patterns; .gitignore respect |
| Structured JSON tools | ✓ | ✓ | JSON + legacy `TOOL:` fallback |
| **Input** |
| Slash commands | ✓ | ✓ | `/model`, `/save`, `/load`, `/sessions`, `/delete`, `/reset`, `/help`, `/compact` |
| Message queue | ✓ | ✓ | Steering (Enter) + Follow-up (Alt+Enter) + Abort (Esc/Ctrl+S) |
| Bash prefix (!) | ✓ | ✗ | pi: `!command` runs + sends, `!!command` runs only |
| **Safety** |
| Bash blacklist | ✓ | ✓ | Both block rm -rf /, dd, mkfs, fork bombs |
| Trust system | ✓ | ✗ | pi: `/trust` per-project |
| Output guard | ✓ | ✗ | pi: output-accumulator.ts |
