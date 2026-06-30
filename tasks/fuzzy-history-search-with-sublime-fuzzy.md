# Fuzzy history search with sublime-fuzzy

## Status

`todo`

## Context

`crates/runie-core/src/input_history.rs:112-141` uses hand-written prefix and substring search loops for history lookup. `sublime_fuzzy` is already a workspace dependency and would give better UX.

## Goal

Replace prefix/substring history search with fuzzy scoring via `sublime_fuzzy` or `nucleo-matcher`, and surface the best matches first.

## Acceptance Criteria

- [ ] Replace `filter_history` / `search_history` with fuzzy matching.
- [ ] Keep exact substring matches highly ranked.
- [ ] Ensure performance is acceptable for large history files.
- [ ] History navigation (`Up`/`Down`, `/history`) works as before.

## Design Impact

No change to TUI element design or composition. Only the ordering/quality of history suggestions changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for fuzzy ranking on history entries.
- **Layer 2 — Event Handling:** `InputMsg::HistorySearch` returns ranked matches.
- **Layer 3 — Rendering:** `TestBackend` `/history` popup shows fuzzy-ranked entries.
- **Layer 4 — E2E:** Headless CLI `/history` query returns matches.
- **Live tmux validation:** Type part of a previous message and press `Up`; the best fuzzy match is inserted.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
