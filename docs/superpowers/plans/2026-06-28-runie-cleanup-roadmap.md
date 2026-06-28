# Runie Cleanup Roadmap — 2026-06-28 Architecture & Code Review

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve the unify/simplify/reduce findings from the 2026-06-28 architecture & code review so the workspace builds cleanly, dead/duplicate code is removed, and the actual crate structure matches the documented `IO | Domain (pure) | UI (pure/MVU)` layering. A third five-round review added a crate-replacement angle: wherever a custom helper mirrors a standard crate, plan to use the crate.

**Architecture:** The second-pass review showed that several "todo" tasks were already completed on disk (dialog module repaired, empty facade crates deleted, dead provider code removed, IPC layers gone). Those tasks have been archived. The remaining active work is concentrated in the actor runtime, runtime bootstrap, event taxonomy, tool-parser shim, public API boundary, CLI config routing, and a final sweep of small duplicates. A third review found that many small `runie-core` helpers (`glob`, `fuzzy`, `path`, keybinding parsing, frontmatter scanning, tool-marker stripping) can be replaced by `pulldown-cmark`, `strum`, `shellexpand`, `ignore`, and other standard crates. The big actor/bootstrap tasks have been split into sequential sub-tasks so that every intermediate commit leaves `cargo check --workspace` green.

**Tech Stack:** Rust, Tokio, Ratatui, ractor, reqwest, pulldown-cmark, strum, glob/globset, nucleo-matcher/sublime-fuzzy, shellexpand, etcetera, ignore, walkdir, tracing.

---

## File structure

- `tasks/index.json` — canonical registry of the 18 active cleanup tasks.
- **Actor runtime (split into three sequential tasks):**
  - `tasks/migrate-production-actors-to-ractor.md`
  - `tasks/delete-dead-actor-modules-and-custom-trait.md`
  - `tasks/collapse-actor-handles-to-typed-map.md`
- **Runtime bootstrap (split into two sequential tasks):**
  - `tasks/expand-leader-start-for-tui-and-cli.md`
  - `tasks/migrate-tui-and-cli-to-leader-bootstrap.md`
- **Tooling and DSL quick wins (parallel-safe):**
  - `tasks/replace-legacy-tool-parsers-with-thin-shim.md` — `partial`. Shim routes parsing but still embeds `legacy`/`markup` submodules; inline/delete them, collapse `tool_markers/strip.rs` (fixing the `strip_empty_code_fences` guardrail violation), reconcile MiniMax ownership, and fix the one `cargo check` warning.
  - `tasks/use-pulldown-cmark-for-tool-marker-stripping.md` — rewrite the stripper as a single `pulldown-cmark` event pass instead of regex passes.
  - `tasks/centralize-built-in-tool-names.md` — `done`. Canonical list already exists in `runie-core::tool::BUILTIN_TOOL_NAMES`; switch remaining consumers to it.
  - `tasks/unify-declarative-resource-loader.md` — extract shared directory-scan/frontmatter logic used by `skills/load.rs` and `declarative/loader.rs`; unify the frontmatter-vs-section-fallback policy.
  - `tasks/use-pulldown-cmark-frontmatter-for-resource-loader.md` — replace the custom frontmatter/body scanner with `pulldown-cmark-frontmatter` + `serde_yaml` after the loader is unified.
- **Remaining active tasks:**
  - `tasks/collapse-event-intent-kind-taxonomies.md` — annotate `Event` variants to generate `EventKind`, `EventCategory`, and `Intent`; `Intent` is a semantic projection, not a mirror.
  - `tasks/use-strum-for-event-intent-names.md` — replace manual `names.rs`/`name.rs`/`EVENT_NAMES` tables with `strum` derives after the taxonomy annotation task lands.
  - `tasks/replace-custom-helpers-with-crates.md` — delete `glob.rs`, `fuzzy.rs`, `path.rs`, and custom keybinding parsing; use standard crates.
  - `tasks/narrow-runie-core-public-api.md` — usage-audit-first narrowing/moving of internal modules.
  - `tasks/route-cli-config-through-configactor.md` — extend `RactorConfigActor` for layered config + MCP, then route CLI inspect/MCP through it.
  - `tasks/unify-tui-render-test-helpers.md` — move duplicated TUI render helpers into a shared test module.
  - `tasks/fix-keybindings-dead-code.md` — convert `parse_key_combo` to `#[cfg(test)]` or document it.
  - `tasks/cleanup-small-duplicates-and-dead-code.md` — `partial`. Final sweep; remaining items are skill-hook consolidation, dead actor-handle fields, stale allows, repetitive `FIXME` comments, and telemetry-vs-tracing decision.
- `docs/Architecture.md` — updated with a "Current cleanup roadmap" section.
- `tasks/archive/` — completed tasks from this and earlier reviews.

## Active task map

| # | Task ID | Priority | What to do |
|---|---------|----------|------------|
| 1 | `migrate-production-actors-to-ractor` | P0/P1 | `partial`. `InputActor`/`RactorPermissionActor`/`RactorConfigActor` already migrated and wired; migrate Provider/Io/Session/FffIndexer/Agent actors and update `testing/actor_harness.rs`. |
| 2 | `delete-dead-actor-modules-and-custom-trait` | P1 | Delete custom `Actor` trait, both legacy and ractor variants of dead actors, move `Reply`, replace `GenericActorHandle`, fix `RactorHandle::rpc`, clean `ActorHandles`. |
| 3 | `collapse-actor-handles-to-typed-map` | P1 | Collapse `ActorHandles` to a typed `ractor::ActorRef` map; reconcile with `LeaderHandle`. |
| 4 | `expand-leader-start-for-tui-and-cli` | P1 | Expand `Leader::start` to full actor set; fix `RactorPermissionHandle` type mismatch; default `Leader::new()` to embedded mode. |
| 5 | `migrate-tui-and-cli-to-leader-bootstrap` | P1 | Replace manual bootstrap; remove duplicate `RactorTurnActor` spawn; fix ACP event plumbing and duplicate stdout forwarders. |
| 6 | `collapse-event-intent-kind-taxonomies` | P1 | Annotate `Event` variants to generate `EventKind`, `EventCategory`, and `Intent`; delete `intent_impl.rs`; generate `names.rs`/`name.rs`. |
| 7 | `use-strum-for-event-intent-names` | P1 | Replace manual `names.rs`/`name.rs`/`EVENT_NAMES` tables with `strum` derives. |
| 8 | `replace-legacy-tool-parsers-with-thin-shim` | P2 | `partial`. Inline/delete `legacy`/`markup` submodules; collapse `tool_markers/strip.rs`; fix `strip_empty_code_fences` guardrail; reconcile MiniMax ownership; fix warning. |
| 9 | `use-pulldown-cmark-for-tool-marker-stripping` | P2 | Rewrite the stripper as a single `pulldown-cmark` event pass instead of regex passes. |
| 10 | `centralize-built-in-tool-names` | P2 | `done`. Switch remaining consumers to the canonical `runie-core::tool::BUILTIN_TOOL_NAMES`. |
| 11 | `unify-declarative-resource-loader` | P2 | Extract shared directory-scan/frontmatter logic; unify frontmatter-vs-section-fallback policy. |
| 12 | `use-pulldown-cmark-frontmatter-for-resource-loader` | P2 | Replace custom frontmatter/body scanner with `pulldown-cmark-frontmatter` + `serde_yaml`. |
| 13 | `route-cli-config-through-configactor` | P2 | Extend `RactorConfigActor` for global+project paths, layered config, and MCP ops; route CLI inspect/MCP through it. |
| 14 | `replace-custom-helpers-with-crates` | P2 | Delete `glob.rs`, `fuzzy.rs`, `path.rs`, and custom keybinding parsing; use `glob`/`globset`, `nucleo-matcher`/`sublime-fuzzy`, `shellexpand`, `crossterm`, `tracing`. |
| 15 | `narrow-runie-core-public-api` | P2 | Usage-audit first; move `display_width`, `labels`, `sanitize` to `runie-util`; delete `path`/`fuzzy`/`glob` if helper-crate task lands first; narrow the rest. |
| 16 | `unify-tui-render-test-helpers` | P3 | Move duplicated TUI render helpers into a shared test module. |
| 17 | `fix-keybindings-dead-code` | P3 | Convert `parse_key_combo` to `#[cfg(test)]` or document it. |
| 18 | `cleanup-small-duplicates-and-dead-code` | P3 | `partial`. Skill-hook consolidation, dead actor-handle fields, stale allows, repetitive `FIXME` comments, telemetry-vs-tracing decision. |

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
- `unify-duplicate-module-names-core-tui`

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
   - `collapse-event-intent-kind-taxonomies` (annotate-first; do not restructure `Event` yet)
   - `use-strum-for-event-intent-names` (after annotation lands)
4. **Phase 4 — Tool/provider shims (parallel-safe).**
   - `replace-legacy-tool-parsers-with-thin-shim`
   - `use-pulldown-cmark-for-tool-marker-stripping`
   - `centralize-built-in-tool-names`
5. **Phase 5 — Declarative DSL quick wins (parallel-safe).**
   - `unify-declarative-resource-loader`
   - `use-pulldown-cmark-frontmatter-for-resource-loader` (after loader is unified)
6. **Phase 6 — CLI config (parallel-safe after bootstrap).**
   - `route-cli-config-through-configactor`
7. **Phase 7 — Helper crates and public API boundary.**
   - `replace-custom-helpers-with-crates`
   - `narrow-runie-core-public-api` (must be last architectural change)
8. **Phase 8 — Small safe cleanups.**
   - `unify-tui-render-test-helpers`
   - `fix-keybindings-dead-code`
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
- The plan has been rebased on the actual workspace state; tasks that were already done on disk are now archived and removed from the active registry.
- Crate-replacement rationale and cross-agent lessons are documented in [`2026-06-28-less-code-crate-replacements.md`](2026-06-28-less-code-crate-replacements.md).
- If any task proves larger than expected, split it further and update `tasks/index.json` and this roadmap.
