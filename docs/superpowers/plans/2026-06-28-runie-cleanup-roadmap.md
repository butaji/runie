# Runie Cleanup Roadmap — 2026-06-28 Architecture & Code Review

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve the unify/simplify/reduce findings from the 2026-06-28 architecture & code review so the workspace builds cleanly, dead/duplicate code is removed, and the actual crate structure matches the documented `IO | Domain (pure) | UI (pure/MVU)` layering.

**Architecture:** The second-pass review showed that several "todo" tasks were already completed on disk (dialog module repaired, empty facade crates deleted, dead provider code removed, IPC layers gone). Those tasks have been archived. The remaining active work is concentrated in the actor runtime, runtime bootstrap, event taxonomy, tool-parser shim, public API boundary, CLI config routing, and a final sweep of small duplicates. The big actor/bootstrap tasks have been split into sequential sub-tasks so that every intermediate commit leaves `cargo check --workspace` green.

**Tech Stack:** Rust, Tokio, Ratatui, ractor, reqwest.

---

## File structure

- `tasks/index.json` — canonical registry of the 11 active cleanup tasks.
- **Actor runtime (split into three sequential tasks):**
  - `tasks/migrate-production-actors-to-ractor.md`
  - `tasks/delete-dead-actor-modules-and-custom-trait.md`
  - `tasks/collapse-actor-handles-to-typed-map.md`
- **Runtime bootstrap (split into two sequential tasks):**
  - `tasks/expand-leader-start-for-tui-and-cli.md`
  - `tasks/migrate-tui-and-cli-to-leader-bootstrap.md`
- **Remaining active tasks:**
  - `tasks/collapse-event-intent-kind-taxonomies.md` — derive `Intent`/`EventKind` from `Event` without restructuring the enum.
  - `tasks/replace-legacy-tool-parsers-with-thin-shim.md` — introduce `quick-xml` shim alongside legacy parsers, then delete legacy files.
  - `tasks/narrow-runie-core-public-api.md` — usage-audit-first narrowing/moving of internal modules.
  - `tasks/route-cli-config-through-configactor.md` — add ConfigActor messages for CLI inspect/MCP.
  - `tasks/cleanup-small-duplicates-and-dead-code.md` — final sweep after architecture is stable.
- `docs/Architecture.md` — updated with a "Current cleanup roadmap" section.
- `tasks/archive/` — completed tasks from this and earlier reviews.

## Active task map

| # | Task ID | Priority | What to do |
|---|---------|----------|------------|
| 1 | `migrate-production-actors-to-ractor` | P0/P1 | `InputActor`/`RactorPermissionActor` already migrated; wire `RactorConfigActor` and migrate Provider/Io/Session/FffIndexer/Agent actors. |
| 2 | `delete-dead-actor-modules-and-custom-trait` | P1 | Delete custom `Actor` trait, dead actor modules, and their `ActorHandles` fields after migration. |
| 3 | `collapse-actor-handles-to-typed-map` | P1 | Collapse `ActorHandles` to a typed `ractor::ActorRef` map; reconcile with `LeaderHandle`. |
| 4 | `expand-leader-start-for-tui-and-cli` | P1 | Make `Leader::start` spawn the full actor set (Input/Agent/FffIndexer) and expose channels/shutdown. |
| 5 | `migrate-tui-and-cli-to-leader-bootstrap` | P1 | Replace manual TUI/CLI bootstrap with `Leader::start`; route CLI input through `InputMsg`. |
| 6 | `collapse-event-intent-kind-taxonomies` | P1 | Derive `Intent`/`EventKind` from flat `Event`; delete manual mirrors. |
| 7 | `replace-legacy-tool-parsers-with-thin-shim` | P2 | Add `quick-xml` shim, prove equivalence with `runie-testing` fixtures, delete legacy parsers, reconcile MiniMax parsing with `runie-provider`. |
| 8 | `route-cli-config-through-configactor` | P2 | Add `RactorConfigActor` messages for inspect/MCP; route CLI commands through the actor. |
| 9 | `narrow-runie-core-public-api` | P2 | Usage-audit first; move shared helpers to a new `runie-util` crate, narrow the rest. |
| 10 | `cleanup-small-duplicates-and-dead-code` | P3 | Skill hooks, tool registry, TUI render helpers, justified dead-code allows. |
| 11 | `unify-duplicate-module-names-core-tui` | P2 | Markdown rename done; rename core `themes.rs` to `theme_tokens.rs` to remove the remaining collision. |

## Archived completed tasks

The following tasks from the 2026-06-28 review were already complete on disk and have been moved to `tasks/archive/`:

- `repair-and-canonicalize-dialog-module`
- `delete-empty-runie-domain-and-runie-io-crates`
- `prune-dead-provider-code-and-rig-core-dependency`
- `deduplicate-provider-registry-data`
- `remove-dead-ipc-event-abstractions`
- `merge-diff-modules`
- `rename-core-ui-to-view`
- `inline-tui-ipc-reexport`
- `fold-protocol-into-core`

Earlier completed work (actor SSOT, config SSOT, MCP adoption, actor migrations, etc.) is also preserved in `tasks/archive/`.

## Execution order

The goal is a **stable phase**: after every merged task the workspace builds and tests pass.

1. **Phase 1 — Actor foundation.**
   - `migrate-production-actors-to-ractor`
   - `delete-dead-actor-modules-and-custom-trait`
   - `collapse-actor-handles-to-typed-map`
2. **Phase 2 — Shared bootstrap.**
   - `expand-leader-start-for-tui-and-cli`
   - `migrate-tui-and-cli-to-leader-bootstrap`
3. **Phase 3 — Event taxonomy.**
   - `collapse-event-intent-kind-taxonomies` (derive-first; do not restructure `Event` yet)
4. **Phase 4 — Tool/provider shims (parallel-safe).**
   - `replace-legacy-tool-parsers-with-thin-shim`
5. **Phase 5 — CLI config (parallel-safe after bootstrap).**
   - `route-cli-config-through-configactor`
6. **Phase 6 — Public API boundary.**
   - `narrow-runie-core-public-api` (must be last architectural change)
7. **Phase 7 — Final sweep.**
   - `cleanup-small-duplicates-and-dead-code`
   - `unify-duplicate-module-names-core-tui`

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
- The plan has been rebased on the actual workspace state; tasks that were already done on disk are now archived and removed from the active registry.
- If any task proves larger than expected, split it further and update `tasks/index.json` and this roadmap.
