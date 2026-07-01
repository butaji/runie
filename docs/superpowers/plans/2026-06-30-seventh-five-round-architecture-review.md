# Seventh Five-Round Architecture & Code Review

## Status

Synthesis complete. Findings ranked and tracked as new `tasks/` files; stale/duplicate task consolidated.

## Goal

Pareto (80/20) simplification: less code, fewer custom implementations, fewer duplicate dependencies, clearer boundaries, and a coherent task backlog. If a standard crate, OS feature, or build-tool feature can replace custom code, plan it that way.

## Methodology

- **Five read-only subagents** reviewed disjoint slices (no code executed):
  1. Backlog audit — duplicates, outdated statuses, merge opportunities, prerequisite chains.
  2. Post-refactor consistency — leftover types, stale docs, dead code, missing tests after recent large merges.
  3. Remaining crate replacements — custom code that still has a crate alternative.
  4. Async/event/runtime simplification — `tokio` misuse, fire-and-forget tasks, race conditions, blocking threads.
  5. Grok Build comparison unblockers — minimal prerequisites to un-block the 5 `compare-*` tasks.
- **~/Code/agents/* survey** focused on: diff/edit fixtures, VT100 deterministic TUI fixtures, headless JSONL, scenario manifests, session replay checkpoints, subagent/MCP specs, message origins.
- **ctx7** was consulted for: `tokio`, `thiserror`, `strum`, `parking_lot`, `winnow`, `vt100`, `similar`, `landlock`, `command-group`.

## Executive Summary

This seventh pass found **backlog-management drift** and a large number of concrete code-level leftovers introduced by recent refactorings:

1. **Backlog hygiene** — `index.json` drifted out of sync with task files; `deduplicate-turn-state-between-turnactor-and-appstate.md` duplicates `merge-agentstate-into-turnstate-projection.md`.
2. **Post-refactor leftovers** — `AgentState` still mirrors authoritative `TurnState` fields; `ModelClient`/`TurnSession` introduced but unused; stale `DynProvider` references; dead re-exports; `std::sync` locks remain despite a done normalization task; blanket Clippy allow; old markdown AST still imported; `derive_builder` incomplete.
3. **Async/runtime risks** — detached agent-turn `JoinHandle`, fire-and-forget provider network calls, late bus subscription race in queue drain, blocking TUI render thread, dropped headless/leader join handles, custom `block_on` in CLI, `Arc<Mutex<dyn FnMut>>` `EmitFn`, unnecessary mutex inside actor state.
4. **Crate replacements** — custom `secrecy` serde wrappers, leftover manual enum string mappings, manual `Display`/`Error`, legacy diff fallback parser, custom file-ref parser, manual timestamp formatting, duplicated `unix_now` snippets.
5. **Grok harness** — headless CLI and TUI lack a `ProviderFactory` injection seam; no deterministic TUI replay harness; VT100 parser could replace tmux scripts.
6. **Agent-pattern imports** — message `origin` metadata from kimi-code would fix follow-up continuity and multi-turn comparisons.

## Backlog Management Findings

| # | Severity | Finding | Action |
|---|----------|---------|--------|
| 1 | P0 | Several task files are `done` in their files but still listed as `todo` in `index.json` | Regenerate `tasks/index.json` |
| 2 | P0 | `deduplicate-turn-state-between-turnactor-and-appstate.md` duplicates `merge-agentstate-into-turnstate-projection.md` | Archive `deduplicate-...`; fold notes into `merge-...` |
| 3 | P1 | Config/credential tasks will conflict and should be sequenced | Keep them but execute in order (see Suggested Execution Order) |

## P0 / P1 Code Findings

| # | Area | File(s) | Issue | Plan | Task |
|---|------|---------|-------|------|------|
| 4 | State | `model/state/agent.rs`, `turn_projections.rs`, `update/system.rs`, `app_state.rs` | `AgentState` still holds authoritative fields and is mutated directly | Fold into `merge-agentstate-into-turnstate-projection` (updated) | (existing task updated) |
| 5 | Async | `runie-agent/src/actor.rs:137-164` | `run_turn` spawns a detached task; no `JoinHandle` tracking | Store handle; reject/queue overlapping runs | `track-and-cancel-inflight-agent-turn-joinhandle` |
| 6 | Docs | `actors/provider/factory.rs:45`, `docs/Architecture.md:97` | References to deleted `DynProvider` remain | Update doc/comments | `remove-stale-dynprovider-references` |
| 7 | Provider | `runie-provider/src/model_client.rs`, `openai/mod.rs`, `lib.rs` | `ModelClient`/`TurnSession` split landed but unused | Wire into factory or delete | `wire-modelclient-turnsession-into-provider-factory-or-delete` |
| 8 | Provider | `runie-provider/src/retry.rs:13-15` | Dead re-export `classify_reqwest_error` | Delete | `delete-dead-classify-reqwest-error-reexport` |
| 9 | Concurrency | `actors/provider/factory.rs`, `session/tree.rs`, `auth/store_trait.rs`, `harness_skills/startup_context.rs`, `harness_skills/loop_detector.rs` | `std::sync::Mutex`/`RwLock` still used in production | `parking_lot` equivalents | `replace-remaining-std-sync-locks-with-parking_lot` |
| 10 | Async | `actors/provider/ractor_provider.rs:256-303` | Network calls spawned with `tokio::spawn` and never joined | Await directly in ractor handler | `await-provider-network-calls-in-ractor-handler` |
| 11 | Async | `runie-tui/src/ui_actor.rs:633-678` | Late bus subscription races with `DeliverQueued` events | Make queue delivery atomic or request/response | `fix-deliverqueued-race-in-uiactor` |
| 12 | Async/TUI | `runie-tui/src/main.rs:257-307` | Render loop is a blocking `std::thread` polling `recv_timeout(16ms)` | Async loop on `tokio::sync::watch` | `make-tui-render-loop-async-with-watch-channel` |
| 13 | Async | `runie-core/src/headless_runtime.rs:41-44` | Headless runtime drops actor join handles | Store and await them in `shutdown()` | `store-and-await-headlessruntime-actor-handles` |
| 14 | Async | `actors/leader/actor.rs`, `handle.rs` | Leader coordinator/TCP handles not stored; shutdown kills actors | Store handles; `JoinSet` + timeout | `graceful-leader-shutdown-with-joinset-timeout` |
| 15 | CLI | `runie-cli/src/main.rs:101-119` | Custom `block_on` creates a new `current_thread` runtime per subcommand | `#[tokio::main]` | `use-tokio-main-in-cli-instead-of-custom-block-on` |
| 16 | Async | `runie-agent/src/stream_response.rs` | `EmitFn` is `Arc<Mutex<dyn FnMut(Event)>>` | `tokio::sync::mpsc` sender | `replace-emitfn-mutex-with-async-channel` |
| 17 | Grok | `runie_agent::headless_helper`, `HeadlessRuntime::spawn` | Headless CLI hard-codes `BuiltProviderFactory`; no seam for replay | Accept `Arc<dyn ProviderFactory>` | `inject-provider-factory-into-headless-cli-and-runtime` |
| 18 | Grok/TUI | `runie-tui/src/main.rs` | TUI cannot be bootstrapped with a custom provider + keystroke DSL | Extract testable `run_with_backend_and_provider` | `add-testable-tui-bootstrap-with-providerfactory-and-keystrokes` |
| 19 | State | `model/state/types.rs`, TUI queue handling | No message origin metadata; follow-ups stuck behind active turn | Add `origin` to `ChatMessage`/`AgentEvent` flow | `add-message-origin-metadata-to-fix-follow-up-continuity` |
| 20 | Auth | `runie-core/src/auth/storage.rs:10-24` | Custom `serialize_secret`/`deserialize_secret` wrappers around `secrecy::SecretString` | Use `secrecy` serde feature | `remove-authtoken-custom-serde-wrappers` |
| 21 | Enums | `commands/dsl/category.rs`, `proto/message/mod.rs`, `provider_event.rs`, `model/state/types.rs`, `agent_phase.rs`, `tool/search/types.rs`, `permissions/mod.rs` | Manual `FromStr`/`Display`/`as_str` mappings still exist | `strum::EnumString`, `Display`, `IntoStaticStr` | `finish-strum-migration-for-remaining-enums` |
| 22 | Errors | `runie-core/src/proto/error.rs:48-54` | Hand-written `Display`/`Error` impls | `#[derive(Debug, Error)]` via `thiserror` | `replace-proto-error-manual-display-with-thiserror` |
| 23 | Diff | `runie-core/src/diff/mod.rs:213-269` | Legacy `fallback_parse_diff` remains | Delete or switch to `similar` | `remove-legacy-diff-fallback-parser` |

## P2 Code Findings

| # | Area | File(s) | Issue | Plan | Task |
|---|------|---------|-------|------|------|
| 24 | Lint | `runie-core/src/model/state/turn_projections.rs:1` | `#![allow(clippy::all)]` hides real issues | Remove blanket allow | `remove-blanket-clippy-allow-in-turn-projections` |
| 25 | Tests | `runie-core/src/tests/appstate_structural.rs` | Tests directly mutate `AgentState` | Rewrite to assert projection from `TurnState` | `rewrite-appstate-structural-tests-to-not-mutate-agentstate` |
| 26 | Builders | `runie-core/src/model_catalog/mod.rs:10-19` | `derive_builder` only applied to `ModelCapabilities` | Finish migration for `ModelInfo`/provider configs | `finish-derive-builder-for-modelinfo-and-provider-configs` |
| 27 | Parsing | `runie-core/src/file_refs.rs:24-90` | Custom `@path:start-end` parser | `winnow`/`nom`/`regex` grammar | `replace-file-ref-parser-with-winnow-or-regex` |
| 28 | Time | `auth/storage.rs`, `proto/message/mod.rs`, `actors/fff_indexer/mod.rs` | Duplicated `SystemTime::now().duration_since(UNIX_EPOCH)` snippets | Centralize `unix_now()` helper | `centralize-unix-now-helper` |
| 29 | Grok | n/a | tmux scripts are the only deterministic TUI comparison path | Adopt `vt100::Parser` backend | Mention in harness design |
| 30 | Tests | `runie-testing/src/`, provider/agent tests | `futures::executor::block_on` inside `#[tokio::test]`; mock sleeps; 5-second timeout test | Async helpers; `tokio::time::pause` | Mention in test-cleanup guidance |

## Agents Survey Takeaways Mapped to Runie

| # | Source | Pattern | Runie application |
|---|--------|---------|-------------------|
| 1 | gptme `patch`/`patch_many` | Conflict-marker hunks + atomic multi-file rollback | Reference diff/edit fixture format for `compare-diff-edit-output-and-fix-gaps` |
| 2 | codex `diff_render.rs` | `diffy` + line numbers + syntax highlighting | Validate Grok parity with line-numbered output |
| 3 | codex `test_backend.rs` | VT100 in-memory terminal backend | `adopt-vt100-parser-for-deterministic-tui-fixtures` |
| 4 | codex `WelcomeWidget` | Breakpoint fallback + buffer assertions | Layer-3 pattern for welcome/hint rendering comparisons |
| 5 | gptme context | `@` mention expansion with `git ls-files` fallback | Inform `compare-file-context-picker-and-fix-gaps` |
| 6 | goose/gptme/codex | Headless `run`/`Exec` with JSONL stdout | Use JSONL as Grok fixture format |
| 7 | aider benchmark | Temp-copy isolation + `.results.json` | Scenario manifest + temp isolation |
| 8 | codex/kimi-code | JSONL session replay with checkpoints | Inform session persistence and multi-turn fixtures |
| 9 | gptme subagent/MCP | Tool specs with planner/executor modes + MCP management tool | Inform MCP annotations task |
| 10 | kimi-code agent-core | `origin` metadata per message | `add-message-origin-metadata-to-fix-follow-up-continuity` |

## Suggested Execution Order

1. Backlog sync — regenerate `tasks/index.json`; archive duplicate.
2. Dangerous async bugs — `track-and-cancel-inflight-agent-turn-joinhandle`, `fix-deliverqueued-race-in-uiactor`, `await-provider-network-calls-in-ractor-handler`.
3. Shutdown/resource leaks — `store-and-await-headlessruntime-actor-handles`, `graceful-leader-shutdown-with-joinset-timeout`, `make-tui-render-loop-async-with-watch-channel`.
4. State SSOT — `merge-agentstate-into-turnstate-projection` (updated).
5. Cleanup leftovers — stale `DynProvider`, dead re-export, std locks, Clippy allow, appstate tests, derive_builder.
6. Crate finish-line — `remove-authtoken-custom-serde-wrappers`, `finish-strum-migration-for-remaining-enums`, `replace-proto-error-manual-display-with-thiserror`, `remove-legacy-diff-fallback-parser`, `replace-file-ref-parser-with-winnow-or-regex`.
7. CLI/runtime — `use-tokio-main-in-cli-instead-of-custom-block-on`, `replace-emitfn-mutex-with-async-channel`.
8. Grok harness — `inject-provider-factory-into-headless-cli-and-runtime`, `add-testable-tui-bootstrap-with-providerfactory-and-keystrokes`.
9. Follow-up continuity — `add-message-origin-metadata-to-fix-follow-up-continuity`.

## Risks & Preconditions

- `AgentState`/`TurnState` collapse is the largest keystone; it blocks TUI state cleanups and multi-turn comparisons.
- Async shutdown changes can introduce hangs if not tested with real actor startup/shutdown.
- Grok Build binary remains quarantined/auth-blocked; keep recorder scripts separate from CI.

## New Tasks Created

See `tasks/` for 24 new task files with unit/e2e/live-tmux acceptance criteria. `merge-agentstate-into-turnstate-projection.md` was updated with concrete file references; `deduplicate-turn-state-between-turnactor-and-appstate.md` was archived.
