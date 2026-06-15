# Runie Implementation Roadmap

This roadmap sequences the active R3/R4 work in dependency order. It is a living document; task statuses are authoritative in `tasks/index.json`.

## Phase 0 — Build Blocker & Quick Wins

Goal: get `cargo test --workspace` green and land low-risk crate adoptions.

- [ ] `tasks/archive/unblock-workspace-build.md` — split/simplify files violating the 500/40/10 lint.
- [ ] `tasks/adopt-notify-config-watcher.md` — replace config polling with `notify`.
- [ ] `tasks/adopt-arboard-clipboard.md` — replace clipboard subprocesses with `arboard`.
- [ ] `tasks/adopt-patch-diff-parser.md` — replace hand-rolled diff parser with `patch`.
- [ ] `tasks/adopt-serde-yaml-skills.md` — parse skill frontmatter with `serde_yml`.
- [ ] `tasks/adopt-palette-theme-colors.md` — replace theme color helpers with `palette`.
- [ ] `tasks/fix-session-store-atomic-writes.md` — atomic `SessionStore::append`.

## Phase 1 — Actor/EventBus & Core Abstractions

Goal: complete the actor/runtime foundation and simplify the core event/state model.

- [ ] `tasks/adopt-or-remove-actor-framework.md` (done) — EventBus wired in `runie-term`.
- [ ] `tasks/event-bus-jsonl-persistence.md` (done) — JSONL session persistence.
- [ ] `tasks/tool-registry-trait.md` (done) — unified `Tool` trait in agent turn.
- [ ] `tasks/harness-skill-framework.md` — skill lifecycle hooks on the event bus.
- [ ] `tasks/complete-appstate-refactor.md` — remove duplicated state fields.
- [ ] `tasks/flatten-event-system.md` — flatten `Event` sub-enums where possible.
- [ ] `tasks/fff-indexer-actor.md` — long-lived `FffIndexerActor`.

## Phase 2 — Tools, Search & Skills

Goal: land FFF-powered search and the harness skill suite.

- [ ] `tasks/fff-unified-search-tool.md` — replace `grep`/`find`/`list_dir`.
- [ ] `tasks/fff-tui-file-picker.md` — FFF-backed `@` picker.
- [ ] `tasks/fff-find-definitions-tool.md` — definition classifier tool.
- [ ] `tasks/fff-query-syntax-and-examples.md` — agent query syntax.
- [ ] `tasks/fff-frecency-and-git-status.md` — frecency + git filtering.
- [ ] `tasks/fff-glob-tool.md` — fast glob tool.
- [ ] `tasks/fff-location-parser.md` — `file:line:col` parsing.
- [ ] `tasks/harness-skill-hashline-edit.md` — hashline edit tool.
- [ ] `tasks/harness-skill-verification-loop.md` — post-completion verification.
- [ ] `tasks/harness-skill-loop-detector.md` — loop detection/recovery.
- [ ] `tasks/harness-skill-startup-context.md` — startup context injection.
- [ ] `tasks/harness-skill-tool-schema-enricher.md` — schema examples.
- [ ] `tasks/permission-rulesets.md` — tool permission rules.
- [ ] `tasks/mcp-client-integration.md` — MCP client for external tools.

## Phase 3 — TUI & Rendering Polish

Goal: consolidate the TUI layer and adopt remaining rendering crates.

- [ ] `tasks/merge-runie-term-into-tui.md` — move terminal setup into `runie-tui`.
- [ ] `tasks/unify-rendering-pipeline.md` — single rendering path.
- [ ] `tasks/adopt-nucleo-non-file-fuzzy.md` — fuzzy matching for panels.
- [ ] `tasks/adopt-textwrap-word-wrap.md` — Unicode word wrapping.
- [ ] `tasks/grok-*` — UI/UX polish tasks.
- [ ] `tasks/unify-markdown-pipeline.md` — consolidate markdown rendering.

## Phase 4 — R4 Team / Orchestrator

Goal: add solo/team execution modes and the orchestrator actor.

- [ ] `tasks/r4-orchestrator-domain-types.md`
- [ ] `tasks/r4-model-trait-resolution.md`
- [ ] `tasks/r4-ask-user-tool.md`
- [ ] `tasks/r4-one-shot-orchestrator-llm.md`
- [ ] `tasks/r4-orchestrator-actor.md`
- [ ] `tasks/r4-subagent-isolation.md`
- [ ] `tasks/r4-subagent-sidebar.md`
- [ ] `tasks/r4-sidebar-task-list.md`
- [ ] `tasks/r4-solo-team-mode-toggle.md`
- [ ] `tasks/r4-team-mode-integration.md`

## Phase 5 — R3 Simplification Cleanup

Goal: finish the R3 simplification pass.

- [ ] `tasks/coalesce-update-modules.md` — reduce 33 update modules to ≤8.
- [ ] `tasks/unify-command-dsl.md`
- [ ] `tasks/unify-diff-model.md`
- [ ] `tasks/cleanup-state-helpers.md`

## Phase 6 — Persistence & Advanced Features

Goal: upgrade session persistence and enable long-running features.

- [ ] `tasks/adopt-redb-session-store.md` — migrate sessions to `redb`.
- [ ] `tasks/remove-test-sleeps.md` — deterministic tests.
- [ ] `tasks/fix-harness-crate-or-archive.md` — resolve `harness/` crate.

## Decision Records

- `docs/adr/0017-actor-runtime-and-event-bus.md`
- `docs/adr/0022-harness-middleware-plugins.md`
- `docs/adr/0023-fff-search-integration.md`
- `docs/CRATE_DECISIONS.md`
