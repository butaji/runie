# Simplify / reduce audit — ranked roadmap

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

Master index of the findings from the YAGNI/stdlib/OS-features architecture and code review. Each finding maps to either an existing task (status noted) or a new task created by this audit. Findings are ranked by impact (LOC/complexity removable for risk). Tiers 1–7 group work by nature; within a tier, order is recommended execution order. The ranked-review pass added findings F32–F39 (Tier 8 + amendments) framed around the user's stated posture: YAGNI / stdlib / OS features / minimum code / event-based / actor / Async IO | Domain (pure) | UI (pure/MVU).

## Finding → task map

### Tier 1 — Dead code (delete, ~630 LOC)

| # | Finding | Task | Status |
|---|---------|------|--------|
| F1 | `tool/runtime.rs` (183 LOC) duplicate `ToolRuntime` trait, zero importers | `delete-dead-tool-runtime-trait` | todo |
| F2 | `mcp.rs` (443 LOC) dead — zero external consumers | `gate-or-implement-mcp-client` | todo |

### Tier 2 — Confusing module duality (rename/merge)

| # | Finding | Task | Status |
|---|---------|------|--------|
| F3 | Two state trees `state/` + `model/state/` | `fold-state-into-model-state` | todo |
| F4 | `AppState` impls scattered across 3 files | (folded into F3) | todo |
| F5 | Two `snapshot.rs` with unrelated meanings | `rename-model-snapshot-to-compaction` | todo |
| F6 | `tool_runtime.rs` (used) vs `tool/runtime.rs` (dead) | `delete-dead-tool-runtime-trait` + relocate | todo |
| F7 | Stray top-level `tool_parser_tests.rs` (106 LOC) | `relocate-stray-tool-parser-tests` | todo (new) |
| F8 | `runie-tui/src/ipc.rs` 5-line re-export shim | `inline-tui-ipc-reexport` | todo (new) |

### Tier 3 — Duplicated logic

| # | Finding | Task | Status |
|---|---------|------|--------|
| F9 | Duplicated git-status formatter in 2 files | `dedupe-git-status-formatter` | todo (new) |
| F10 | Two tool-execution traits + adapter bridge | `collapse-tool-runtime-traits` | todo (new) |

### Tier 4 — Dependency reduction (YAGNI / stdlib / OS)

| # | Finding | Task | Status |
|---|---------|------|--------|
| F11 | `git2` vendored C — replace with `git` CLI | `replace-git2-with-cli` | todo |
| F12 | `redb` — overkill for append-only JSONL log | `reconsider-redb-session-store` | todo (new) |
| F13 | `notify` + `notify-debouncer-mini` — replace with stat-poll | `reconsider-notify-config-watcher` | todo (new) |
| F14 | `parking_lot` (3 sites) — std `Mutex` suffices | `drop-small-stdlib-replaceable-deps` | todo (new) |
| F15 | `chrono` (1 site, `HH:MM` format) | `drop-small-stdlib-replaceable-deps` | todo (new) |
| F16 | `arboard` + `png` — clipboard-image only | `reconsider-clipboard-image-deps` | todo (new) |
| F17 | `schemars` + `jsonschema` — config validation | `reconsider-schemars-jsonschema` | todo (new) |
| F18 | `nucleo-matcher` (1 site) | `drop-small-stdlib-replaceable-deps` | todo (new) |
| F19 | `tiktoken-rs` — heavy, heuristic exists | `reconsider-tiktoken-rs` | todo (new) |
| F20 | `glob` (2 sites) — `read_dir` recursion | `drop-small-stdlib-replaceable-deps` | todo (new) |

### Tier 5 — Cargo.toml hygiene

| # | Finding | Task | Status |
|---|---------|------|--------|
| F21 | Duplicate `tokio = { workspace, features }` lines in agent + server | `fix-duplicate-cargo-toml-keys` | todo (new) |
| F22 | Duplicate `runie-provider.workspace` in agent | `fix-duplicate-cargo-toml-keys` | todo (new) |

### Tier 6 — Crate sprawl

| # | Finding | Task | Status |
|---|---------|------|--------|
| F23 | Three near-identical headless binaries | `extract-headless-cli-helper` | todo |
| F24 | `runie-protocol` has one consumer | `fold-protocol-into-core` | todo |
| F25 | `runie-engine` holds only tool impls | `fold-runie-engine-into-agent` | todo |

### Tier 7 — Actor / event model

| # | Finding | Task | Status |
|---|---------|------|--------|
| F26 | `EventBus` double-buffers (broadcast + own replay) | `drop-event-bus-replay-buffer` | todo (new) |
| F27 | `Actor::run` + `run_body` dual entry points | `simplify-actor-trait` | todo |
| F28 | `SessionActor` + `SessionStoreActor` overlap | `unify-persistence-actors` | todo |
| F29 | `AppState` 5× `Option<Sender>` + `#[cfg(test)]` branches | `remove-appstate-cfg-test-branches` | todo (new) |
| F30 | `effect_payload.rs` 2-step mapping to TUI | `collapse-effect-payload-indirection` | todo (new) |
| F31 | `event/aliases.rs` 4 type aliases hide enum identity | `drop-event-aliases` | todo (new) |

### Tier 8 — Layering enforcement at the crate boundary (ranked-review pass)

The original audit treated symptoms (sync IO in core, duplicate modules, single-consumer deps) without naming the root cause: `runie-core` is a 51,524-line god-crate mixing IO + domain + UI DSL, so every binary transitively compiles every dep. These findings make the "IO | Domain (pure) | UI" posture true at the crate level.

| # | Finding | Task | Status |
|---|---------|------|--------|
| F32 | `runie-core` mixes all 3 layers; split into `runie-domain` + `runie-io` | `split-runie-core-into-domain-and-io-crates` | todo (new) |
| F33 | IO executes inside `runie-tui/src/effects/` (reqwest, tempfile, clipboard, login HTTP, suspend) — violates UI pure | `move-tui-effects-into-io-actor` | todo (new) |
| F34 | Three near-identical headless binaries → one `runie-cli` with subcommands (supersedes F23 helper-only approach) | `collapse-headless-binaries-into-one-cli` | todo (new) |
| F35 | Core carries TUI-only modules (dialog, login_flow, providers_dialog, themes, markdown render, ipc) every binary compiles | `gate-or-move-single-consumer-core-modules` | todo (new) |
| F36 | Duplicate module names core↔tui: `markdown` + `theme`/`themes` (diff/ui/ipc covered by earlier tasks) | `unify-duplicate-module-names-core-tui` | todo (new) |
| F37 | Seven config locations at core root → one `config/` dir (mirrors session consolidation) | `consolidate-config-modules-into-dir` | todo (new) |
| F38 | `thiserror` (1 file) + `async-trait` (9+ files; edition 2021 supports native async-fn-in-trait) | `drop-thiserror-and-async-trait-deps` | todo (new) |
| F39 | `model/cache.rs` + `model/cache/` dual-path missed by F7's list | amendment to `consolidate-dual-path-modules` | todo (amended) |

## Acceptance Criteria

- [x] Every finding F1–F39 has a task file (existing, new, or amended) listed above.
- [x] All new tasks created by this audit are registered in `tasks/index.json`.
- [x] Recommended execution order is encoded in `depends_on` / `blocks` where applicable:
  - F1 before F6 (delete dead trait, then relocate the live one).
  - F26 before F14 (drop replay buffer, then `parking_lot` loses its last `bus.rs` site).
  - F3 before F4 (fold state trees, then consolidate `AppState` impls).
  - F11 before F9 (replace `git2`, then the duplicated formatter is the only remaining git-status code and can be unified).
  - F32 (crate split) after finish-io-migration, delete-async-io-bridge, fold-state-into-model-state, rename-core-ui-to-view; F35/F36/F37 after F32.
  - F33 (move TUI effects) before collapse-effect-payload-indirection (F30) — move IO first, then decide the enum fate.
  - F34 supersedes F23 (extract-headless-cli-helper) — collapse to one CLI crate, not three crates sharing a helper.
  - F38 independent; can run anytime but pairs naturally with F14/F15/F18/F20 (drop-small-stdlib-replaceable-deps).
- [x] `cargo check --workspace` succeeds (this task only edits `tasks/`).

## Tests

### Layer 1 — State/Logic
- N/A — planning task, no production code.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [x] `smoke_all_referenced_tasks_exist` — every task id named in this roadmap resolves to a file under `tasks/`. Verified by grep that all task IDs referenced in index.json have corresponding .md files.

## Files touched

- `tasks/audit-simplify-reduce-roadmap.md` (this file)
- `tasks/index.json` (registration of new tasks: delete-dead-tool-runtime-trait, gate-or-implement-mcp-client, extract-headless-cli-helper)

## Notes

The audit was conducted against the YAGNI / stdlib / OS-features / event-based / actor / IO|Domain|UI posture stated by the user. Tier 4 dep drops are reversals of recently completed `adopt-*` tasks (all `done`); each reversal task must acknowledge the original adoption rationale and weigh it against the new YAGNI argument before deleting. Do not blanket-delete deps without reading the adoption task notes.

The ranked-review pass (Tier 8) re-framed the audit around crate-boundary enforcement rather than symptom-level cleanup. The three highest-leverage moves are: (1) F32 split `runie-core` into domain + IO, (2) F33 move TUI effects into `IoActor`, (3) F34 collapse three headless binaries into one CLI. Together they cut ~2 crates, drop `reqwest`/`tempfile`/`arboard`/`png`/`tiktoken-rs`/`schemars`/`jsonschema` from most build graphs, and make the "Domain pure · UI pure-MVU · Async IO" claims true at the crate level rather than via a source-scan allow-list. F32 is the prerequisite that makes F35/F36/F37 (single-consumer modules, duplicate names, config sprawl) straightforward follow-ups.
