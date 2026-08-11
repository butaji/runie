# Completion matrix

This is the finite acceptance boundary for the current harness backlog. An
item is complete only when its source, event/replay test, and listed runtime
evidence are present. “Richer”, “additional”, and “future” work outside this
table is intentionally out of scope for this pass.

| ID | Finite behavior | Required evidence |
| --- | --- | --- |
| H01 tools | bounded output card with facts, preview, truncation, and `/jobs <id>` | core event tests, renderer projection tests, TUI command case |
| H02 permissions | ask/auto/yolo/deny plus answered, cancelled, and rejected traces | YAML decision matrix, policy tests, dialog smoke cases |
| H03 shell | foreground stream, background start/status/output/cancel/clear, failure and cancellation | lifecycle replay, bounded-output tests, TUI cases |
| H04 effort | model selection and `/effort` accept only declared levels; all supported provider wire shapes | exhaustive profile matrix, request-body tests, model/effort smoke cases |
| H05 context | `/context`, `/context compact`, `/clear`, `/reset`, required/disabled/unknown recovery | YAML recovery fixtures, actor tests, TUI cases |
| H06 plan | `/plan on|off|view|clear` and validated todo transitions | todo replay, invariant tests, TUI cases |
| H07 subagents | explore/plan/code role validation, bounded output, owned lifecycle | role replay, rejection tests, TUI lifecycle case |
| H08 scheduler | queued/running cancellation, cancel-all, clear-finished, status filters | scheduler replay, metric tests, TUI cases |
| H09 web | bounded Generic/Brave/Tavily decode and normalized source cards | provider fixtures, cap/error tests, deterministic TUI projection |
| H10 MCP | stdio/http discovery, call, notification, reconnect, close, lifecycle filters | JSON-RPC replay, transport tests, TUI cases |
| H11 plugins | manifest discovery, install/uninstall, capability registration, bounded execution | manifest fixtures, lifecycle replay, command/tool tests |
| H12 Git | status/diff/review/worktrees plus approval-gated commit/push/revert/conflict summary | command-result tests, inverse/recovery replay, TUI cases |
| H13 sessions | resume/fork/rename/export, picker/history, counted undo | journal replay, picker/history tests, TUI cases |
| H14 provider contract | normalized usage, finish reasons, retry/failure, unsupported effort | cross-provider conformance matrix and replay tests |
| H15 questions | single/multi-select, validation, cancel, pending, paginated/filterable history, clear | YAML broker traces, validation tests, TUI cases |
| H16 media | declared modality filtering and supported provider image/video/audio encodings | media matrix, MIME/bounds tests, pure projection tests |
| H17 IDE | bounded JSON-RPC/LSP frame lifecycle and diagnostic projection | frame replay, malformed/bounds tests, deterministic TUI projection |
| H18 JSONL | prompt/stdin loop, provider/model/context/effort metadata, terminal exit events | JSONL fixtures, exit-code tests, noninteractive smoke command |
| H19 diagnostics | `/doctor inspect|fix`, `/usage`, bounded bundle and metric rows | report/telemetry replay, bounds tests, TUI cases |
| H20 TUI polish | searchable selectors, approvals, tasks, tool cards, keyboard navigation | pure widget tests, fixture captures, live tmux cases |

## Reduction closure

| Task | Closure criterion |
| --- | --- |
| reduction-04 | canonical feed records are the only mutable source; snapshot parity and insert/update/remove replay prove no stale index |
| reduction-08 | all hot-path consumers use immutable shared projections; isolation and allocation-focused tests pass |
| reduction-10 | representative core, feed, status, dialog, and paint TUI fixtures use the common YAML/event harness |
| reduction-12 | every claimed semantic boundary has a source inventory, no stale queued claim, and structural lint evidence |

## Verification gate

The final evidence set must include `just ci`, a fresh run of
`scripts/tmux-command-smoke.sh` with at least 50 passing TUI cases and zero
failures, and a task-status audit showing no item marked adopted without its
matrix evidence.
