# Replace fff-search indexer with ignore/walkdir

## Status

`done`

## Context

`crates/runie-core/src/actors/fff_indexer/` wraps `fff-search` 0.9.5-nightly, which pulls in a pre-release `notify 9.0.0-rc.4` conflicting with the workspace `notify 7.0`. It also maintains a custom global registry and manual git-status formatting.

## Goal

Replace the FFF indexer with `ignore::WalkBuilder`/`walkdir` for traversal, `nucleo-matcher`/`sublime_fuzzy` for fuzzy ranking, and the workspace `notify` for file watching. Remove the `fff-search` dependency and the duplicate `notify` version.

## Acceptance Criteria

- [ ] Remove `fff-search` from dependencies.
- [ ] Implement indexing with `ignore` + `walkdir`.
- [ ] Preserve frecency/query persistence or replace with a simpler MRU cache.
- [ ] `@` picker and `search` tool behavior is unchanged.
- [ ] `cargo tree -d` no longer shows `notify` duplicates.

## Design Impact

No change to TUI element design or composition. Only file-search behavior becomes more correct (gitignore-aware).

## Tests

- **Layer 1 — State/Logic:** Unit tests for indexing and scoring in a temp repo.
- **Layer 2 — Event Handling:** `IoMsg::Search` returns expected results.
- **Layer 3 — Rendering:** `TestBackend` `@` picker shows the same ranked files.
- **Layer 4 — E2E:** Headless CLI file search returns correct results.
- **Live tmux validation:** Open `@` picker in a repo with `.gitignore`; ignored files are excluded and ranking feels similar.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
