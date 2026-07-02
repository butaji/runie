# Round 1 — Backlog Consolidation and Duplicate Removal

## Findings

### 1. Duplicate task groups

| Concept | Duplicate task files | Recommendation |
|---------|----------------------|----------------|
| Event taxonomy → `strum` | `derive-event-taxonomies-with-strum-or-proc-macro.md` <br> `finish-strum-migration-for-remaining-enums.md` <br> `use-strum-for-event-intent-names.md` | Keep `derive-event-taxonomies-with-strum-or-proc-macro.md` as canonical; others are done. |
| `Intent`/`Event` collapse | `collapse-intent-into-shared-event-payloads.md` <br> `collapse-event-intent-kind-taxonomies.md` | Merge descriptions; one canonical task. |
| `AgentState` → `TurnState` | `merge-agentstate-into-turnstate-projection.md` (1st pass R1/R2) | Single canonical task. |
| Direct `AppState` mutation | `remove-direct-appstate-mutation-from-tui-handlers.md` <br> `remove-direct-appstate-mutation-from-core-update-handlers.md` <br> `route-tui-autocomplete-through-inputactor-events.md` <br> `route-permission-clearance-through-permissionactor.md` | Conceptually one SSOT-in-TUI cleanup; tasks already done. |
| TUI widgets → crates | `replace-custom-input-box-with-tui-textarea.md` (done) <br> `replace-custom-form-rendering-with-tui-textarea.md` <br> `finish-replacing-custom-tui-widgets.md` | Merge remaining into `finish-replacing-custom-tui-widgets.md`. |
| Config layering | `replace-layered-config-merge-with-figment.md` <br> `remove-manual-env-overrides-from-config-layers.md` <br> `single-config-resolve-default-model.md` | Merge into one config-unification umbrella. |
| Session persistence | `standardize-session-persistence-on-jsonl.md` <br> `adopt-snapshot-journal-jsonl-pattern.md` <br> `replay-sessions-via-events-through-appstate.md` <br> `use-atomic-writes-for-config-and-session-files.md` | Merge into canonical JSONL persistence epic. |
| Tool registry | `generate-tool-dispatch-from-single-registry.md` <br> `unify-tool-registry-and-dispatch.md` | Single canonical task. |
| Observed async work | `enforce-observed-async-work-in-all-actors.md` <br> `track-orphan-spawned-tasks-in-provider-io-session-actors.md` <br> `remove-timeout-polling-from-uiactor-and-actors.md` | One SSOT-actor-lifecycle umbrella; close orphan task marked `wontfix`. |
| Roadmap meta-tasks | `execute-five-round-architecture-review-roadmap.md` <br> `execute-second-pass-architecture-review-roadmap.md` <br> `execute-third-pass-architecture-review-roadmap.md` <br> `execute-magic-numbers-cleanup-roadmap.md` | Merge into one live backlog execution task. |

### 2. Contradictions

| Conflict | Tasks | Resolution |
|----------|-------|------------|
| SQLite vs JSONL | `migrate-session-persistence-to-rusqlite.md` (`todo`) vs `standardize-session-persistence-on-jsonl.md` (`done`) | JSONL is the decided direction. Close the SQLite task as `wontfix`. |
| `ProviderProtocol` abstraction | 2nd pass R2 recommends removal/realization; task is `wontfix` | Update 2nd pass R2 doc to reflect the `wontfix` decision. |

### 3. Tasks that should be archived

- `migrate-session-persistence-to-rusqlite.md` — conflicts with decided JSONL direction.
- `track-orphan-spawned-tasks-in-provider-io-session-actors.md` — marked `wontfix`, superseded by other tasks.
- Duplicate `done` tasks that are already represented by a canonical task.

## Recommended changes

1. Add `supersedes` / `duplicates` fields to task frontmatter.
2. Mark `migrate-session-persistence-to-rusqlite.md` as `wontfix` with reason.
3. Merge remaining TUI widget tasks into `finish-replacing-custom-tui-widgets.md`.
4. Merge remaining session persistence tasks into `standardize-session-persistence-on-jsonl.md`.
5. Archive completed roadmap meta-tasks and create one unified execution tracker.

## Task mapping

| Finding | Task file | Status |
|---------|-----------|--------|
| Resolve SQLite/JSONL contradiction | `tasks/resolve-sqlite-vs-jsonl-persistence-conflict.md` | **new** |
| Update ProviderProtocol doc to wontfix | `tasks/update-providerprotocol-doc-to-reflect-wontfix-decision.md` | **new** |
| Merge duplicate TUI widget tasks | `tasks/merge-duplicate-tui-widget-replacement-tasks.md` | **new** |
| Merge duplicate session persistence tasks | `tasks/merge-duplicate-session-persistence-tasks.md` | **new** |
| Create unified backlog execution task | `tasks/create-unified-architecture-backlog-execution-task.md` | **new** |
