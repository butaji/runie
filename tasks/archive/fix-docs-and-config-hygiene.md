# Fix docs and config hygiene

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P3

**Depends on**: none
**Blocks**: none

## Description

Four small docs/config hygiene issues:
- 27 done/superseded `tasks/*.md` files not moved to `tasks/archive/` (per `index.json` statuses).
- `docs/Architecture.md:210-211,253` references deleted `tools.rs`, `parser.rs`, `update/dispatch.rs`.
- `.gitignore` has unrelated entries (`/SuperAGI`, `/autogen`, `/crewAI`, `.ralph/`, `typescript`); missing `__pycache__/`.
- `scripts/verify-tests.sh` `EXPECTED_TOTAL=2271` vs CI `EXPECTED_TOTAL_TESTS=1806` — out of sync.

## Acceptance Criteria

- [ ] 27 done/superseded `tasks/*.md` moved to `tasks/archive/`; `index.json` `file` paths updated to `tasks/archive/...`.
- [ ] `docs/Architecture.md` code-layout block updated: `tools.rs`/`parser.rs` removed, `update/dispatch.rs` reference fixed.
- [ ] `.gitignore` cleaned: unrelated entries removed, `__pycache__/` added.
- [ ] `scripts/verify-tests.sh` `EXPECTED_TOTAL` reconciled with CI `EXPECTED_TOTAL_TESTS` (or the script removed if CI is the source of truth).
- [ ] `cargo check --workspace` succeeds (no code change expected).

## Tests

### Layer 1 — State/Logic
- N/A — docs/config hygiene.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_task_index_paths_resolve` — every `file` in `index.json` resolves on disk after the moves.
- [ ] `smoke_architecture_doc_no_dead_refs` — referenced files in `docs/Architecture.md` exist.

## Files touched

- `tasks/*.md` → `tasks/archive/` (27 files)
- `tasks/index.json`
- `docs/Architecture.md`
- `.gitignore`
- `scripts/verify-tests.sh`

## Notes

The 27 task files: `unify-actor-architecture` (superseded), `permission-system-runtime-wiring`, `write-file-error-handling`, `event-bus-replay-semantics`, `session-replay-startup-ordering`, `orchestrator-stub-implementation`, `bash-safety-hardening`, `session-store-blocking-io`, `fff-indexer-blocking-scan`, `tool-context-env-reduction`, `legacy-tool-enum-removal`, `hashline-edit-skill-apply`, `verification-loop-async`, `session-summary-incremental`, `subagent-async-api`, `mock-provider-determinism`, `build-rs-complexity-heuristic`, `orchestrator-event-alias-docs`, `event-bus-poisoned-mutex`, `providers-dialog-clones`, `headless-approval-defaults`, `agent-registry-depth-tracking`, `arch-guardrails-enforce-3-layers`, `centralize-app-state-ownership`, `remove-io-from-runie-core`, `pure-snapshot-and-tool-runtime-trait`, `consolidate-binary-setup`.
