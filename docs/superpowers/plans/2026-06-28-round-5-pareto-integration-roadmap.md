# Round 5 — Pareto Simplification & Integration Roadmap

## Consolidation priorities

Duplicate concepts should be collapsed before adding features:

1. **State:** `TurnState` is the SSOT; `AgentState` becomes a typed projection.
2. **Events:** canonical `Event` enum replaces `DurableCoreEvent` + generated taxonomies.
3. **Persistence:** JSONL is the canonical runtime store; SQLite is deferred. Remove or fold `SqliteStore` into the JSONL path.
4. **Config:** `figment` replaces custom layering.
5. **Input/TUI:** `tui-input`/`tui-textarea` replace custom input state.
6. **Provider factory:** `async_trait` or enum replaces `Pin<Box<dyn Future>>`.

## Quick wins (80/20)

| Change | Effort | Impact | Task |
|--------|--------|--------|------|
| `strum` for event names | low | high | `finish-strum-migration-for-remaining-enums.md` |
| `figment` for config | low-medium | high | `replace-layered-config-merge-with-figment.md` |
| `pulldown-cmark` + frontmatter | low | medium | `replace-streaming-buffer-classifier-with-pulldown-cmark.md` |
| Helper crates (`globset`, `crokey`, `shell-words`, `textwrap`) | low | medium | `finish-replacing-remaining-custom-helpers-with-crates.md` |
| `ringbuffer` for `SpeedWindow` | low | low | `replace-speedwindow-with-ringbuffer-crate.md` |

## Integration order

1. **SSOT first:** fix direct `AppState` mutations and merge `AgentState` into `TurnState` projection.
2. **Crate replacements:** config (`figment`), event names (`strum`), markdown (`pulldown-cmark`), helpers.
3. **Async ownership:** track all spawned tasks, remove timeouts/sleeps.
4. **Module splits:** break files >500 lines after their internals are simplified.
5. **Persistence cleanup:** standardize on JSONL and remove the dual `SqliteStore` path once the event model is single-enum.

## Success metrics

- Fewer LOC in `runie-core` and `runie-tui`.
- Zero files >500 lines in production code.
- Fewer custom modules replaced by crates.
- All spawned tasks tracked (`JoinHandle`/`JoinSet`/completion event).
- No `sleep`-based polling in production or tests.

## Task mapping

| Finding | Task file | Status |
|---------|-----------|--------|
| Execute roadmap & track metrics | `tasks/execute-five-round-architecture-review-roadmap.md` | **new** |
| Enforce file/function/complexity limits | `tasks/enforce-advertised-file-function-complexity-limits.md` | existing `done` |
| Audit tasks for SSOT compliance | `tasks/audit-tasks-for-events-based-ssot-actor-compliance.md` | existing `todo` |
