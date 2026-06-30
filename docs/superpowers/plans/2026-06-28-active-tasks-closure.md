# Active Tasks Closure — What Actually Remains

**Date:** 2026-06-28

The seventh-pass sweep (`2026-06-28-seventh-pass-final-sweep.md`) left **24 tasks** that were not yet `done`:

- **22 `todo`** tasks
- **1 `partial`** task (`replace-config-validator-with-jsonschema`)
- **1 `wontfix`** task (`use-pulldown-cmark-for-tool-marker-stripping`)

Since that sweep, implementation work has completed all 22 `todo` tasks and the 1 `partial` task. The current registry is:

- `tasks/index.json`: **111 entries**, **0 active**, **108 done**, **3 wontfix**

## The 24 tasks from the sweep — final state

### Completed (21 tasks)

| Task | Status | Why it was on the list |
|------|--------|------------------------|
| `remove-unsafe-box_to_arc-from-leader-bootstrap` | done | Removed unsafe `box_to_arc` from leader bootstrap. |
| `remove-mutex-eventbus-wrappers-from-actor-state` | done | Removed `Mutex<EventBus>` wrappers from actor `State`. |
| `unify-provider-reply-and-channel-wrappers` | done | Unified duplicate `Reply<T>` / `ProviderActorHandle`. |
| `implement-graceful-leader-shutdown` | done | Leader now stops children and awaits turn join handle. |
| `convert-agent-and-fff-indexer-to-ractor-state` | done | Moved mutable actor-local state into `type State`. |
| `centralize-provider-credential-resolution` | done | Single resolver for env → dotenvy → keyring → config. |
| `remove-dead-yaml-line-parsers-from-resource-loader` | done | Deleted `parse_yaml_line` / `strip_quotes` dead helpers. |
| `replace-normalize-raw-frontmatter-with-standard-splitter` | done | Frontmatter uses standard `---` delimiter handling. |
| `unify-permissionmode-between-permissions-and-subagents` | done | Single canonical `PermissionMode` enum. |
| `extend-markdown-inline-styles-to-lists-and-blockquotes` | done | Lists/blockquotes preserve inline styles. |
| `simplify-word-wrap-to-single-pass` | done | `word_wrap` honors first/rest widths in one pass. |
| `normalize-std-sync-locks-in-tui-production-code` | done | TUI globals converted to `parking_lot`. |
| `fix-500-line-file-limit-violations` | done | Files split to comply with the 500-line guardrail. |
| `drop-once_cell-and-futures-util-workspace-deps` | done | Replaced `once_cell` with `LazyLock`; dropped `futures-util`. |
| `convert-production-eprintln-to-tracing` | done | Production `eprintln!` replaced with `tracing` events. |
| `rename-telemetry-rs-to-tracing-init` | done | Module renamed to `tracing_init.rs`. |
| `harden-actors-against-mutex-poisoning` | done | Actor panics on missing handles converted to errors. |
| `remove-sync-turn-queue-fallback-from-app-state` | done | Sync `TurnQueue` fallback removed. |
| `replace-config-validator-with-jsonschema` | done | Validator fully replaced by `jsonschema`. |
| `unify-library-error-types-with-thiserror` | done | Library errors unified on `thiserror`. |
| `unify-headless-runtime-bootstrap` | done | CLI headless runtime path unified. |

### Explicitly `wontfix` (3 tasks)

These tasks do **not** need to be done. Each has a clear rationale in its task file.

| Task | Rationale |
|------|-----------|
| `use-pulldown-cmark-for-tool-marker-stripping` | Tool markers (`[TOOL_CALL]`, XML tags) are not valid CommonMark, so `pulldown-cmark` would still need custom scanning. The current string-based `strip.rs` is ~200 lines, correct, and well-tested. |
| `adopt-palette-for-theme-color-math` | The hand-rolled `darken`/`blend` functions are simple sRGB interpolation, correct, tested, and dependency-free. `palette` would add compile time/binary size with no clear benefit for this use case. |
| `fix-fff-search-pre-release-notify` | All versions of `fff-search` require `notify 9.0.0-rc.4`. Replacing `fff-search` entirely is a larger refactor out of scope for cleanup. The two `notify` versions coexist without functional issues. |

## What this means

- **No active implementation work remains** from the cleanup roadmap.
- The 3 `wontfix` tasks are intentional decisions, not backlog.
- Any new work should come from a new planning cycle or user requirement, not from reopening these tasks.

## Verification

- `cargo check --workspace`: passed
- `cargo test --workspace`: passed
- `tasks/index.json`: 108 done, 3 wontfix, 0 active
