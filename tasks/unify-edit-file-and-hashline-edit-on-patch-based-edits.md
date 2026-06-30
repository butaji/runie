# Unify edit_file and hashline_edit on patch-based edits

## Status

`todo`

## Context

`crates/runie-agent/src/tool/edit_file.rs` does naive search/replace, while `crates/runie-core/src/harness_skills/hashline_edit.rs` does line-number + hash-addressed edits. Both read/write files manually and both re-implement diff formatting.

## Goal

Unify both on patch-based edits using `diffy`/`similar` (already in deps). `edit_file` should accept a unified diff or search/replace block and apply it via `diffy::Patch`; hashline becomes a thin validation layer on top.

## Acceptance Criteria

- [ ] Remove duplicated file read/write logic.
- [ ] Use `diffy::Patch` (or `similar::TextDiff`) as the single application path.
- [ ] Support both diff blocks and hashline-style edits through the same code.
- [ ] All existing edit tests pass.

## Design Impact

No change to TUI element design or composition. Only file-edit tool behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for applying diff and hashline edits to temp files.
- **Layer 2 — Event Handling:** `IoMsg::ApplyDiff` emits the correct change facts.
- **Layer 3 — Rendering:** Diff widget snapshots match.
- **Layer 4 — E2E:** Provider replay fixture edits a file successfully.
- **Live tmux validation:** Ask the agent to edit a file; the diff preview and applied result match expectations.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
