# Runie Implementation Roadmap

This roadmap reflects the current state after the R3 simplification and the architecture/code review. Task statuses are authoritative in `tasks/index.json`.

## Completed Foundations

R3 crate adoptions, actor/event-bus, tool registry, FFF search, harness skills, TUI rendering pipeline, and redb session persistence are complete. Historical task files for these items live in `tasks/archive/`.

## Active Work — Review Findings

The following issues were identified in the latest architecture/code review and are tracked as new `tasks/` entries. They are listed in priority order.

### Critical

- [ ] `tasks/permission-system-runtime-wiring.md` — wire `PermissionManager`/`ApprovalSink` into the real tool execution path in `runie-agent` and headless/server modes.
- [ ] `tasks/write-file-error-handling.md` — fix `WriteFileTool` so parent-directory creation failures are surfaced and abort the write.
- [ ] `tasks/event-bus-replay-semantics.md` — clone replay events instead of draining the buffer so late subscribers receive history.
- [ ] `tasks/session-replay-startup-ordering.md` — ensure the UI actor subscribes with replay before `SessionActor` publishes durable replay events.

### High

- [ ] `tasks/orchestrator-stub-implementation.md` — implement the planner call and subagent dispatch in `OrchestratorActor`, or gate Team mode as incomplete.
- [ ] `tasks/bash-safety-hardening.md` — replace trivial substring checks with a real shell parser or command allowlist.
- [ ] `tasks/session-store-blocking-io.md` — move `SessionStore` I/O off the async runtime with `tokio::task::spawn_blocking`.
- [ ] `tasks/fff-indexer-blocking-scan.md` — run the FFF picker scan wait in `spawn_blocking`.
- [ ] `tasks/tool-context-env-reduction.md` — default `ToolContext` to a minimal environment instead of capturing full process env.

### Medium

- [ ] `tasks/legacy-tool-enum-removal.md` — delete the orphaned `runie_agent::tools::Tool` enum and consolidate bash logic.
- [ ] `tasks/hashline-edit-skill-apply.md` — make `HashlineEditSkill` actually apply validated edits.
- [ ] `tasks/verification-loop-async.md` — make verification loop asynchronous and remove `unwrap`.
- [ ] `tasks/session-summary-incremental.md` — update session summary incrementally instead of reloading the full event log.
- [ ] `tasks/subagent-async-api.md` — expose `run_subagent` as an `async fn` and remove the nested runtime.
- [ ] `tasks/mock-provider-determinism.md` — use a seeded RNG in `MockProvider` delays.
- [ ] `tasks/build-rs-complexity-heuristic.md` — document or replace the simplistic complexity metric.

### Low / Info

- [ ] `docs/spec-lint-thresholds.md` — resolved; `docs/SPEC.md` now matches enforced 500/40/10 limits.
- [ ] `tasks/orchestrator-event-alias-docs.md` — add doc comment explaining `OrchestratorEvent` alias.
- [ ] `tasks/event-bus-poisoned-mutex.md` — use `parking_lot::Mutex` or handle poisoning gracefully in `EventBus::publish`.
- [ ] `tasks/providers-dialog-clones.md` — reduce unnecessary model string cloning.
- [ ] `tasks/headless-approval-defaults.md` — define safe defaults for non-interactive modes once permission wiring lands.
- [ ] `tasks/agent-registry-depth-tracking.md` — implement real subagent depth tracking.

## Lint Rules (Strict Enforcement)

**All code must comply with these limits — no exceptions:**

| Metric | Limit |
|--------|-------|
| File lines | 500 |
| Function lines | 40 |
| Complexity | 10 |

Production code only is subject to function-length and complexity checks; test functions and files under `tests/` are exempt. Enforced by `crates/runie-core/build.rs`. Build fails on violations.

## Decision Records

- `docs/adr/0017-actor-runtime-and-event-bus.md`
- `docs/adr/0022-harness-middleware-plugins.md`
- `docs/adr/0023-fff-search-integration.md`
- `docs/CRATE_DECISIONS.md`
