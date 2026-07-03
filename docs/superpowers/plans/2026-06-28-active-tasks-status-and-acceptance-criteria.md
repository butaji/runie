# Active Tasks — Final Status & Acceptance Criteria

**Date:** 2026-06-29  
**Scope:** The 24 tasks that were not `done` after the seventh-pass final sweep.  
**Current state:** 21 `done`, 3 `wontfix`, 0 active.

---

## Done (21 tasks)

### Actor runtime / bootstrap

| Task | Status | Acceptance Criteria |
|------|--------|---------------------|
| `remove-unsafe-box_to_arc-from-leader-bootstrap` | done | `AgentActorFactory::spawn` and `AgentSpawnFuture` return `Arc<dyn LeaderAgentHandle>`; `box_to_arc` deleted; tests pass. |
| `remove-mutex-eventbus-wrappers-from-actor-state` | done | `Mutex<EventBus>` removed from all actor `State` types; emit helpers updated; tests pass. |
| `unify-provider-reply-and-channel-wrappers` | done | Single shared `Reply<T>`; dead `ProviderActorHandle` removed; tests pass. |
| `implement-graceful-leader-shutdown` | done | `Leader` stores child cells + turn join handle; `shutdown` stops children and awaits join; tests pass. |
| `convert-agent-and-fff-indexer-to-ractor-state` | done | `AgentActor` and `FffIndexerActor` use `type State`; interior mutexes removed; tests pass. |
| `harden-actors-against-mutex-poisoning` | done | Actor modules use `parking_lot::Mutex`; missing handles emit errors instead of panicking; tests pass. |
| `remove-sync-turn-queue-fallback-from-app-state` | done | Sync fallback refactored/removed; tests use event-driven path; tests pass. |

### Provider / config / auth

| Task | Status | Acceptance Criteria |
|------|--------|---------------------|
| `centralize-provider-credential-resolution` | done | Single resolver covers env → dotenvy → keyring → config; manual `.env` re-parsing removed; tests pass. |
| `replace-config-validator-with-jsonschema` | done | `jsonschema` validates serialized config; `validate.rs` deleted; registry checks inlined; tests pass. |
| `unify-library-error-types-with-thiserror` | done | Hand-written errors converted; shared `RunieError` introduced; `anyhow` removed from library APIs; tests pass. |

### Session / loaders / DSL

| Task | Status | Acceptance Criteria |
|------|--------|---------------------|
| `remove-dead-yaml-line-parsers-from-resource-loader` | done | `parse_yaml_line` / `strip_quotes` deleted; exports removed; tests pass. |
| `replace-normalize-raw-frontmatter-with-standard-splitter` | done | `normalize_raw_frontmatter` deleted; standard `---` delimiter handling used; tests pass. |
| `unify-permissionmode-between-permissions-and-subagents` | done | Single canonical `PermissionMode`; duplicate enum removed; `FromStr` handles legacy strings; tests pass. |

### TUI / markdown / input

| Task | Status | Acceptance Criteria |
|------|--------|---------------------|
| `extend-markdown-inline-styles-to-lists-and-blockquotes` | done | List and blockquote states preserve inline styles; renderers honor them; tests pass. |
| `simplify-word-wrap-to-single-pass` | done | `word_wrap` honors first/rest widths in one pass; output matches previous two-pass implementation; tests pass. |
| `normalize-std-sync-locks-in-tui-production-code` | done | TUI globals use `parking_lot`; poison recovery removed; tests pass. |

### Build / CI / dependencies / telemetry

| Task | Status | Acceptance Criteria |
|------|--------|---------------------|
| `fix-500-line-file-limit-violations` | done | All production `.rs` files ≤ 500 lines; file-limit check passes; tests pass. |
| `drop-once_cell-and-futures-util-workspace-deps` | done | `once_cell` replaced with `std::sync::LazyLock`; `futures-util` removed; tests pass. |
| `convert-production-eprintln-to-tracing` | done | Production `eprintln!` replaced with `tracing` events; tests pass. |
| `rename-telemetry-rs-to-tracing-init` | done | `telemetry.rs` renamed to `tracing_init.rs`; call sites updated; tests pass. |
| `remove-dead-migration-scaffolding` | done | Dead `RactorActor`, `ActorHandles` alias, deprecated `InputActorHandle` removed; tests pass. |
| `unify-headless-runtime-bootstrap` | done | CLI uses canonical `runie-core::HeadlessRuntime`; duplicate path removed; tests pass. |

---

## Wontfix (3 tasks)

| Task | Status | Why it does not need to be done |
|------|--------|---------------------------------|
| `use-pulldown-cmark-for-tool-marker-stripping` | wontfix | Tool markers (`[TOOL_CALL]`, XML tags) are not valid CommonMark. The current string-based `strip.rs` is ~200 lines, correct, well-tested, and simpler than an event-stream rewrite. |
| `adopt-palette-for-theme-color-math` | wontfix | The hand-rolled `darken`/`blend` functions are simple sRGB interpolation, correct, tested, and dependency-free. `palette` would add compile time/binary size with no clear benefit for this use case. |
| `fix-fff-search-pre-release-notify` | wontfix | All `fff-search` versions require `notify 9.0.0-rc.4`. Replacing `fff-search` entirely is a larger refactor out of scope for cleanup. The two `notify` versions coexist without functional issues. |

---

## Verification

- `cargo check --workspace`: passed
- `cargo test --workspace`: passed
- `tasks/index.json`: 108 done, 3 wontfix, 0 active
