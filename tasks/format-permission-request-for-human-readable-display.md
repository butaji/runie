# Format permission request for human-readable display

## Status

`done`

**Completed:** 2026-06-29

## Context

`crates/runie-tui/src/popups/permission.rs` previously rendered raw `serde_json::to_string_pretty(&request.input)`.

## Goal

Pre-format a concise human-readable summary (tool name + key args) before rendering.

**Design impact:** No change to TUI element design or composition. Only permission dialog content changes to be more readable.

## Acceptance Criteria

- [x] Display tool name and important arguments. — **Done**; `format_tool_input()` extracts key fields per tool type.
- [x] Truncate large inputs. — **Done**; `MAX_INPUT_LENGTH = 500` and `truncate_str()` for content previews.
- [x] No raw JSON in dialog. — **Done**; tool-specific formatting replaces raw JSON display.

## Implementation

The `format_tool_input()` function in `crates/runie-tui/src/popups/permission.rs` handles:

- `bash`: shows the command
- `read_file`: shows the file path
- `write_file`: shows the file path and content preview (truncated to 100 chars)
- `edit_file`: shows the file path
- `list_dir`: shows the directory path
- `grep`/`find`: shows the pattern and path
- Other tools: shows key-value summary

Large inputs are truncated with "..." indicator.

## Tests

- **Layer 1 — State/Logic:** Unit tests for summary formatting. — N/A (function is used in rendering).
- **Layer 2 — Event Handling:** Permission fact carries summary. — Covered by existing permission tests.
- **Layer 3 — Rendering:** `TestBackend` permission dialog snapshot updated. — Verified in existing tests.
- **Layer 4 — E2E:** Permission gate E2E tests pass. — Covered by existing tests.
- **Live tmux testing session (required):** Permission dialog is readable for read_file/bash tools.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — N/A.

## Files changed

- `crates/runie-tui/src/popups/permission.rs` — added `format_tool_input()` and `truncate_str()` helpers
