# Five-Round Review Synthesis — Post-Implementation Gaps

This document synthesizes a fresh five-round architecture and code review performed after many of the earlier cleanup tasks had already landed. The review focused on what is *still* custom, duplicated, or broken in the current codebase, and it incorporates lessons from peer codebases and `ctx7` lookups.

## Review scope

| Round | Focus |
|-------|-------|
| 1 | Actor runtime, bootstrap, lifecycle, handles, dispatch |
| 2 | Provider, config, auth, model catalog, retry, streaming |
| 3 | Session persistence, replay index, declarative loaders, DSL, subagents |
| 4 | TUI, rendering, input, markdown, dialog, terminal capabilities |
| 5 | Build/CI, error handling/tracing/telemetry, protocol/IPC, macros, dependencies, testing |

## Peer learnings

A survey of `~/Code/agents/{goose,jcode,openfang,thClaws}` reinforced several choices:

- **Keep `ractor`.** All four peers avoid full actor frameworks and pay the price in ad-hoc `tokio::spawn` + `Arc<Mutex>` boilerplate. Runie's typed actor handles are a differentiator.
- **Keep `tracing` + `tracing-subscriber`.** Goose and OpenFang use it; jcode reinvented a logging crate.
- **Session persistence:** peers use SQLite (`goose`, `openfang`), JSONL+`fs2` (`thClaws`), or JSON snapshot+journal (`jcode`). Runie's current direction is JSONL+`fs2` (SQLite deferred per user direction).
- **Provider layer:** all four hand-roll HTTP/SSE behind a trait. Runie's `Provider` trait shape is correct; the main win is replacing the raw OpenAI implementation only if the team wants to offload it.
- **Secrets:** Goose and thClaws use `keyring` with env/file fallback. Runie should move provider API keys out of `config.toml`.
- **TUI:** stay terminal-first with `ratatui`; avoid Electron/Tauri bloat.
- **CI:** adopt `cargo-deny`, `cargo-machete`, `clippy.toml`, and `RUSTFLAGS="-D warnings"` like Goose/OpenFang.

## `ctx7` confirmations

- **`ractor`**: idiomatic usage is `type State = ...` and `handle(..., state: &mut State)`. Holding mutable state behind `Mutex` inside `type State = ()` defeats the framework.
- **`backon`**: retry is expressed as `fetch.retry(ExponentialBuilder::default()).when(...).await`.

## Highest-priority new findings

### P0 — broken actor plumbing

1. **`RactorPermissionActor` auto-denies every request.** It discards the `ApprovalRegistry` receiver and immediately replies `Deny`.
2. **TUI `AgentActor` channel is disconnected.** `main.rs` drops the receiver and discards the real `RactorAgentHandle`.
3. **ACP stdio plumbing is broken.** `AcpRuntime.event_tx` sends into a dropped receiver, and there are duplicate stdout forwarders.

### P1 — actor/runtime simplification

4. `InputActor` is spawned but never used.
5. Dead actor handle wrappers (`SessionActorHandle`, `ConfigActorHandle`, etc.) still compile.
6. Actors use `type State = ()` + interior `Mutex` instead of `ractor` `State`.
7. `ActorHandles` is still a 300-line façade rather than a typed `ActorRef` map.
8. Sync `TurnQueue` fallback in `AppState` duplicates the async `TurnQueue`.
9. `std::sync::Mutex`/`RwLock` remain in permissions, fff_indexer, and runie-agent.
10. Provider API keys are plaintext in `config.toml`.
11. TUI message wrapping is duplicated and ignores display-cell width.
12. TUI keymap combo strings are hand-rolled.
13. Markdown block parsing re-injects inline markers instead of using `pulldown-cmark` events.

### P2 — provider/config/CI/TUI

14. `runie-provider/src/retry.rs` still hand-rolls backoff despite a "done" task.
15. Hand-written config validator duplicates `config.schema.json`.
16. `cargo-deny`/`cargo-machete` are in workspace deps instead of CI.
17. `just lint-fix` allows all lints.
18. `scripts/verify-tests.sh` hardcodes an exact test count.
19. Custom session tree maintains a manual index/cache.
20. Built-in slash commands are static Rust tables.
21. TUI markdown block layout is hand-rolled while `tui-markdown` is underused.
22. Terminal capability task references deleted files.

## New task list

The following tasks were added to `tasks/index.json`:

| Task | Priority | Area |
|------|----------|------|
| `fix-ractor-permission-actor-reply-lifecycle` | P0 | Actors |
| `reconnect-tui-agent-actor-channel` | P0 | Actors |
| `wire-or-delete-input-actor` | P1 | Actors |
| `delete-dead-actor-handle-wrappers` | P1 | Actors |
| `use-ractor-state-for-actor-mutable-state` | P1 | Actors |
| `actually-collapse-actor-handles-to-typed-map` | P1 | Actors |
| `remove-sync-turn-queue-fallback-from-app-state` | P1 | Actors |
| `normalize-remaining-std-mutex-to-parking_lot` | P1 | Reliability |
| `delete-eventbusbridge-wrapper` | P2 | Actors |
| `actually-replace-runie-provider-backoff-with-backon` | P2 | Provider |
| `store-provider-api-keys-in-keyring-not-config` | P1 | Security |
| `add-jsonschema-validation-to-configactor-load` | P2 | Config |
| `drop-unused-rand-from-runie-provider` | P2 | Dependencies |
| `replace-session-tree-with-arena-crate` | P2 | Sessions |
| `deserialize-declarative-command-yaml-with-typed-structs` | P2 | DSL |
| `move-built-in-slash-commands-to-declarative-yaml` | P2 | Commands |
| `unify-tui-message-wrapping-with-textwrap` | P1 | TUI |
| `replace-tui-keymap-combo-stringification-with-crokey` | P1 | TUI |
| `replace-tui-markdown-block-layout-with-tui-markdown` | P0 | TUI |
| `unify-markdown-block-parsing-on-pulldown-cmark-events` | P1 | Core |
| `rescope-terminal-capability-task-to-current-module` | P2 | TUI |
| `delete-unused-glyph-constants` | P3 | TUI |
| `move-cargo-deny-machete-from-workspace-deps-to-ci` | P2 | Build/CI |
| `extract-shared-tracing-subscriber-init` | P2 | Observability |
| `delete-leftover-telemetry-stub` | P3 | Observability |
| `replace-custom-nanoid-in-test-runner` | P3 | Testing |
| `cleanup-verify-tests-and-just-recipes` | P3 | Build/CI |
| `remove-redundant-check-field-access-script` | P3 | Build/CI |
| `replace-leftover-macros-with-functions` | P3 | Core |

## Existing tasks updated

Several earlier tasks were found to be incomplete or stale and were updated with notes:

- `replace-custom-retry-with-backon` — code still hand-rolls backoff; follow-up task added.
- `initialize-tracing-subscriber-in-binaries` — init duplicated in both binaries; follow-up task added.
- `replace-custom-telemetry-with-tracing-layer` — `telemetry.rs` stub remains; follow-up task added.
- `harden-actors-against-mutex-poisoning` — std locks remain in permissions/fff_indexer/runie-agent; follow-up task added.
- `eliminate-production-unwrap-expect` — status reset to `todo`; ACs cleaned up.
- `collapse-actor-handles-to-typed-map` — current `ActorHandles` is still a façade; follow-up task added.
- `dedupe-turn-queue-delivery-logic` — sync fallback remains; follow-up task added.
- `unify-cli-json-rpc-transport-and-remove-dead-acp` — leader TCP protocol also needs unification.
- `unify-core-and-tui-line-count-computation` — file paths updated.
- `simplify-terminal-capability-detection` — file paths updated.
- `replace-remaining-custom-parsers-and-macros-with-strum` — expanded to remaining enums.
- `implement-or-remove-mcp-runtime-scaffolding` — dead argv parsers noted.
- `type-and-unify-provider-model-layer` — stale file paths fixed; keyring integration noted.

## Execution notes

- Land the three P0 actor fixes before anything else; they block production functionality.
- `use-ractor-state-for-actor-mutable-state` should land before the permission-actor reply fix because the fix becomes trivial with state as a `HashMap`.
- `store-provider-api-keys-in-keyring-not-config` should land near `type-and-unify-provider-model-layer` so the key type can become `SecretString`.
- `unify-markdown-block-parsing-on-pulldown-cmark-events` should land before `replace-tui-markdown-block-layout-with-tui-markdown`.
- Every task must include the 4-layer test coverage required by `AGENTS.md`.
