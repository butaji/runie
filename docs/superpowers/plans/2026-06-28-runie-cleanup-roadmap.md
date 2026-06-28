# Runie Cleanup Roadmap — 2026-06-28 Architecture & Code Review

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve the unify/simplify/reduce findings from the 2026-06-28 architecture & code review so the workspace builds cleanly, dead/duplicate code is removed, and the actual crate structure matches the documented `IO | Domain (pure) | UI (pure/MVU)` layering. A third five-round review added a crate-replacement angle. A fourth five-round review dug deeper into provider/model/catalog/cache, session/store/index/replay, agent turn/subagent/tool search, TUI capabilities/diff/message/markdown, and DSL/view/dialog/commands. Wherever a custom helper mirrors a standard crate, the plan is to use the crate.

**Architecture:** The second-pass review showed that several "todo" tasks were already completed on disk (dialog module repaired, empty facade crates deleted, dead provider code removed, IPC layers gone). Those tasks have been archived. The remaining active work is concentrated in the actor runtime, runtime bootstrap, event taxonomy, tool-parser shim, public API boundary, CLI config routing, and a final sweep of small duplicates. A third review found that many small `runie-core` helpers (`glob`, `fuzzy`, `path`, keybinding parsing, frontmatter scanning, tool-marker stripping) can be replaced by `pulldown-cmark`, `strum`, `shellexpand`, `ignore`, and other standard crates. A fourth review found additional crate replacements in provider/config/auth (`backon`, `keyring`, `jsonschema`, `dotenvy`), CLI (`clap`), TUI widgets (`tui-textarea`, `tui-input`, `ansi_colours`), and tooling (`shell-words`, Clippy/CI). The big actor/bootstrap tasks have been split into sequential sub-tasks so that every intermediate commit leaves `cargo check --workspace` green.

**Tech Stack:** Rust, Tokio, Ratatui, ractor, reqwest, pulldown-cmark, strum, glob/globset, nucleo-matcher/sublime-fuzzy, shellexpand, etcetera, ignore, walkdir, tracing, backon, keyring, jsonschema, clap, dotenvy, shell-words, tui-textarea, tui-input, ansi_colours, notify.

---

## File structure

- `tasks/index.json` — canonical registry of the 33 active cleanup tasks.
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
- **Third-pass crate-replacement tasks (P0/P1):**
  - `tasks/replace-custom-retry-with-backon.md` — delete `runie-provider/src/retry.rs` and use `backon`/`reqwest-retry`.
  - `tasks/replace-xor-auth-with-keyring.md` — store tokens in the OS keyring with a headless fallback.
  - `tasks/replace-config-validator-with-jsonschema.md` — validate `Config` against the generated schema.
  - `tasks/use-clap-derive-for-cli.md` — typed CLI parsing.
  - `tasks/replace-custom-tui-widgets-with-ratatui-ecosystem.md` — `tui-textarea`, `tui-input`, `List`, `ansi_colours`, `crossterm`.
  - `tasks/delete-dead-runie-macros-crate.md` — remove unused proc-macro crate.
  - `tasks/centralize-test-fixtures-and-mocks.md` — shared MiniMax fixtures and mock helpers.
  - `tasks/unify-provider-credential-resolution-with-dotenvy.md` — single `.env` load + provider credential resolution.
  - `tasks/replace-bash-safety-with-shell-words.md` — tokenize with `shell-words` + deny-list.
  - `tasks/replace-build-linter-with-clippy-ci.md` — Clippy lints + CI file-limit check.
- **Third-pass simplification tasks (P1/P2):**
  - `tasks/use-notify-directly-in-config-actor.md` — remove watcher thread bridge.
  - `tasks/unify-provider-config-persistence.md` — single config persistence helper.
  - `tasks/simplify-slash-command-dsl.md` — collapse `CommandSpec`/`CommandDef`.
  - `tasks/unify-permission-system-rules.md` — merge permission rule engines.
- **Fourth-pass provider / model / session unification (P0/P1):**
  - `tasks/unify-session-store-and-index-with-rusqlite.md` — single SQLite store for sessions and replay index.
  - `tasks/type-and-unify-provider-model-layer.md` — typed provider/model structs + single catalog.
  - `tasks/deduplicate-turn-queue-delivery-logic.md` — one queue with explicit delivery ids.
  - `tasks/use-channels-for-subagent-result-collection.md` — channels for subagent results.
- **Fourth-pass parser / markdown unification (P0/P1):**
  - `tasks/unify-markdown-processing-around-pulldown-cmark.md` — single pulldown-cmark event stream.
  - `tasks/replace-think-filter-with-regex.md` — regex-based think-block stripping.
  - `tasks/extract-shared-streaming-response-parser.md` — provider-agnostic streaming parser.
  - `tasks/delete-or-merge-inspector-tool-pipeline.md` — merge or remove duplicate inspector path.
- **Fourth-pass TUI simplification (P2):**
  - `tasks/simplify-terminal-capability-detection.md` — `supports-color`/`supports-hyperlinks`.
  - `tasks/unify-core-and-tui-line-count-computation.md` — one line-count source of truth.
  - `tasks/collapse-dialogstate-variants.md` — small mutually exclusive dialog state machine.
- `docs/Architecture.md` — updated with a "Current cleanup roadmap" section.
- `docs/superpowers/plans/2026-06-28-fourth-pass-crate-review.md` — detailed fourth-pass findings.
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
| 14 | `replace-custom-helpers-with-crates` | P2 | `reopened`. Replacement crates are adopted but the legacy modules still exist; finish deleting them. |
| 15 | `narrow-runie-core-public-api` | P2 | Usage-audit first; move `display_width`, `labels`, `sanitize` to `runie-util`; delete `path`/`fuzzy`/`glob` if helper-crate task lands first; narrow the rest. |
| 16 | `unify-tui-render-test-helpers` | P3 | Move duplicated TUI render helpers into a shared test module. |
| 17 | `fix-keybindings-dead-code` | P3 | Convert `parse_key_combo` to `#[cfg(test)]` or document it. |
| 18 | `cleanup-small-duplicates-and-dead-code` | P3 | `partial`. Skill-hook consolidation, dead actor-handle fields, stale allows, repetitive `FIXME` comments, telemetry-vs-tracing decision. |
| 19 | `replace-custom-retry-with-backon` | P0 | Delete `runie-provider/src/retry.rs`; use `backon`/`reqwest-retry`. |
| 20 | `replace-xor-auth-with-keyring` | P0 | Store tokens in OS keyring with headless fallback. |
| 21 | `replace-config-validator-with-jsonschema` | P0 | Validate `Config` against generated schema. |
| 22 | `unify-provider-credential-resolution-with-dotenvy` | P1 | Load `.env` once; consolidate provider credential resolution. |
| 23 | `unify-provider-config-persistence` | P1 | Single config persistence helper or `RactorConfigActor` owner. |
| 24 | `use-clap-derive-for-cli` | P0 | Typed CLI parsing with `clap`. |
| 25 | `use-notify-directly-in-config-actor` | P1 | Remove watcher thread bridge. |
| 26 | `simplify-slash-command-dsl` | P1 | Collapse `CommandSpec`/`CommandDef`. |
| 27 | `unify-permission-system-rules` | P1 | Merge permission rule engines. |
| 28 | `replace-custom-tui-widgets-with-ratatui-ecosystem` | P0 | `tui-textarea`, `tui-input`, `List`, `ansi_colours`, `crossterm`. |
| 29 | `delete-dead-runie-macros-crate` | P1 | Remove unused proc-macro crate. |
| 30 | `centralize-test-fixtures-and-mocks` | P1 | Shared MiniMax fixtures and mock helpers. |
| 31 | `replace-bash-safety-with-shell-words` | P2 | Tokenize with `shell-words` + deny-list. |
| 32 | `replace-build-linter-with-clippy-ci` | P2 | Clippy lints + CI file-limit check. |
| 33 | `unify-session-store-and-index-with-rusqlite` | P0 | Single SQLite store for sessions, messages, checkpoints, and replay index. |
| 34 | `type-and-unify-provider-model-layer` | P0 | Typed `Provider`/`Model` structs and a single model catalog. |
| 35 | `deduplicate-turn-queue-delivery-logic` | P1 | One turn queue with explicit delivery ids; no duplicate `TurnComplete`. |
| 36 | `use-channels-for-subagent-result-collection` | P0 | Subagent actor sends results on `tokio::sync` channels. |
| 37 | `unify-markdown-processing-around-pulldown-cmark` | P0 | All markdown parsing through one event stream. |
| 38 | `replace-think-filter-with-regex` | P1 | Replace custom `<think>` matcher with `regex`. |
| 39 | `extract-shared-streaming-response-parser` | P1 | Provider-agnostic SSE/JSON/delta parser. |
| 40 | `delete-or-merge-inspector-tool-pipeline` | P1 | Remove or merge the duplicate `inspect` tool path. |
| 41 | `simplify-terminal-capability-detection` | P2 | `supports-color`/`supports-hyperlinks` + single `TermCaps` snapshot. |
| 42 | `unify-core-and-tui-line-count-computation` | P2 | One source of truth for wrapped line counts. |
| 43 | `collapse-dialogstate-variants` | P2 | Small mutually exclusive `DialogState` state machine. |

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
9. **Phase 9 — Provider / config / auth crate replacements.**
   - `replace-custom-retry-with-backon`
   - `replace-xor-auth-with-keyring`
   - `replace-config-validator-with-jsonschema`
   - `unify-provider-credential-resolution-with-dotenvy`
   - `unify-provider-config-persistence`
10. **Phase 10 — CLI / commands / permissions simplification.**
    - `use-clap-derive-for-cli`
    - `use-notify-directly-in-config-actor`
    - `simplify-slash-command-dsl`
    - `unify-permission-system-rules`
11. **Phase 11 — TUI / macros / testing cleanup.**
    - `replace-custom-tui-widgets-with-ratatui-ecosystem`
    - `delete-dead-runie-macros-crate`
    - `centralize-test-fixtures-and-mocks`
12. **Phase 12 — Final tooling simplification.**
    - `replace-bash-safety-with-shell-words`
    - `replace-build-linter-with-clippy-ci`
13. **Phase 13 — Fourth-pass provider / model / session unification.**
    - `unify-session-store-and-index-with-rusqlite`
    - `type-and-unify-provider-model-layer`
    - `deduplicate-turn-queue-delivery-logic` (before subagent channels)
    - `use-channels-for-subagent-result-collection`
14. **Phase 14 — Fourth-pass parser / markdown unification.**
    - `unify-markdown-processing-around-pulldown-cmark` (before think-filter)
    - `replace-think-filter-with-regex`
    - `extract-shared-streaming-response-parser` (after typed provider/model layer)
    - `delete-or-merge-inspector-tool-pipeline`
15. **Phase 15 — Fourth-pass TUI simplification.**
    - `simplify-terminal-capability-detection`
    - `unify-core-and-tui-line-count-computation`
    - `collapse-dialogstate-variants`

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
- Deeper provider/config/auth, CLI/commands/permissions, TUI/widgets, and testing/build findings are documented in [`2026-06-28-third-pass-crate-review.md`](2026-06-28-third-pass-crate-review.md).
- The fourth-pass review (provider/model/catalog/cache, session/store/index/replay, agent turn/subagent/tool search, TUI capabilities/diff/message/markdown, DSL/view/dialog/commands) is documented in [`2026-06-28-fourth-pass-crate-review.md`](2026-06-28-fourth-pass-crate-review.md).
- If any task proves larger than expected, split it further and update `tasks/index.json` and this roadmap.
