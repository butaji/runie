# Unify diff generation and parsing on one crate

## Status

`todo`

## Context

`diff/mod.rs` uses `similar` for generation and `diffy` for parsing. `harness_skills/hashline_edit.rs` also uses `similar`. Two diff libraries are in deps.

## Goal

Evaluate whether `diffy::create_patch` is acceptable for generation; if so, drop `similar`. Otherwise, use `similar` for both generation and parsing.

## Acceptance Criteria

- [ ] Pick one crate for both generating and parsing diffs.
- [ ] Migrate `Diff::generate`, `Diff::parse`, and hashline edit.
- [ ] All diff tests pass; TUI diff rendering unchanged.

## Design Impact

No change to TUI element design or composition. Only diff implementation changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for diff generation and parsing.
- **Layer 2 — Event Handling:** `IoMsg::ApplyDiff` emits same facts.
- **Layer 3 — Rendering:** Diff widget snapshots match.
- **Layer 4 — E2E:** Provider replay fixture applies a diff.
- **Live tmux validation:** Ask the agent to edit a file; diff preview matches.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
