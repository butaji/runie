# Compare file context picker and fix gaps

**Status**: blocked

> **Blocked by**: `build-runie-vs-grok-build-comparison-harness` (todo), `prepare-grok-build-reference-for-comparison` (todo), Grok Build fixtures not present
**Milestone**: R7
**Category**: Input / Commands
**Priority**: P1

**Depends on**: build-runie-vs-grok-build-comparison-harness, fix-tui-at-file-picker-shows-no-files
**Blocks**: none

## Description

Compare Grok Build's file context attachment (e.g. `@src/lib.rs`) with Runie's `@` file picker. Identify why Runie's picker shows “No files found” and fix it. Add E2E and live tmux parity tests.

## Scenario Set

1. Attach a single file: `@src/lib.rs`.
2. Attach multiple files: `@src/lib.rs @src/main.rs`.
3. Filter the picker with partial path.
4. Use file context in a follow-up prompt.

## Acceptance Criteria

- [ ] Each scenario runs in both tools.
- [ ] Runie `@` picker lists files from the current working directory.
- [ ] Selecting a file inserts `@path` into the input box.
- [ ] Actionable findings become tasks with unit + E2E + live tmux AC.
- [ ] `cargo test --workspace` passes after fixes.

## Tests

### Layer 1 — State/Logic
- [ ] `at_picker_populates_from_index` — picker suggestion list is non-empty when files exist.

### Layer 3 — Rendering
- [ ] `file_picker_renders_known_file` — `TestBackend` shows `Cargo.toml` after `@`.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_at_picker_lists_files` — live tmux script presses `@` and sees a file list.

## Files touched

- `crates/runie-core/src/actors/fff_indexer/ractor_fff_indexer.rs`
- `crates/runie-core/src/update/dialog.rs`
- `crates/runie-tui/src/ui_actor.rs`

## Fixture / Replay Strategy

Use recorded Grok Build TUI pane fixtures for `@` picker behavior. Derive Runie `TestBackend` expected buffers from the pane dumps. Do not invoke live Grok Build from `cargo test` or CI.

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- Directly overlaps with `fix-tui-at-file-picker-shows-no-files`; this task adds the Grok Build reference dimension.
