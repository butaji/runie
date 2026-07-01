# Delete ProviderConfigBox alias and rename DynProviderFactory

## Status

`todo`

## Context

`ProviderConfigBox` type alias is still defined and re-exported despite zero usages; the production factory is still named `DynProviderFactory` after `DynProvider` was deleted; docs still reference the old wrapper.

## Goal

Remove the alias and re-exports; rename the factory; update stale doc comments and `AGENTS.md` example.

## Acceptance Criteria
- [ ] Delete `ProviderConfigBox` alias and re-exports.
- [ ] Rename `DynProviderFactory` to `ProviderFactory` (or `BuiltProviderFactory`).
- [ ] Update factory doc comment.
- [ ] `cargo check --workspace` passes.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** All provider/agent tests pass.
- **Live tmux validation:** `/provider` and headless smoke tests work.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
