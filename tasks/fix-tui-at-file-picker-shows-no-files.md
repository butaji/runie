# Fix TUI @ file picker shows "No files found"

**Status**: todo
**Milestone**: R7
**Category**: Input / Commands
**Priority**: P2

**Depends on**: fix-tui-mock-simple-text-response-repetition
**Blocks**: none

## Description

Typing `@` in the input box opens the file picker dialog, but it immediately shows `No files found` even though the project directory contains many files. The picker therefore cannot be used to attach file context.

## Live Evidence

```
          ╭ Files ───────────────────────────────────────────────────╮
          │ ❯                                                        │
          │ ──────────────────────────────────────────────────────── │
          │   No files found                                         │
          ╰──────────────────────────────────────────────────────────╯
```

The project root (`/Users/admin/Code/GitHub/runie-dev`) has `Cargo.toml`, `crates/`, `docs/`, etc.

## Acceptance Criteria

- [ ] Typing `@` opens a file picker populated with files from the current working directory.
- [ ] The picker respects `.gitignore` and hidden-file rules consistently with the rest of the app.
- [ ] Selecting a file inserts `@path/to/file` into the input box.
- [ ] `cargo test --workspace` passes.
- [ ] Live tmux `@` scenario shows a non-empty file list.

## Tests

### Layer 1 — State/Logic
- [ ] `at_file_picker_populates_from_git_status` — given a mock file index, the picker suggestion list is non-empty.

### Layer 2 — Event Handling
- [ ] `at_trigger_opens_file_picker` — `@` key event opens the picker dialog and emits the correct indexing intent.

### Layer 3 — Rendering
- [ ] `file_picker_renders_file_list` — `TestBackend` shows at least one file path in the picker.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_at_picker_lists_files` — live tmux script presses `@` and asserts the captured pane contains a known file (e.g. `Cargo.toml`).

## Files touched

- `crates/runie-core/src/actors/fff_indexer/ractor_fff_indexer.rs`
- `crates/runie-core/src/update/dialog.rs`
- `crates/runie-tui/src/ui_actor.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- The picker may be waiting for an async `FffIndexerActor` result that never arrives, or the index may be filtered by git-status incorrectly.
- Check that `FffIndexerActor` is spawned and wired to the event bus during TUI bootstrap.
