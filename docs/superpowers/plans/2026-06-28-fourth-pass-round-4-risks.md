# Round 4 — Risk and Assumption Review

## High-risk assumptions

### 1. `rmcp` will solve MCP integration

**Risk:** `rmcp` is new; its API may change. If MCP servers are not a near-term requirement, removing MCP config is lower risk than adopting an immature crate.
**Mitigation:** spike `rmcp` client in a branch before committing; time-box to one day.

### 2. JSONL persistence will scale

**Risk:** Long sessions may produce multi-megabyte JSONL files, slowing load/resume.
**Mitigation:** snapshot+journal pattern (already planned) and compaction threshold.

### 3. Replacing custom TUI widgets with `tui-textarea`/`tui-input`

**Risk:** These crates may not support all current input behaviors (e.g., paste, grapheme clusters, history).
**Mitigation:** spike one widget first; keep custom fallback until parity is proven.

### 4. Feature-gating `runie-core`

**Risk:** Adding features can introduce `cfg` fragmentation and test matrix explosion.
**Mitigation:** start with one feature (MCP), keep default feature set broad, add CI matrix only after stabilization.

### 5. Grok fixture comparison

**Risk:** Grok UI/behavior changes; fixtures become stale.
**Mitigation:** record fixtures as part of CI nightly, version them, and treat differences as signals, not regressions.

### 6. Typed error propagation

**Risk:** `anyhow` is deeply embedded; replacing it with typed errors may touch many call sites.
**Mitigation:** convert one boundary at a time (provider → core event) rather than a big-bang refactor.

## Assumptions to validate

| Assumption | Validation task |
|------------|-----------------|
| `strum` can derive all event taxonomy needs | `derive-event-taxonomies-with-strum-or-proc-macro` |
| `figment` can express all config overrides | already validated in first pass |
| JSONL + atomic writes is reliable enough | `use-atomic-writes-for-config-and-session-files` |
| `tui-textarea` matches custom input parity | spike before `finish-replacing-custom-tui-widgets` |
| `rmcp` is production-ready | spike before `wire-rmcp-client-or-remove-mcp-config` |

## Task mapping

| Finding | Task file | Status |
|---------|-----------|--------|
| Spike `rmcp` feasibility | `tasks/spike-rmcp-feasibility-before-mcp-decision.md` | **new** |
| Spike `tui-textarea` parity | `tasks/spike-tui-textarea-parity-before-widget-replacement.md` | **new** |
| Define JSONL compaction threshold | `tasks/define-jsonl-compaction-threshold-and-policy.md` | **new** |
