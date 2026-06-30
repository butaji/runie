# Drop legacy diff parser and rely on diffy

## Status

`todo`

## Context

`crates/runie-core/src/diff/mod.rs:203-319` keeps a `legacy_parse_diff` state machine for imperfect agent output, plus a manual `HunkBuilder`. `diffy` (already a dep) is used only as a best-effort parse with fallback to the custom parser.

## Goal

Delete the legacy parser and `HunkBuilder`; normalize imperfect diffs with a small pre-processing pass and rely entirely on `diffy::Patch::from_str` and `similar::TextDiff`.

## Acceptance Criteria

- [ ] Remove `LegacyParseState` and related helpers.
- [ ] Remove manual `HunkBuilder`; use `diffy`/`similar` directly.
- [ ] Add a small normalization pass for common agent diff deviations (missing newlines, context markers) before `diffy` parsing.
- [ ] All existing diff tests pass.

## Design Impact

No change to TUI element design or composition. Only diff parsing behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for `diffy::Patch` parsing on well-formed and slightly malformed diffs.
- **Layer 2 — Event Handling:** `IoMsg::ApplyDiff` emits the correct file-change facts.
- **Layer 3 — Rendering:** `TestBackend` diff widget snapshots are unchanged.
- **Layer 4 — E2E:** Provider replay fixture applies a tool-generated diff successfully.
- **Live tmux validation:** Ask the agent to edit a file; verify the diff preview and applied result match expectations.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
