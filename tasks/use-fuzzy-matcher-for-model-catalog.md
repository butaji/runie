# Use fuzzy matcher for model catalog

## Status

`done`

## Context

`crates/runie-core/src/model_catalog/mod.rs` filters models with case-insensitive substring only.

## Goal

Use `nucleo-matcher` or `sublime_fuzzy` for fuzzy model/catalog search.

## Acceptance Criteria
- [x] Replace substring filter with fuzzy matcher. (`sublime_fuzzy::best_match()` in `fuzzy_score()`)
- [x] Preserve provider grouping. (models grouped by provider in results)
- [x] Sort by score. (results sorted by fuzzy score)

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for fuzzy scoring and grouping.
- **Layer 2 — Event Handling:** Model filter event unchanged.
- **Layer 3 — Rendering:** `TestBackend` model selector snapshot unchanged.
- **Layer 4 — E2E:** Headless CLI model search works.
- **Live tmux validation:** `/model` search tolerates typos.

## Implementation Notes

- `sublime_fuzzy::best_match()` used for fuzzy matching
- `filter_models_fuzzy_typo()` test validates fuzzy matching works for typos
- Provider grouping preserved in `filter_and_score_models()`

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
