# Seventh-Pass Final Sweep — Remaining Custom Code & Prematurely Closed Tasks

This document captures the findings from a fresh five-round review performed after the task backlog had been mostly cleared. The sweep focused on identifying:

1. Custom code that still mirrors crates/OS features.
2. Tasks marked `done` whose acceptance criteria are not actually satisfied.
3. New simplification/unification opportunities not yet tracked.

## Baseline

- `cargo check --workspace`: passed
- `cargo test --workspace`: 715 passed, 0 failed, 3 ignored
- `tasks/index.json` before this sweep: 90 entries, 1 active (`wontfix`), 89 `done`.

## Summary of findings

| Area | Done tasks reopened | New tasks created |
|------|---------------------|-------------------|
| Actor / bootstrap / lifecycle | 4 | 6 |
| Provider / config / auth | 4 | 2 |
| Session / loaders / DSL | 2 | 5 |
| TUI / markdown / input | 3 | 4 |
| Build / CI / errors / telemetry | 4 | 3 |
| **Total** | **17** | **20** |

After corrections, the registry stands at **110 tasks, 22 active, 86 done, 1 wontfix**.

## Done tasks reopened

| Task | Why it was reopened |
|------|---------------------|
| `eliminate-production-unwrap-expect` | Actor-spawn `unwrap`s remain in `ractor_config.rs`, `ractor_permission.rs`, `ractor_turn.rs`; `Leader::spawn_actors` returns `Result` but still panics. |
| `harden-actors-against-mutex-poisoning` | `runie-agent/src/actor.rs` still panics on missing provider/permission handles after emitting `Event::Error`. |
| `remove-sync-turn-queue-fallback-from-app-state` | `apply_queue_delivery_sync` still exists in `update/session.rs` as a sync fallback used in test mode. |
| `replace-config-validator-with-jsonschema` | `crates/runie-core/src/config/validate.rs` still exists and mixes jsonschema with hand-written registry validation. |
| `normalize-remaining-std-mutex-to-parking_lot` | Production `std::sync::Mutex`/`RwLock` remain in `session/tree.rs`, `provider/config.rs`, `declarative/register.rs`, `harness_skills`, and `runie-tui`. |
| `unify-provider-credential-resolution-with-dotenvy` | `runie-provider/src/config/mod.rs` still manually re-parses `.env` after `dotenvy::dotenv`. |
| `store-provider-api-keys-in-keyring-not-config` | `api_key` is still `String`, plaintext fallback remains, and `AuthToken.token` is `String`. |
| `add-jsonschema-validation-to-configactor-load` | `RactorConfigActor::pre_start` emits `ConfigLoaded` without validating the config. |
| `unify-session-store-and-index` | `sessions.json` index is still read by `/resume` and `SessionActor::Load`, but no longer written. |
| `replace-tui-markdown-block-layout-with-tui-markdown` | `tui-markdown` is only exercised by a test helper; production rendering still hand-rolls block layout. |
| `unify-markdown-block-parsing-on-pulldown-cmark-events` | `heal_markdown` still uses a char-level state machine; AC requires event-driven healing. |
| `replace-custom-helpers-with-crates` | `crates/runie-core/src/path.rs` still exists with a custom `normalize_path`. |
| `unify-library-error-types-with-thiserror` | `RunieError` is an unused `anyhow::Error` newtype; `anyhow::Result` still pervades library APIs. |
| `cleanup-small-duplicates-and-dead-code` | Dozens of repetitive `FIXME: Audit environment access` comments remain; undocumented `#[allow(dead_code)]` items remain. |
| `migrate-tui-and-cli-to-leader-bootstrap` | CLI does not use `Leader::start` at all. |
| `use-clap-derive-for-cli` | `config`/`mcp` subcommands from the AC are missing. |
| `centralize-built-in-tool-names` | Tool names are still hard-coded in `tool_runner.rs`, `turn/mod.rs`, and tests. |

## New tasks created

| Task | Priority | Brief description |
|------|----------|-------------------|
| `remove-unsafe-box_to_arc-from-leader-bootstrap` | P1 | Change factory/future to return `Arc<dyn LeaderAgentHandle>` directly. |
| `remove-mutex-eventbus-wrappers-from-actor-state` | P2 | Hold `EventBus` directly in ractor `State`; remove `Mutex`. |
| `unify-provider-reply-and-channel-wrappers` | P2 | Delete duplicate `Reply<T>` and `ProviderActorHandle`. |
| `implement-graceful-leader-shutdown` | P2 | Stop child actors and await turn join handle on shutdown. |
| `convert-agent-and-fff-indexer-to-ractor-state` | P2 | Move mutable actor-local state into `type State`. |
| `remove-dead-migration-scaffolding` | P3 | Delete `RactorActor`, `ActorHandles` alias, deprecated `InputActorHandle`, `send_message` alias. |
| `centralize-provider-credential-resolution` | P2 | Single resolver for env → dotenvy → keyring → config priority. |
| `remove-dead-yaml-line-parsers-from-resource-loader` | P2 | Delete `parse_yaml_line`/`strip_quotes` dead helpers. |
| `replace-normalize-raw-frontmatter-with-standard-splitter` | P2 | Use a frontmatter crate that handles `---` delimiters directly. |
| `unify-permissionmode-between-permissions-and-subagents` | P2 | Re-export canonical `PermissionMode`; remove duplicate enum. |
| `extend-markdown-inline-styles-to-lists-and-blockquotes` | P2 | Remove plain-text limitation in `BlockParser`. |
| `simplify-word-wrap-to-single-pass` | P2 | Honor first/rest widths in one `textwrap` pass. |
| `adopt-palette-for-theme-color-math` | P2 | Replace hand-rolled `darken`/`blend` with `palette`. |
| `normalize-std-sync-locks-in-tui-production-code` | P1 | Convert TUI production globals to `parking_lot`. |
| `fix-500-line-file-limit-violations` | P1 | Split files exceeding the 500-line guardrail. |
| `drop-once_cell-and-futures-util-workspace-deps` | P2 | Use `std::sync::LazyLock`; remove unused `futures-util`. |
| `fix-fff-search-pre-release-notify` | P2 | Avoid pre-release `notify 9.0.0-rc.4` via `fff-search`. |
| `convert-production-eprintln-to-tracing` | P2 | Replace production `eprintln!` with `tracing` events. |
| `rename-telemetry-rs-to-tracing-init` | P3 | Rename module to reflect that it only initializes tracing. |
| `unify-headless-runtime-bootstrap` | P3 | Consolidate CLI headless runtime paths. |

## Highest-priority remaining work

1. **CLI leader bootstrap** (`migrate-tui-and-cli-to-leader-bootstrap`) — the CLI and TUI have divergent lifecycles.
2. **Unsafe `box_to_arc`** — unnecessary `unsafe` in the leader bootstrap.
3. **Config validation at load time** (`add-jsonschema-validation-to-configactor-load`) — invalid configs can be accepted silently.
4. **Provider API keys in plaintext** (`store-provider-api-keys-in-keyring-not-config`) — security task is incomplete.
5. **500-line file-limit violations** — `ractor_config.rs` is 972 lines, violating the documented guardrail.
6. **Remaining actor panics** (`harden-actors-against-mutex-poisoning`, `eliminate-production-unwrap-expect`).

## Execution notes

- Land the P0/P1 actor and config fixes before cosmetic cleanups.
- Coordinate `centralize-provider-credential-resolution` with `store-provider-api-keys-in-keyring-not-config`.
- `unify-markdown-block-parsing-on-pulldown-cmark-events` should land before `extend-markdown-inline-styles-to-lists-and-blockquotes`.
- Every task must include the 4-layer test coverage required by `AGENTS.md`.
