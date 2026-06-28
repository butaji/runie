# Runie Cleanup Roadmap — 2026-06-28 Architecture & Code Review

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve the unify/simplify/reduce findings from the 2026-06-28 architecture & code review so the workspace builds cleanly, dead/duplicate code is removed, and the actual crate structure matches the documented `IO | Domain (pure) | UI (pure/MVU)` layering.

**Architecture:** The second-pass review showed that several "todo" tasks were already completed on disk (dialog module repaired, empty facade crates deleted, dead provider code removed, IPC layers gone). The remaining work is concentrated in the actor runtime, runtime bootstrap, event taxonomy, tool-parser shim, public API boundary, CLI config routing, and a final sweep of small duplicates. The big actor/bootstrap tasks have been split into sequential sub-tasks so that every intermediate commit leaves `cargo check --workspace` green.

**Tech Stack:** Rust, Tokio, Ratatui, ractor, reqwest.

---

## File structure

- `tasks/index.json` — canonical task registry.
- **Already completed (verified on disk):**
  - `tasks/repair-and-canonicalize-dialog-module.md`
  - `tasks/delete-empty-runie-domain-and-runie-io-crates.md`
  - `tasks/prune-dead-provider-code-and-rig-core-dependency.md`
  - `tasks/deduplicate-provider-registry-data.md`
  - `tasks/remove-dead-ipc-event-abstractions.md`
- **Actor runtime (split into three sequential tasks):**
  - `tasks/migrate-production-actors-to-ractor.md`
  - `tasks/delete-dead-actor-modules-and-custom-trait.md`
  - `tasks/collapse-actor-handles-to-typed-map.md`
- **Runtime bootstrap (split into two sequential tasks):**
  - `tasks/expand-leader-start-for-tui-and-cli.md`
  - `tasks/migrate-tui-and-cli-to-leader-bootstrap.md`
- **Remaining tasks:**
  - `tasks/collapse-event-intent-kind-taxonomies.md` — derive `Intent`/`EventKind` from `Event` without restructuring the enum.
  - `tasks/replace-legacy-tool-parsers-with-thin-shim.md` — introduce `quick-xml` shim alongside legacy parsers, then delete legacy files.
  - `tasks/narrow-runie-core-public-api.md` — usage-audit-first narrowing/moving of internal modules.
  - `tasks/route-cli-config-through-configactor.md` — add ConfigActor messages for CLI inspect/MCP.
  - `tasks/cleanup-small-duplicates-and-dead-code.md` — final sweep after architecture is stable.
- `docs/Architecture.md` — updated with a "Current cleanup roadmap" section.

## Task map

| # | Task ID | Priority | Status | What to do |
|---|---------|----------|--------|------------|
| 1 | `repair-and-canonicalize-dialog-module` | P0 | done | Dialog module wired; duplicate TUI subtree removed. |
| 2 | `delete-empty-runie-domain-and-runie-io-crates` | P1 | done | Empty facade crates deleted. |
| 3 | `prune-dead-provider-code-and-rig-core-dependency` | P1 | done | Dead `catalog/`, `registry/`, `rig_adapter.rs` removed; `rig-core` dropped. |
| 4 | `deduplicate-provider-registry-data` | P2 | done | Only one `registry_data.rs` remains; no duplication. |
| 5 | `remove-dead-ipc-event-abstractions` | P2 | done | `event.rs`, `ipc.rs`, `channels.rs` deleted. |
| 6 | `migrate-production-actors-to-ractor` | P0/P1 | todo | Migrate Config/Provider/Io/Session/Input/Turn/Agent actors to ractor while keeping the custom trait temporarily. |
| 7 | `delete-dead-actor-modules-and-custom-trait` | P1 | todo | Delete custom `Actor` trait and dead actor modules after migration. |
| 8 | `collapse-actor-handles-to-typed-map` | P1 | todo | Collapse `ActorHandles` to a typed `ractor::ActorRef` map. |
| 9 | `expand-leader-start-for-tui-and-cli` | P1 | todo | Make `Leader::start` spawn the full actor set and expose channels/shutdown. |
| 10 | `migrate-tui-and-cli-to-leader-bootstrap` | P1 | todo | Replace manual TUI/CLI bootstrap with `Leader::start`. |
| 11 | `collapse-event-intent-kind-taxonomies` | P1 | todo | Derive `Intent`/`EventKind` from flat `Event`; delete manual mirrors. |
| 12 | `replace-legacy-tool-parsers-with-thin-shim` | P2 | todo | Add `quick-xml` shim, prove equivalence with fixtures, delete legacy parsers. |
| 13 | `route-cli-config-through-configactor` | P2 | todo | Add ConfigActor messages for inspect/MCP; route CLI commands through actor. |
| 14 | `narrow-runie-core-public-api` | P2 | todo | Usage-audit first; move shared helpers to utility crate, narrow the rest. |
| 15 | `cleanup-small-duplicates-and-dead-code` | P3 | todo | Skill hooks, tool registry, TUI render helpers, justified dead-code allows. |

## Execution order

The goal is a **stable phase**: after every merged task the workspace builds and tests pass.

1. **Phase 0 — Close already-done tasks.** Tasks 1–5 are already complete on disk; verify and mark done.
2. **Phase 1 — Actor foundation.**
   - `migrate-production-actors-to-ractor`
   - `delete-dead-actor-modules-and-custom-trait`
   - `collapse-actor-handles-to-typed-map`
3. **Phase 2 — Shared bootstrap.**
   - `expand-leader-start-for-tui-and-cli`
   - `migrate-tui-and-cli-to-leader-bootstrap`
4. **Phase 3 — Event taxonomy.**
   - `collapse-event-intent-kind-taxonomies` (derive-first; do not restructure `Event` yet)
5. **Phase 4 — Tool/provider shims (parallel-safe).**
   - `replace-legacy-tool-parsers-with-thin-shim`
6. **Phase 5 — CLI config (parallel-safe after bootstrap).**
   - `route-cli-config-through-configactor`
7. **Phase 6 — Public API boundary.**
   - `narrow-runie-core-public-api` (must be last architectural change)
8. **Phase 7 — Final sweep.**
   - `cleanup-small-duplicates-and-dead-code`

## Verification

After every task:

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace
```

The final state must satisfy:

- `cargo check --workspace` passes with zero new warnings.
- `cargo test --workspace` passes.
- `runie-domain` and `runie-io` crates no longer exist.
- `rig-core` is not in the dependency graph.
- No module name exists in both `runie-core` and `runie-tui` after the dialog move.
- `runie-core` public API is limited to documented surface types plus legitimately shared utility crates.

## Notes

- The two largest refactorings (actor runtime and event taxonomy) are deliberately incremental. Do not attempt to delete `trait.rs` or restructure the flat `Event` enum in a single commit.
- Several original tasks overlapped with previously-completed work. The plan has been rebased on the actual workspace state; tasks that were already done on disk are now marked done.
- If any task proves larger than expected, split it further and update `tasks/index.json` and this roadmap.
