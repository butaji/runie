# Runie Cleanup Roadmap — 2026-06-28 Architecture & Code Review

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve the unify/simplify/reduce findings from the 2026-06-28 architecture & code review so the workspace builds cleanly, dead/duplicate code is removed, and the actual crate structure matches the documented `IO | Domain (pure) | UI (pure/MVU)` layering. A third five-round review added a crate-replacement angle. A fourth five-round review dug deeper into provider/model/catalog/cache, session/store/index/replay, agent turn/subagent/tool search, TUI capabilities/diff/message/markdown, and DSL/view/dialog/commands. A fifth five-round review focused on build/CI/test harness, error handling/tracing/telemetry, protocol/IPC leftovers, declarative loaders/DSLs, and macros/codegen. Wherever a custom helper mirrors a standard crate, the plan is to use the crate.

**Architecture:** The second-pass review showed that several "todo" tasks were already completed on disk (dialog module repaired, empty facade crates deleted, dead provider code removed, IPC layers gone). Those tasks have been archived. A third review found that many small `runie-core` helpers (`glob`, `fuzzy`, `path`, keybinding parsing, frontmatter scanning, tool-marker stripping) can be replaced by `pulldown-cmark`, `strum`, `shellexpand`, `ignore`, and other standard crates. A fourth review found additional crate replacements in provider/config/auth (`backon`, `keyring`, `jsonschema`, `dotenvy`), CLI (`clap`), TUI widgets (`tui-textarea`, `tui-input`, `ansi_colours`), and tooling (`shell-words`, Clippy/CI). A fifth review found broken CI feature flags, dead MCP feature flag, dependency hygiene issues, library-wide `anyhow` usage without typed errors, missing `tracing` subscriber, custom telemetry with no backend, broken ACP plumbing, a still-existing `runie-protocol` crate, and many macros/small parsers that should use `strum`. **As of 2026-06-29, all 108 cleanup tasks from the original architecture review are `done` and the workspace passes `cargo check --workspace` and `cargo test --workspace` cleanly.** The remaining 3 tasks are marked `wontfix` (tool-marker stripping intentionally string-based, pre-release `notify` dependency issue, `palette` adoption deferred). Live mock/MiniMax TUI testing on 2026-06-30, three five-round architecture/code reviews, a deep crate-replacement analysis, and a planned Grok Build comparison uncovered additional bugs and parity gaps; those are tracked as active `todo` tasks plus `partial` tasks in `tasks/index.json`. As of 2026-07-02, 338 tasks are `done`, 8 are `wontfix`, 4 are `partial`, 5 are `blocked`, and 57 are `todo` (total 412). Recent task closures: use-textwrap-for-blockquote-word-wrap (textwrap already used in blockquote rendering), use-atomic-writes-for-config-and-session-files (atomic_write implemented in io/atomic_write.rs), adopt-snapshot-journal-jsonl-pattern (session store already uses JSONL with fs2 advisory locks), generate-tool-dispatch-from-single-registry (BUILTIN_TOOL_NAMES centralized), simplify-permission-policy-chain-to-ruleset-match (PermissionSet ruleset implemented), replace-layered-config-merge-with-figment (Figment already used for config layers). Recent closures: 062-add-tool-result-cache-expiry (tool result cache with TTL expiry in runie-core/tool/cache.rs). Recent closures: await-provider-network-calls-in-ractor-handler (await network calls inline in RactorProviderActor), centralize-provider-http-timeouts-and-retry-constants, centralize-reqwest-client-and-url-normalization, standardize-session-persistence-on-jsonl, derive-event-taxonomies-with-strum-or-proc-macro, unify-bash-execution-into-single-shell-module-with-command-group, unify-form-submit-paths-through-command-registry, unify-sse-parsing-on-openai-frame, replace-emitfn-mutex-with-async-channel, single-config-resolve-default-model, remove-direct-projection-bypasses-in-dispatch-and-domain-ops (use set_git_info/set_cwd_name from domain_ops instead of direct accessor mutation).

**Tech Stack:** Rust, Tokio, Ratatui, ractor, reqwest, pulldown-cmark, strum, glob/globset, nucleo-matcher/sublime-fuzzy, shellexpand, etcetera, ignore, walkdir, tracing, backon, keyring, jsonschema, clap, dotenvy, shell-words, tui-textarea, tui-input, ansi_colours, notify.

**TUI design freeze:** TUI *behavior* may change (key handling, event ordering, redraw pacing, error surfacing), but TUI *element design and composition* (layouts, widgets, colors, spacing, status bar shape, message list look, popup layout, theme palette) stays frozen unless a task explicitly scopes a visual redesign. Any crate/widget replacement must reproduce the existing visual output.

---

## Status: Active bugs from live TUI/CLI testing, architecture review, and Grok Build comparison

The original cleanup is complete: **243 `done`**, **6 `wontfix`**, **93 `todo`**, **5 `blocked`**, **2 `partial`** (total 349 tasks). A 2026-06-30 five-round architecture/code review produced 13 new `todo` tasks ([`2026-06-30-five-round-architecture-review.md`](2026-06-30-five-round-architecture-review.md)). A deep crate-replacement analysis added 16 more ([`2026-06-30-crate-replacement-deep-dive.md`](2026-06-30-crate-replacement-deep-dive.md)). A second five-round review added another 16 `todo` tasks ([`2026-06-30-second-five-round-architecture-review.md`](2026-06-30-second-five-round-architecture-review.md)). A third five-round review added another 20 `todo` tasks ([`2026-06-30-third-five-round-architecture-review.md`](2026-06-30-third-five-round-architecture-review.md)). A fourth five-round review added another 21 `todo` tasks ([`2026-06-30-fourth-five-round-architecture-review.md`](2026-06-30-fourth-five-round-architecture-review.md)). A fifth five-round review added another 25 `todo` tasks ([`2026-06-30-fifth-five-round-architecture-review.md`](2026-06-30-fifth-five-round-architecture-review.md)). A sixth five-round review added another 20 `todo` tasks ([`2026-06-30-sixth-five-round-architecture-review.md`](2026-06-30-sixth-five-round-architecture-review.md)). A seventh five-round review added another 24 `todo` tasks and archived one duplicate ([`2026-06-30-seventh-five-round-architecture-review.md`](2026-06-30-seventh-five-round-architecture-review.md)). An eighth five-round review added another 25 `todo` tasks and fixed backlog drift ([`2026-06-30-eighth-five-round-architecture-review.md`](2026-06-30-eighth-five-round-architecture-review.md)).

- **57 `todo` tasks** — see `tasks/index.json` for details.
- **5 `blocked` tasks** — Grok Build comparison tasks blocked on fixture/recorder/harness readiness; see `tasks/index.json`.
- **4 `partial` tasks** — tasks with partial implementation; see `tasks/index.json`.
- **8 `wontfix` tasks** — see `tasks/index.json` for the current list.

**Recent closures:** `fix-tui-mock-simple-text-response-repetition` and `fix-tui-turn-complete-leaves-working-status-and-queued` are now `done`. Root cause: `agent_running` guard cleared on `Done` instead of `TurnCompleted`, causing double agent spawn and infinite output.

Recent closures: `remove-direct-projection-bypasses-in-dispatch-and-domain-ops` — use `set_git_info`/`set_cwd_name` from `domain_ops.rs` instead of direct `git_info_mut()`/`cwd_name_mut()` mutation in `dispatch.rs`.

The workspace still passes `cargo check --workspace` and `cargo test --workspace`, but the TUI is not yet usable end-to-end. The highest-priority fixes are:
1. `remove-direct-appstate-mutation-from-tui-handlers` — unblock actor-SSOT event-driven sync.
2. `subscribe-tui-to-initial-facts-before-leader-start` — fix empty config/history at launch.
3. `fix-tui-mock-simple-text-response-repetition` — infinite `hello` echo loop.
4. `fix-tui-turn-complete-leaves-working-status-and-queued` — turn completes but status stays `Working...`.
5. `fix-cli-headless-native-tool-permission-denied-loop` — headless loops on denied tools.

Remaining items:
- **3 `wontfix` tasks**: tool-marker stripping (string-based by design), `fff-search` pre-release `notify` (deferred), `palette` color math adoption (deferred).

See `tasks/index.json` for the full registry and `tasks/` for individual task files with acceptance-criteria evidence. The final closure, including which of the last 24 tasks were completed and which 3 are intentionally `wontfix`, is documented in [`2026-06-28-active-tasks-closure.md`](2026-06-28-active-tasks-closure.md).
  - `tasks/unify-provider-config-persistence.md` — single config persistence helper.
  - `tasks/simplify-slash-command-dsl.md` — collapse `CommandSpec`/`CommandDef`.
  - `tasks/unify-permission-system-rules.md` — merge permission rule engines.
- **Fourth-pass provider / model / session unification (P0/P1):**
  - `tasks/unify-session-store-and-index.md` — single headered JSONL file with `fs2` advisory locks for sessions and replay index. SQLite is deferred.
  - `tasks/type-and-unify-provider-model-layer.md` — typed provider/model structs + single catalog.
  - `tasks/dedupe-turn-queue-delivery-logic.md` — one queue with explicit delivery ids.
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
- **Fifth-pass CI / build / dependency hygiene (P0/P1):**
  - `tasks/fix-broken-schema-feature-in-ci-and-justfile.md` — remove broken `--features schema` usage.
  - `tasks/delete-or-fix-dead-mcp-feature-flag.md` — gate or delete the empty `mcp` feature.
  - `tasks/remove-unused-dependencies-and-normalize-workspace-deps.md` — remove `futures`, dedupe `tempfile`, workspace-inherit inline deps.
  - `tasks/fix-dev-automation-recipes.md` — fix `bacon.toml` `test` job and `just lint-fix`.
  - `tasks/replace-build-linter-with-clippy-ci.md` — Clippy + CI file-limit check.
  - `tasks/introduce-cargo-deny-and-cargo-machete-ci.md` — dependency audits in CI.
- **Fifth-pass error handling / observability (P0/P1):**
  - `tasks/unify-library-error-types-with-thiserror.md` — typed library errors.
  - `tasks/initialize-tracing-subscriber-in-binaries.md` — initialize `tracing-subscriber`.
  - `tasks/replace-custom-telemetry-with-tracing-layer.md` — replace telemetry collector with tracing layer.
  - `tasks/harden-actors-against-mutex-poisoning.md` — safe mutex usage in actors.
  - `tasks/eliminate-production-unwrap-expect.md` — recoverable errors instead of panics.
- **Fifth-pass protocol / IPC cleanup (P1):**
  - `tasks/fold-runie-protocol-into-core.md` — fold the still-existing protocol crate into `runie-core`.
  - `tasks/unify-cli-json-rpc-transport-and-remove-dead-acp.md` — shared transport; fix/delete ACP.
- **Fifth-pass declarative loaders / DSL / parser cleanup (P1/P2):**
  - `tasks/migrate-subagent-loader-to-resource-loader.md` — use shared loader for subagent types.
  - `tasks/fix-dry-run-tool-names-discrepancy.md` — align dry-run tool list with canonical names.
  - `tasks/replace-remaining-custom-parsers-and-macros-with-strum.md` — strum derives for small enums/macros.
  - `tasks/implement-or-remove-mcp-runtime-scaffolding.md` — implement `rmcp` client or delete scaffolding.
- **Fifth-pass test harness hardening (P2):**
  - `tasks/replace-tmux-test-with-ratatui-tests.md` — delete shell/tmux test.
  - `tasks/remove-sleep-from-automatic-tests.md` — deterministic waits.
- **Post-review actor runtime hardening (P0/P1):**
  - `tasks/fix-ractor-permission-actor-reply-lifecycle.md` — stop auto-denying permission requests.
  - `tasks/reconnect-tui-agent-actor-channel.md` — fix disconnected TUI agent channel.
  - `tasks/use-ractor-state-for-actor-mutable-state.md` — idiomatic `ractor` `State`.
  - `tasks/wire-or-delete-input-actor.md` — remove orphan input actor.
  - `tasks/delete-dead-actor-handle-wrappers.md` — delete legacy handle structs.
  - `tasks/actually-collapse-actor-handles-to-typed-map.md` — finish handle collapse.
- **Post-review queue/mutex normalization (P1/P2):**
  - `tasks/remove-sync-turn-queue-fallback-from-app-state.md` — delete sync fallback.
  - `tasks/normalize-remaining-std-mutex-to-parking_lot.md` — std locks → parking_lot.
  - `tasks/delete-eventbusbridge-wrapper.md` — delete thin wrapper.
  - `tasks/use-channels-for-subagent-result-collection.md` — channels for subagents.
- **Post-review provider/config/security (P1/P2):**
  - `tasks/store-provider-api-keys-in-keyring-not-config.md` — keyring for API keys.
  - `tasks/add-jsonschema-validation-to-configactor-load.md` — validate at load time.
  - `tasks/actually-replace-runie-provider-backoff-with-backon.md` — finish retry.
  - `tasks/drop-unused-rand-from-runie-provider.md` — deterministic mocks.
- **Post-review TUI/markdown unification (P0/P2):**
  - `tasks/replace-tui-markdown-block-layout-with-tui-markdown.md` — use `tui-markdown`.
  - `tasks/unify-markdown-block-parsing-on-pulldown-cmark-events.md` — event-driven markdown.
  - `tasks/unify-tui-message-wrapping-with-textwrap.md` — display-width wrapping.
  - `tasks/replace-tui-keymap-combo-stringification-with-crokey.md` — `crokey` combos.
  - `tasks/rescope-terminal-capability-task-to-current-module.md` — update task scope.
- **Post-review declarative loaders / build / CI (P2/P3):**
  - `tasks/deserialize-declarative-command-yaml-with-typed-structs.md` — typed YAML.
  - `tasks/move-built-in-slash-commands-to-declarative-yaml.md` — YAML built-ins.
  - `tasks/replace-session-tree-with-arena-crate.md` — arena tree.
  - `tasks/move-cargo-deny-machete-from-workspace-deps-to-ci.md` — binaries in CI.
  - `tasks/extract-shared-tracing-subscriber-init.md` — shared subscriber init.
  - `tasks/cleanup-verify-tests-and-just-recipes.md` — dev recipe fixes.
  - `tasks/remove-redundant-check-field-access-script.md` — duplicate script.
  - `tasks/replace-leftover-macros-with-functions.md` — macro cleanup.
  - `tasks/delete-unused-glyph-constants.md` — dead glyphs.
- `docs/Architecture.md` — updated with a "Current cleanup roadmap" section.
- `docs/superpowers/plans/2026-06-28-fourth-pass-crate-review.md` — detailed fourth-pass findings.
- `docs/superpowers/plans/2026-06-28-fifth-pass-crate-review.md` — detailed fifth-pass findings.
- `docs/superpowers/plans/2026-06-28-five-round-review-synthesis.md` — post-implementation gaps and peer learnings.
- `docs/superpowers/plans/2026-06-28-task-verification-report.md` — verification of whether tasks marked `done` are truly complete.
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
| 14 | `replace-custom-helpers-with-crates` | P2 | `done`. `glob.rs`, `fuzzy.rs`, `path.rs` deleted; `shellexpand`/`nucleo-matcher`/`sublime-fuzzy`/`glob` used directly; `parse_key_combo` moved to `#[cfg(test)]`. |
| 15 | `narrow-runie-core-public-api` | P2 | Usage-audit first; move `display_width`, `labels`, `sanitize` to `runie-util`; delete `path`/`fuzzy`/`glob` if helper-crate task lands first; narrow the rest. |
| 16 | `unify-tui-render-test-helpers` | P3 | Move duplicated TUI render helpers into a shared test module. |
| 17 | `fix-keybindings-dead-code` | P3 | Convert `parse_key_combo` to `#[cfg(test)]` or document it. |
| 18 | `cleanup-small-duplicates-and-dead-code` | P3 | `partial`. Skill-hook consolidation, dead actor-handle fields, stale allows, repetitive `FIXME` comments, telemetry-vs-tracing decision. |
| 19 | `replace-custom-retry-with-backon` | P0 | Delete `runie-provider/src/retry.rs`; use `backon`/`reqwest-retry`. |
| 20 | `replace-xor-auth-with-keyring` | P0 | Store tokens in OS keyring with headless fallback. |
| 21 | `replace-config-validator-with-jsonschema` | P0 | `partial`. Hand-written validator replaced with `jsonschema`; `validate_registry` remains inline. |
| 22 | `unify-provider-credential-resolution-with-dotenvy` | P1 | Load `.env` once; consolidate provider credential resolution. |
| 23 | `unify-provider-config-persistence` | P1 | Single config persistence helper or `RactorConfigActor` owner. |
| 24 | `use-clap-derive-for-cli` | P0 | Typed CLI parsing with `clap`. |
| 25 | `use-notify-directly-in-config-actor` | P1 | Remove watcher thread bridge. |
| 26 | `simplify-slash-command-dsl` | P1 | Collapse `CommandSpec`/`CommandDef`. |
| 27 | `unify-permission-system-rules` | P1 | Merge permission rule engines. |
| 28 | `replace-custom-tui-widgets-with-ratatui-ecosystem` | P0 | `tui-textarea`, `tui-input`, `List`, `ansi_colours`, `crossterm`. |
| 29 | `delete-dead-runie-macros-crate` | P1 | `done`. The crate no longer exists in the workspace. |
| 30 | `centralize-test-fixtures-and-mocks` | P1 | Shared MiniMax fixtures and mock helpers; also consolidate env-lock/temp-config helpers. |
| 31 | `replace-bash-safety-with-shell-words` | P2 | Tokenize with `shell-words` + deny-list. |
| 32 | `replace-build-linter-with-clippy-ci` | P2 | Clippy lints + CI file-limit check. |
| 33 | `unify-session-store-and-index` | P0 | Single headered JSONL file with `fs2` advisory locks for sessions and replay index. SQLite deferred. |
| 34 | `type-and-unify-provider-model-layer` | P0 | Typed `Provider`/`Model` structs and a single model catalog. |
| 35 | `dedupe-turn-queue-delivery-logic` | P1 | One turn queue with explicit delivery ids; no duplicate `TurnComplete`. |
| 36 | `use-channels-for-subagent-result-collection` | P0 | Subagent actor sends results on `tokio::sync` channels. |
| 37 | `unify-markdown-processing-around-pulldown-cmark` | P0 | All markdown parsing through one event stream. |
| 38 | `replace-think-filter-with-regex` | P1 | Replace custom `<think>` matcher with `regex`. |
| 39 | `extract-shared-streaming-response-parser` | P1 | Provider-agnostic SSE/JSON/delta parser. |
| 40 | `delete-or-merge-inspector-tool-pipeline` | P1 | Remove or merge the duplicate `inspect` tool path. |
| 41 | `simplify-terminal-capability-detection` | P2 | `supports-color`/`supports-hyperlinks` + single `TermCaps` snapshot. |
| 42 | `unify-core-and-tui-line-count-computation` | P2 | One source of truth for wrapped line counts. |
| 43 | `collapse-dialogstate-variants` | P2 | Small mutually exclusive `DialogState` state machine. |
| 44 | `fix-broken-schema-feature-in-ci-and-justfile` | P0 | Remove broken `--features schema` from CI and justfile. |
| 45 | `delete-or-fix-dead-mcp-feature-flag` | P0 | Gate or delete the empty `mcp` feature. |
| 46 | `remove-unused-dependencies-and-normalize-workspace-deps` | P1 | Remove `futures`, dedupe `tempfile`, workspace-inherit inline deps. |
| 47 | `fix-dev-automation-recipes` | P1 | Fix `bacon.toml` `test` job and `just lint-fix`. |
| 48 | `replace-build-linter-with-clippy-ci` | P2 | Clippy lints + CI file-limit check. |
| 49 | `unify-library-error-types-with-thiserror` | P0 | Typed library errors via `thiserror`. |
| 50 | `initialize-tracing-subscriber-in-binaries` | P0 | Initialize `tracing-subscriber` in CLI/TUI. |
| 51 | `replace-custom-telemetry-with-tracing-layer` | P1 | Replace telemetry collector with `tracing` layer. |
| 52 | `harden-actors-against-mutex-poisoning` | P1 | Safe mutex usage in actors. |
| 53 | `fold-runie-protocol-into-core` | P1 | Fold the still-existing protocol crate into `runie-core`. |
| 54 | `unify-cli-json-rpc-transport-and-remove-dead-acp` | P1 | Shared transport; fix or delete ACP. |
| 55 | `migrate-subagent-loader-to-resource-loader` | P1 | Use shared loader for subagent types. |
| 56 | `fix-dry-run-tool-names-discrepancy` | P0 | Align dry-run tool list with `BUILTIN_TOOL_NAMES`. |
| 57 | `replace-remaining-custom-parsers-and-macros-with-strum` | P2 | `strum` derives for small enums/macros. |
| 58 | `implement-or-remove-mcp-runtime-scaffolding` | P2 | Implement `rmcp` client or delete scaffolding. |
| 59 | `replace-tmux-test-with-ratatui-tests` | P2 | Delete shell/tmux test; port coverage to Rust. |
| 60 | `remove-sleep-from-automatic-tests` | P2 | Deterministic waits instead of `sleep()`. |
| 61 | `eliminate-production-unwrap-expect` | P2 | Recoverable errors instead of production panics. |
| 62 | `introduce-cargo-deny-and-cargo-machete-ci` | P2 | Dependency audits in CI. |
| 63 | `fix-ractor-permission-actor-reply-lifecycle` | P0 | Stop auto-denying; resolve pending requests. |
| 64 | `reconnect-tui-agent-actor-channel` | P0 | Fix disconnected TUI agent channel. |
| 65 | `use-ractor-state-for-actor-mutable-state` | P1 | Idiomatic `ractor` `State`. |
| 66 | `wire-or-delete-input-actor` | P1 | Remove orphan input actor. |
| 67 | `delete-dead-actor-handle-wrappers` | P1 | Delete legacy handle structs. |
| 68 | `actually-collapse-actor-handles-to-typed-map` | P1 | Finish handle collapse. |
| 69 | `remove-sync-turn-queue-fallback-from-app-state` | P1 | Delete sync fallback. |
| 70 | `normalize-remaining-std-mutex-to-parking_lot` | P1 | std locks → parking_lot. |
| 71 | `delete-eventbusbridge-wrapper` | P2 | Delete thin wrapper. |
| 72 | `use-channels-for-subagent-result-collection` | P0 | Channels for subagents. |
| 73 | `store-provider-api-keys-in-keyring-not-config` | P1 | Keyring for API keys. |
| 74 | `add-jsonschema-validation-to-configactor-load` | P2 | Validate at load time. |
| 75 | `actually-replace-runie-provider-backoff-with-backon` | P2 | Finish retry. |
| 76 | `drop-unused-rand-from-runie-provider` | P2 | Deterministic mocks. |
| 77 | `replace-tui-markdown-block-layout-with-tui-markdown` | P0 | Use `tui-markdown`. |
| 78 | `unify-markdown-block-parsing-on-pulldown-cmark-events` | P1 | Event-driven markdown. |
| 79 | `unify-tui-message-wrapping-with-textwrap` | P1 | Display-width wrapping. |
| 80 | `replace-tui-keymap-combo-stringification-with-crokey` | P1 | `crokey` combos. |
| 81 | `rescope-terminal-capability-task-to-current-module` | P2 | Update task scope. |
| 82 | `deserialize-declarative-command-yaml-with-typed-structs` | P2 | Typed YAML. |
| 83 | `move-built-in-slash-commands-to-declarative-yaml` | P2 | YAML built-ins. |
| 84 | `replace-session-tree-with-arena-crate` | P2 | Arena tree. |
| 85 | `move-cargo-deny-machete-from-workspace-deps-to-ci` | P2 | Binaries in CI. |
| 86 | `extract-shared-tracing-subscriber-init` | P2 | Shared subscriber init. |
| 87 | `cleanup-verify-tests-and-just-recipes` | P3 | Dev recipe fixes. |
| 88 | `remove-redundant-check-field-access-script` | P3 | Duplicate script. |
| 89 | `replace-leftover-macros-with-functions` | P3 | Macro cleanup. |
| 90 | `delete-unused-glyph-constants` | P3 | Dead glyphs. |

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
    - `unify-session-store-and-index`
    - `type-and-unify-provider-model-layer`
    - `dedupe-turn-queue-delivery-logic` (before subagent channels)
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
16. **Phase 16 — Fifth-pass CI / build / dependency hygiene.**
    - `fix-broken-schema-feature-in-ci-and-justfile`
    - `delete-or-fix-dead-mcp-feature-flag`
    - `remove-unused-dependencies-and-normalize-workspace-deps`
    - `fix-dev-automation-recipes`
    - `replace-build-linter-with-clippy-ci`
    - `introduce-cargo-deny-and-cargo-machete-ci`
17. **Phase 17 — Fifth-pass error handling / observability.**
    - `unify-library-error-types-with-thiserror` (before unwrap cleanup)
    - `initialize-tracing-subscriber-in-binaries`
    - `replace-custom-telemetry-with-tracing-layer`
    - `harden-actors-against-mutex-poisoning`
    - `eliminate-production-unwrap-expect`
18. **Phase 18 — Fifth-pass protocol / IPC cleanup.**
    - `fold-runie-protocol-into-core` (before transport unification)
    - `unify-cli-json-rpc-transport-and-remove-dead-acp`
19. **Phase 19 — Fifth-pass declarative loaders / DSL / parser cleanup.**
    - `migrate-subagent-loader-to-resource-loader`
    - `fix-dry-run-tool-names-discrepancy`
    - `replace-remaining-custom-parsers-and-macros-with-strum`
    - `implement-or-remove-mcp-runtime-scaffolding`
20. **Phase 20 — Fifth-pass test harness hardening.**
    - `replace-tmux-test-with-ratatui-tests`
    - `remove-sleep-from-automatic-tests`
21. **Phase 21 — Post-review actor runtime hardening.**
    - `fix-ractor-permission-actor-reply-lifecycle`
    - `reconnect-tui-agent-actor-channel`
    - `use-ractor-state-for-actor-mutable-state`
    - `wire-or-delete-input-actor`
    - `delete-dead-actor-handle-wrappers`
    - `actually-collapse-actor-handles-to-typed-map`
22. **Phase 22 — Post-review queue/mutex normalization.**
    - `remove-sync-turn-queue-fallback-from-app-state`
    - `normalize-remaining-std-mutex-to-parking_lot`
    - `delete-eventbusbridge-wrapper`
    - `use-channels-for-subagent-result-collection`
23. **Phase 23 — Post-review provider/config/security.**
    - `store-provider-api-keys-in-keyring-not-config`
    - `add-jsonschema-validation-to-configactor-load`
    - `actually-replace-runie-provider-backoff-with-backon`
    - `drop-unused-rand-from-runie-provider`
24. **Phase 24 — Post-review TUI/markdown unification.**
    - `replace-tui-markdown-block-layout-with-tui-markdown`
    - `unify-markdown-block-parsing-on-pulldown-cmark-events`
    - `unify-tui-message-wrapping-with-textwrap`
    - `replace-tui-keymap-combo-stringification-with-crokey`
    - `rescope-terminal-capability-task-to-current-module`
25. **Phase 25 — Post-review declarative loaders / build / CI.**
    - `deserialize-declarative-command-yaml-with-typed-structs`
    - `move-built-in-slash-commands-to-declarative-yaml`
    - `replace-session-tree-with-arena-crate`
    - `move-cargo-deny-machete-from-workspace-deps-to-ci`
    - `extract-shared-tracing-subscriber-init`
    - `cleanup-verify-tests-and-just-recipes`
    - `remove-redundant-check-field-access-script`
    - `replace-leftover-macros-with-functions`
    - `delete-unused-glyph-constants`

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
- The fifth-pass review (build/CI/test harness, error handling/tracing/telemetry, protocol/IPC leftovers, declarative loaders/DSLs, macros/codegen) is documented in [`2026-06-28-fifth-pass-crate-review.md`](2026-06-28-fifth-pass-crate-review.md).
- A fresh five-round review after implementation found additional gaps; see [`2026-06-28-five-round-review-synthesis.md`](2026-06-28-five-round-review-synthesis.md) for details, peer-codebase learnings, and `ctx7` confirmations.
- A final five-round sweep after the backlog was mostly cleared found additional custom code and prematurely closed tasks; see [`2026-06-28-seventh-pass-final-sweep.md`](2026-06-28-seventh-pass-final-sweep.md).
- If any task proves larger than expected, split it further and update `tasks/index.json` and this roadmap.
