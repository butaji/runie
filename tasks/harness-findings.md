# Coding harness backlog

Ranked from the current Runie codebase and comparison with `../../kimi-code`.
Status is `planned` until source, event tests, replay tests, and TUI smoke
evidence prove the item.

1. `harness-01-tools` — **partial** — production Read, Write, Edit, Grep, Glob, and Bash tools are registered and tested; centralized traversal/sensitive-path policy and structured read/search metadata are covered, while richer output and background lifecycle remain.
2. `harness-02-permissions` — **partial** — pure Ask/Auto/YOLO/Deny policy now routes live mutating tools through either the owned question dialog or an explicit block; approval commands project into the live mode store; a YAML decision matrix is replayed, while UI cancellation traces remain.
3. `harness-03-shell-lifecycle` — **partial** — Bash now streams stdout/stderr chunks and typed `background_bash`, `background_jobs`, and `background_cancel` cover the owned job lifecycle; background snapshots now bound combined output and preserve failure status/output, and `/jobs` exposes the owned snapshot through the command-result dialog; richer TUI controls/output cards remain.
4. `harness-04-model-effort-ui` — **partial** — model metadata now supplies finite effort rows and selection, with YAML replay coverage proving unsupported levels are excluded; the HTTP request boundary now exposes each model’s declared provider-wire effort value; provider-specific payload adapters remain.
5. `harness-05-context` — **partial** — pure token estimation, strict automatic-compaction thresholding, actor-owned compaction preparation, and a typed `CompactionDecision` projection now exist; automatic loop-triggered recovery and durable summary UX remain.
6. `harness-06-plan-todos` — **partial** — replayable validated `todo_write` snapshots now reduce through an owned `TodoActor`, which also enforces the invariant at its state boundary; broader plan enforcement remains.
7. `harness-07-subagents` — **partial** — typed explore/plan/code requests now execute through an owned loop hook, declare replayable role capabilities, and return role/capability/output result data; capability enforcement remains.
8. `harness-08-tool-scheduler` — **partial** — independent read/search tools remain parallel while mutations and subagents are serialized; typed per-tool resource keys now partition only conflicting calls into separate batches, preserving parallelism for independent work; persistent queue prioritization remains.
9. `harness-09-web` — typed bounded `web_search` contract, executor hook, HTTP client with structured citation results, and optional live endpoint wiring now exist; provider adapters and source cards remain.
10. `harness-10-mcp` — **partial** — typed server/tool data, atomic stdio discovery/call registration, protocol-correct initialized notifications, ID-correlated notification-tolerant responses, structured JSON-RPC error propagation, and bounded HTTP JSON transport with bearer auth and a 1 MiB response cap now exist; streaming auth/session cleanup remains.
11. `harness-11-plugins` — **partial** — validated declarative plugin manifests and a duplicate-safe registry now model commands, tools, and hooks; filesystem installation and runtime lifecycle ownership remain.
12. `harness-12-git` — **partial** — bounded read-only Git inspection, typed non-mutating `git_commit_prepare`, approval-gated `git_commit`, explicit validated `git_push`, and safe inverse-commit `git_revert` now exist; richer conflict recovery remains.
13. `harness-13-sessions` — **partial** — resume/fork/rename/export foundations exist, and an actor-owned session search index now reduces upsert/remove/search events from bounded `SessionSnapshot` previews; full undo and storage discovery UX remain.
14. `harness-14-provider-contract` — **partial** — normalized usage, typed finish reasons with lossless raw provider values, response IDs/models, retry policy, and serializable provider failure classification now exist; adapter conformance remains.
15. `harness-15-user-questions` — **partial** — structured validation, owned broker, live selector, multi-select answers, YAML replay, explicit broker cancellation, broker-side option validation, and bounded resolution traces for answered/cancelled/rejected requests are implemented; rejected traces now retain the attempted structured answer, while durable session export remains.
16. `harness-16-media` — **partial** — model-declared input modalities now expose capability-aware `supports_input`/`supports_images` predicates, `ToolRegistry::tools_for_model` filters tools by required modality, and `ImageContent::new` validates image MIME/base64 data; video and provider-specific media encoding remain.
17. `harness-17-ide` — ACP or equivalent IDE protocol.
18. `harness-18-noninteractive` — **partial** — typed JSONL event encoding/decoding, deterministic completed/aborted/failed exit codes, and pure `--jsonl`/explicit approval argument parsing now exist in `runie-core`; CLI stdin/stdout wiring remains.
19. `harness-19-diagnostics` — **partial** — actor-owned `/doctor` now projects a serializable report with explicit checks and fix intent, telemetry exposes a serializable usage summary over ended provider spans, and `DiagnosticBundle` exports/replays the combined data with renderer-neutral metric rows; terminal visualization remains.
20. `harness-20-tui-polish` — searchable selectors, approvals, tasks, tool cards, and keyboard UX.

Existing foundations are not duplicated here: actors/events, provider registry,
model catalog, replay/session storage, compaction primitives, queues, hooks,
telemetry, pure snapshots, command palette, and TUI smoke infrastructure.

Smoke requirement: `scripts/tmux-command-smoke.sh` covers 50 palette cases plus
the Quit lifecycle, for 51 TUI-only cases. Provider-backed coding prompts remain
environment-dependent and must be recorded separately from local UI evidence.
