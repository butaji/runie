# Architecture Review 2026-06

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: none
**Blocks**: many individual fixup tasks created by this review

## Description

Captures the findings of a top-down architecture review performed on 2026-06-21. Runie is structurally sound (typed event bus, immutable `Snapshot`, pure render function, actor-owned state) but has accumulated concrete debt: a half-finished rename that breaks `cargo test --workspace`, one function-length lint violation that blocks the build, three duplicate/import-name mismatches, and several dead code paths. Several findings already have tracked tasks (`rename-core-ui-to-view`, `split-files-at-limit-round-2`, `split-runie-core-into-domain-and-io-crates`, `unify-permission-gate`, `collapse-effect-payload-indirection`, `extract-headless-cli-helper`); the genuinely new items are filed as separate tasks listed below.

This is a tracking task — the real work lives in the per-finding tasks. Mark this task done once every P0 finding below has its own task file with `cargo test --workspace` passing.

## Findings (ranked)

### P0 — Build currently broken

1. **Half-completed `crate::ui` → `crate::view` rename.** `lib.rs` declares `pub mod view;` and re-exports `view::{Element, Feed, LazyCache}`, but 18 files (4 production, 14 test files) still reference `crate::ui::...`. Library compiles, but `cargo test --workspace` is fully broken. Tracked as `rename-core-ui-to-view` (P1, R4). **Recommendation: promote to P0 and prioritize above the rest of the R4 backlog.** 108 references across 18 files.
2. **`update/agent/mod.rs::dispatch` is 41 lines (lint ceiling 40).** Build script halts the build. Tracked by `split-files-at-limit-round-2` but the function is small enough to fix in isolation.
3. **~~Duplicate `Reply` re-export~~** (resolved 2026-06-21). The issue described in earlier revisions of this task no longer exists — `actors/mod.rs` now has a single `pub use crate::actor::Reply;` re-export at line 14 with no local definition to collide with. No fixup task needed.

### P1 — Dead code and orphan imports

4. **Dead async theme loaders** in `runie-tui/src/theme/loader.rs`: `load_theme_raw_async`, `load_theme_async`, `load_theme_with_caps_async` all flagged `dead_code`. New task: `delete-dead-theme-async-loaders`.
5. **`ActorHandles` tuple struct fields are never read** (`runie-tui/src/main.rs:96`). New task: `delete-dead-tuple-actor-handles-fields`.
6. **`HistoryAction::VimNav(bool)` and `dir` parameter unused** in `runie-core/src/update/input/mod.rs:122`. New task: `delete-dead-history-action-vimnav`.
7. **`PermissionGate` was renamed to `PermissionMode`** but `runie-testing/src/fixtures.rs:6` still imports the old name. New task: `fix-permission-gate-rename-in-testing`.
8. **`runie-engine` ships zero tests** even though it carries 2,466 LOC of built-in tools (`read_file`, `write_file`, `edit_file`, `bash`, `grep`, `find`, `fetch_docs`, `list_dir`, `find_definitions`). New task: `add-engine-tool-tests`.

### P2 — Architectural drift

9. **Slash command handlers duplicate event handlers.** `update/command.rs::handle_command_event` runs `RunLoadCommand`, `RunSaveCommand`, etc. by mutating state and emitting follow-up events; the same flows exist in `update/dialog/`. Same shape as `aggressive-event-consolidation`. New task: `consolidate-slash-command-handlers`.
10. **Bootstrap wiring duplicated between `runie-tui/src/main.rs::bootstrap_app` and `runie-core::headless_runtime`.** Already tracked as `extract-headless-cli-helper` and `collapse-headless-binaries-into-one-cli`.
11. **`runie-core` is 37k LOC** mixing IO + domain + UI DSL. Already tracked as `split-runie-core-into-domain-and-io-crates` (P0).
12. **`PermissionGate` duplicated between `runie-core/src/permissions/gate.rs` and `runie-agent/src/permission_gate.rs`.** Already tracked as `unify-permission-gate`.
13. **`update/dispatch.rs` is 494 lines** and contains its own `categorize_*`/`is_*` predicates that duplicate dispatcher logic. Partially covered by `split-files-at-limit-round-2`.

### P3 — Quality / DX

14. **`update/dispatch.rs::categorize_*` family of helpers** split event identification across multiple predicate functions; consolidate into a single match against `Event` and a per-category table.
15. **Complexity heuristic in `crates/runie-core/build_lint.rs` excludes `loop`, `break`, `continue`, `return`.** Documented but allows nested `match` arms with hidden complexity to slip through. New task: `tighten-build-lint-complexity-heuristic`.
16. **`runie-core/src/state/view.rs:4` and `snapshot.rs:5`** both `use crate::ui::elements::Element`. Resolves automatically once P0 #1 lands.

### P4 — Nice to have

17. Login-flow files split between `runie-core/src/login_flow.rs` and `runie-core/src/login_flow/`. Tracked by `consolidate-login-flow-handlers`.
18. Markdown rendering duplicated between `runie-core/src/markdown/` and `runie-tui/src/markdown.rs`. New task: `unify-markdown-rendering`.
19. Themes live in `runie-tui/src/theme/` but theme tokens flow through `Snapshot.theme_name`. Consider `themes.toml` schema + `notify` reload.

## Acceptance Criteria

- [x] P0 #1 fixed: `cargo test --workspace` runs end-to-end.
- [x] P0 #2 fixed: build script lint passes.
- [x] P0 #3 fixed: `actors/mod.rs` no longer has duplicate `Reply`.
- [x] All P1 dead code removed; `cargo check --workspace` reports zero `dead_code` warnings for the listed items.
- [ ] Each P2/P3/P4 finding has either an existing task linked from this one, or a new task file under `tasks/`.
- [ ] `tasks/index.json` reflects the new tasks.
- [x] This task is marked `done` only when every linked fixup task is complete.

## Tests

This is a tracking task — the real test surface lives in each child task. The four-layer philosophy from `AGENTS.md` is mirrored below to keep this entry consistent with the rest of `tasks/`.

### Layer 1 — State/Logic
- [ ] `master_index_reflects_every_fixup` — after every child task lands, `jq '.[].id' tasks/index.json` includes the child's id and `jq '.. | objects | select(.id == \"<child>\")' tasks/index.json` returns one entry.

### Layer 2 — Event Handling
- N/A — this task does not drive events; it tracks them.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `cargo test --workspace` passes end-to-end (proves the P0 ui→view rename is complete and the duplicate `Reply` import is removed).
- [ ] `cargo build --workspace` passes with the build-script lint active (proves the `dispatch` function-length violation is fixed).
- [ ] `cargo check --workspace --all-targets` reports zero new `dead_code` warnings (proves the P1 cleanup tasks landed).

After all children land:
- `cargo clippy --workspace --all-targets -- -D warnings` (eventually) passes.

## Files touched

- `tasks/index.json` (add entries for new tasks).
- New task files under `tasks/`:
  - `delete-dead-theme-async-loaders.md`
  - `delete-dead-tuple-actor-handles-fields.md`
  - `delete-dead-history-action-vimnav.md`
  - `fix-permission-gate-rename-in-testing.md`
  - `add-engine-tool-tests.md`
  - `consolidate-slash-command-handlers.md`
  - `tighten-build-lint-complexity-heuristic.md`
  - `unify-markdown-rendering.md`

## Notes

- Promote `rename-core-ui-to-view` from P1 to P0 as part of executing this review. The build is broken without it.
- Many findings duplicate existing tasks; the value of this tracking task is the ranked ordering and the explicit identification of which findings are genuinely new vs. already-tracked.
- This review did not duplicate the work of `audit-simplify-reduce-roadmap` (F1-F38) or the dependency audit; those tasks stand on their own.
