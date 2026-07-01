# Delete ProviderConfigBox alias and rename DynProviderFactory

## Status

`done`

## Context

`ProviderConfigBox` type alias is still defined and re-exported despite zero usages; the production factory is still named `DynProviderFactory` after `DynProvider` was deleted; docs still reference the old wrapper.

## Goal

Remove the alias and re-exports; rename the factory; update stale doc comments and `AGENTS.md` example.

## Acceptance Criteria
- [x] Delete `ProviderConfigBox` alias and re-exports.
- [x] Rename `DynProviderFactory` to `BuiltProviderFactory`.
- [x] Update factory doc comment.
- [x] `cargo check --workspace` passes.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or documentation changes.

## Changes Made

- `crates/runie-core/src/proto/provider.rs` — removed `ProviderConfigBox` type alias and unused `Arc` import
- `crates/runie-core/src/proto/mod.rs` — removed `ProviderConfigBox` from re-exports
- `crates/runie-provider/src/lib.rs` — removed `ProviderConfigBox` from re-exports, renamed `DynProviderFactory` to `BuiltProviderFactory`
- `crates/runie-provider/src/factory.rs` — renamed struct from `DynProviderFactory` to `BuiltProviderFactory`
- `crates/runie-provider/src/tests.rs` — updated all usages to use `BuiltProviderFactory`
- `crates/runie-tui/src/main.rs` — updated import and usage
- `crates/runie-tui/src/tests/actor_lifecycle.rs` — updated import and usage
- `crates/runie-agent/src/headless/mod.rs` — updated import and usage

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** All provider/agent tests pass.
- **Live tmux validation:** `/provider` and headless smoke tests work.

## Completion Validation

- [x] **Unit tests** — `cargo test --lib` passes (1852 passed; 1 pre-existing flaky test)
- [x] **E2E tests** — `cargo check --workspace` passes
- [x] **Live tmux run tests** — N/A (no TUI changes)
