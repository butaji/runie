# Runie Cleanup Roadmap — 2026-06-28 Architecture & Code Review

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve the unify/simplify/reduce findings from the 2026-06-28 architecture & code review so the workspace builds cleanly, dead/duplicate code is removed, and the actual crate structure matches the documented `IO | Domain (pure) | UI (pure/MVU)` layering.

**Architecture:** The review found that the codebase is in a mid-refactor state: several previously-completed tasks left facades, unused adapters, and duplicate subsystems behind. The work is grouped into a small set of high-impact cleanup tasks plus one sweep task for small duplicates. Each task lives in `tasks/` with exact files, acceptance criteria, and four-layer tests per `AGENTS.md`.

**Tech Stack:** Rust, Tokio, Ratatui, ractor, reqwest.

---

## File structure

- `tasks/index.json` — canonical task registry; new entries added for every cleanup task.
- `tasks/repair-and-canonicalize-dialog-module.md` — P0 build fix + duplicate dialog subtree removal.
- `tasks/delete-empty-runie-domain-and-runie-io-crates.md` — delete the empty facade crates.
- `tasks/prune-dead-provider-code-and-rig-core-dependency.md` — remove unused provider catalog/registry/rig_adapter and the `rig-core` dependency.
- `tasks/collapse-event-intent-kind-taxonomies.md` — unify the mirrored `Event`/`Intent`/`EventKind` taxonomies.
- `tasks/consolidate-actor-runtime-on-ractor.md` — remove the custom `Actor` trait and actors that are only used in tests.
- `tasks/centralize-runtime-bootstrap-in-leaderactor.md` — route TUI/CLI bootstrap through `Leader::start`.
- `tasks/deduplicate-provider-registry-data.md` — collapse duplicated provider registry YAML structs.
- `tasks/replace-legacy-tool-parsers-with-thin-shim.md` — shrink the deprecated tool-parsing fallback stack.
- `tasks/narrow-runie-core-public-api.md` — make internal modules `pub(crate)`.
- `tasks/route-cli-config-through-configactor.md` — stop CLI inspect/MCP commands from writing config directly.
- `tasks/remove-dead-ipc-event-abstractions.md` — delete unused protocol/IPC event layers.
- `tasks/cleanup-small-duplicates-and-dead-code.md` — small duplicated helpers, dead code, manual trait impls.
- `docs/Architecture.md` — updated with a "Cleanup roadmap" section linking to this plan and the tasks.

## Task map

| # | Task ID | Priority | Existing coverage | What to do |
|---|---------|----------|-------------------|------------|
| 1 | `repair-and-canonicalize-dialog-module` | P0 | `gate-or-move-single-consumer-core-modules` assumes dialog will move later; `unify-duplicate-module-names-core-tui` does not cover dialog. | Wire `runie-core/src/dialog/` back into `lib.rs`, fix 415 `crate::dialog::` references, delete the duplicate `runie-tui/src/dialog/` subtree. |
| 2 | `delete-empty-runie-domain-and-runie-io-crates` | P0/P1 | `split-runie-core-into-domain-and-io-crates` created the facade crates and is marked done. | Delete the now-empty crates and remove them from the workspace. |
| 3 | `prune-dead-provider-code-and-rig-core-dependency` | P1 | `collapse-provider-wrappers` covers wrapper collapse; `unify-provider-stack-with-rig-core` is done but left the adapter. | Delete `runie-provider/src/catalog/`, `registry/`, `rig_adapter.rs`, and drop the `rig-core` dependency. |
| 4 | `collapse-event-intent-kind-taxonomies` | P0/P1 | `event-taxonomy-for-actor-state-sync`, `aggressive-event-consolidation`, `collapse-event-aliases`, `collapse-event-variants-subdir` address pieces. | Make `Intent`/`EventKind` derive from a single `Event` source or nest domain sub-enums; remove the 1:1 manual mirror. |
| 5 | `consolidate-actor-runtime-on-ractor` | P1 | `replace-actor-runtime-with-ractor` is done but the custom `Actor` trait and many dead actors remain. | Delete the custom `Actor` trait, `spawn_actor`, `GenericActorHandle`, and actors not spawned in production. |
| 6 | `centralize-runtime-bootstrap-in-leaderactor` | P1 | `leader-actor-shared-runtime` exists but clients still manually spawn actors. | Route `runie-tui/src/main.rs` and `runie-cli/src/acp.rs` through `Leader::start`. |
| 7 | `deduplicate-provider-registry-data` | P2 | `move-provider-catalog-to-provider-crate` moved the catalog but left duplicate `registry_data.rs`. | Keep one canonical `registry_data.rs` and re-export it. |
| 8 | `replace-legacy-tool-parsers-with-thin-shim` | P2 | `make-mcp-the-only-tool-boundary` is done, but legacy parsers still ship. | Replace MiniMax XML parsing with `quick-xml`, centralize JSON detection, delete legacy/markup/inline parsers. |
| 9 | `narrow-runie-core-public-api` | P2 | No existing task targets the broad public API. | Audit `lib.rs` exports and make internal utilities `pub(crate)`. |
| 10 | `route-cli-config-through-configactor` | P2 | `config-ssot-via-configactor` covers production code but CLI inspect/MCP bypass the actor. | Route `runie-cli/src/inspect.rs` and `mcp.rs` config operations through `ConfigActor`. |
| 11 | `remove-dead-ipc-event-abstractions` | P2 | `fold-protocol-into-core` covers protocol folding; this task targets the unused event-shaping layers. | Delete `runie-protocol/src/event.rs`, `runie-core/src/ipc.rs`, `runie-core/src/channels.rs`. |
| 12 | `cleanup-small-duplicates-and-dead-code` | P3 | Several small duplicates have no dedicated task. | `DynProvider` wrapper, duplicate `now()` helper, duplicated skill hooks, triplicated built-in tool registry, duplicated TUI render test helpers, dead `#[allow(dead_code)]` items, manual derives. |

## Execution order

1. **Task 1** (`repair-and-canonicalize-dialog-module`) must be first — the workspace does not compile without it.
2. **Tasks 2 and 3** are independent deletions; do them early for quick LOC reduction.
3. **Task 4** (`collapse-event-intent-kind-taxonomies`) should follow once the actor runtime is stable; it touches routing tables.
4. **Tasks 5 and 6** are interdependent: finish runtime consolidation before collapsing the bootstrap.
5. **Tasks 7–11** can run in parallel after the build is green.
6. **Task 12** is a background sweep that can be done incrementally.

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
- `rig-core` is no longer in the dependency graph.
- No module name exists in both `runie-core` and `runie-tui` after the dialog move.
- `runie-core/src/lib.rs` public API is limited to documented surface types.

## Notes

- Several findings overlap with existing tasks that are marked `done` (e.g., `split-runie-core-into-domain-and-io-crates`, `unify-provider-stack-with-rig-core`, `replace-actor-runtime-with-ractor`). The new tasks capture the *current* remaining work; if a previously-completed task left a facade or adapter behind, the new task supersedes it for the current branch.
- Each task file contains the exact file paths, code snippets, and test expectations needed for implementation.
- If a task proves larger than expected, split it into follow-up tasks and update `tasks/index.json` and this roadmap.
