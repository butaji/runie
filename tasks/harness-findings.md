# Coding harness backlog

Ranked from the current Runie codebase and comparison with `../../kimi-code`.
Status is `planned` until source, event tests, replay tests, and TUI smoke
evidence prove the item.

1. `harness-01-tools` — **partial** — production Read, Write, Edit, Grep, Glob, and Bash tools are registered and tested; centralized traversal/sensitive-path policy is covered, while richer output remains.
2. `harness-02-permissions` — **partial** — pure Ask/Auto/YOLO policy blocks known mutating tools without an approval hook; interactive TUI approval remains.
3. `harness-03-shell-lifecycle` — **partial** — Bash supports timeout/cancellation and captured output; streaming and background task actors remain.
4. `harness-04-model-effort-ui` — atomic model/effort picker from model metadata.
5. `harness-05-context` — automatic compaction, summaries, limits, and recovery.
6. `harness-06-plan-todos` — constrained plan mode and persistent todo state.
7. `harness-07-subagents` — isolated coder/explore/plan agents with result delivery.
8. `harness-08-tool-scheduler` — concurrent independent tools and serialized conflicts.
9. `harness-09-web` — optional search, fetch, source extraction, and source cards.
10. `harness-10-mcp` — MCP discovery, transports, auth, registration, and cleanup.
11. `harness-11-plugins` — installable skills, commands, tools, and hooks.
12. `harness-12-git` — status, diff, patch review, worktrees, and safe commit preparation.
13. `harness-13-sessions` — resume, fork, rename, undo, export, and session search UX.
14. `harness-14-provider-contract` — normalized usage, finish reasons, retries, IDs, and errors.
15. `harness-15-user-questions` — **partial** — structured AskUserQuestion validation, owned broker, and live TUI selector are implemented; replay fixtures and multi-select UI remain.
16. `harness-16-media` — image/video input and capability-aware tool exposure.
17. `harness-17-ide` — ACP or equivalent IDE protocol.
18. `harness-18-noninteractive` — JSON/JSONL, CI behavior, approvals, and exit codes.
19. `harness-19-diagnostics` — doctor, tracing, usage, export bundles, and visualization.
20. `harness-20-tui-polish` — searchable selectors, approvals, tasks, tool cards, and keyboard UX.

Existing foundations are not duplicated here: actors/events, provider registry,
model catalog, replay/session storage, compaction primitives, queues, hooks,
telemetry, pure snapshots, command palette, and TUI smoke infrastructure.

Smoke requirement: `scripts/tmux-command-smoke.sh` covers 49 palette cases plus
the Quit lifecycle, for 50 TUI-only cases. Provider-backed coding prompts remain
environment-dependent and must be recorded separately from local UI evidence.
