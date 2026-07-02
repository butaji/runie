# Merge duplicate TUI widget replacement tasks

## Status

`done`

## Description

`replace-custom-form-rendering-with-tui-textarea.md` and `replace-custom-input-box-with-tui-textarea.md` overlap with `finish-replacing-custom-tui-widgets.md`. Merged descriptions and closed duplicates.

## Changes made

1. **Marked `replace-custom-input-box-with-tui-textarea.md`**: Already done (input box replacement completed)

2. **Marked `replace-custom-form-rendering-with-tui-textarea.md`**: Superseded - it's a subset of the canonical task. Updated to reference `finish-replacing-custom-tui-widgets.md`.

3. **Updated `finish-replacing-custom-tui-widgets.md`**: 
   - Added status section showing input box is done
   - Added note that form renderer is tracked here
   - Updated acceptance criteria to show input box is complete

## Acceptance criteria

1. **Unit tests** — N/A; backlog task.
2. **E2E tests** — N/A; backlog task.
3. **Live tmux tests** — N/A; backlog task.

## Completion Validation

- [x] **Task merged** — `replace-custom-form-rendering-with-tui-textarea.md` references canonical task.
- [x] **Canonical task updated** — `finish-replacing-custom-tui-widgets.md` includes status of completed work.
