# Runie Implementation Roadmap

This roadmap sequences the active R3/R4 work in dependency order. It is a living document; task statuses are authoritative in `tasks/index.json`.

## Phase 0 — Build Blocker & Quick Wins

Goal: get `cargo test --workspace` green and land low-risk crate adoptions.

- [x] ~~`tasks/archive/unblock-workspace-build.md`~~ — done
- [x] ~~`tasks/adopt-notify-config-watcher.md`~~ — done
- [x] ~~`tasks/adopt-arboard-clipboard.md`~~ — done
- [x] ~~`tasks/adopt-patch-diff-parser.md`~~ — done
- [x] ~~`tasks/adopt-serde-yaml-skills.md`~~ — done (serde_yaml 0.9 for skill frontmatter)
- [ ] `tasks/adopt-palette-theme-colors.md` — replace theme color helpers with `palette` crate
- [x] ~~`tasks/fix-session-store-atomic-writes.md`~~ — done

## Phase 1 — Actor/EventBus & Core Abstractions

Goal: complete the actor/runtime foundation and simplify the core event/state model.

- [x] ~~`tasks/adopt-or-remove-actor-framework.md`~~ — done
- [x] ~~`tasks/event-bus-jsonl-persistence.md`~~ — done
- [x] ~~`tasks/tool-registry-trait.md`~~ — done
- [x] ~~`tasks/harness-skill-framework.md`~~ — done
- [x] ~~`tasks/complete-appstate-refactor.md`~~ — done
- [x] ~~`tasks/flatten-event-system.md`~~ — done
- [ ] `tasks/fff-indexer-actor.md` — long-lived FFFIndexerActor (P0, blocks all FFF tools)

## Phase 2 — Tools, Search & Skills

Goal: land FFF-powered search and the harness skill suite.

### FFF Tools (blocked on fff-indexer-actor)
- [ ] `tasks/fff-unified-search-tool.md` — replace grep/find/list_dir (P0)
- [ ] `tasks/fff-tui-file-picker.md` — FFF-backed @ picker (P0)
- [ ] `tasks/fff-find-definitions-tool.md` — definition classifier tool (P1)
- [ ] `tasks/fff-query-syntax-and-examples.md` — agent query syntax (P1)
- [ ] `tasks/fff-frecency-and-git-status.md` — frecency + git filtering (P1)
- [ ] `tasks/fff-glob-tool.md` — fast glob tool (P2)
- [ ] `tasks/fff-location-parser.md` — file:line:col parsing (P2)

### Harness Skills (independent of FFF)
- [ ] `tasks/harness-skill-verification-loop.md` — post-completion verification (P0)
- [ ] `tasks/harness-skill-hashline-edit.md` — hashline edit tool (P0)

## Phase 3 — TUI & Rendering Polish

Goal: consolidate the TUI layer and adopt remaining rendering crates.

- [ ] `tasks/merge-runie-term-into-tui.md` — move terminal setup into runie-tui (P1)
- [x] ~~`tasks/unify-rendering-pipeline.md`~~ — deleted core format_test; TUI sole renderer (done)
- [x] ~~`tasks/adopt-nucleo-non-file-fuzzy.md`~~ — done
- [x] ~~`tasks/adopt-textwrap-word-wrap.md`~~ — done
- [x] ~~`tasks/adopt-syntect.md`~~ — done
- [x] ~~`tasks/adopt-pulldown-cmark.md`~~ — done
- [x] ~~`tasks/adopt-tiktoken-rs.md`~~ — done
- [ ] `tasks/unify-markdown-pipeline.md` — consolidate core + TUI markdown parsing (P2, depends on unify-rendering-pipeline ✅)
- [ ] `tasks/adopt-palette-theme-colors.md` — palette crate for theme colors (P2)

## Phase 4 — R4 Team / Orchestrator

Goal: add solo/team execution modes and the orchestrator actor.

- [ ] `tasks/r4-orchestrator-domain-types.md` (P0, blocks r4-*)
- [ ] `tasks/r4-model-trait-resolution.md` (P0)
- [ ] `tasks/r4-ask-user-tool.md` (P0)
- [ ] `tasks/r4-one-shot-orchestrator-llm.md` (P0)
- [ ] `tasks/r4-orchestrator-actor.md` (P0)
- [ ] `tasks/r4-subagent-isolation.md` (P0)
- [ ] `tasks/r4-subagent-sidebar.md` (P0)
- [ ] `tasks/r4-sidebar-task-list.md` (P1)
- [ ] `tasks/r4-solo-team-mode-toggle.md` (P0)
- [ ] `tasks/r4-team-mode-integration.md` (P0)

## Phase 5 — R3 Simplification Cleanup

Goal: finish the R3 simplification pass.

- [x] ~~`tasks/coalesce-update-modules.md`~~ — reduced update/ to 8 flat + 2 subdirs (done)
- [x] ~~`tasks/unify-command-dsl.md`~~ — done
- [x] ~~`tasks/unify-diff-model.md`~~ — done
- [x] ~~`tasks/cleanup-state-helpers.md`~~ — SpeedWindow VecDeque, stubs removed (done)
- [ ] `tasks/session-list-summaries.md` — core done; UI wiring and LLM summaries (P2)

## Phase 6 — Persistence & Advanced Features

Goal: upgrade session persistence and enable long-running features.

- [ ] `tasks/adopt-redb-session-store.md` — migrate sessions to redb (P2, depends on event-bus-jsonl-persistence ✅)
- [x] ~~`tasks/remove-test-sleeps.md`~~ — done
- [x] ~~`tasks/fix-harness-crate-or-archive.md`~~ — done

## Decision Records

- `docs/adr/0017-actor-runtime-and-event-bus.md`
- `docs/adr/0022-harness-middleware-plugins.md`
- `docs/adr/0023-fff-search-integration.md`
- `docs/CRATE_DECISIONS.md`
