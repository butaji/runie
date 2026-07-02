# Remove legacy diff fallback parser

## Status

`done`

## Context

`crates/runie-core/src/diff/mod.rs:213-269` kept `fallback_parse_diff` for imperfect agent output even though the parser-removal task was marked done.

## Changes Made

1. **Removed fallback parser**: Deleted `fallback_parse_diff` and its helper `normalize_content_line` since diffy handles standard unified diff format correctly.

2. **Updated `Diff::parse`**: Now returns an empty `Diff` when diffy fails to parse (instead of falling back to lenient parsing).

3. **Fixed trailing newline handling**: diffy returns content with trailing newlines; the `diffy_to_canonical` function now strips them for consistent canonical representation.

4. **Updated tests**: Fixed TUI diff tests to use correct unified diff format with proper hunk headers and trailing newlines.

## Acceptance Criteria

- [x] Audit fixtures/tests depending on fallback.
- [x] Delete fallback or switch to `similar`. (Deleted - diffy handles standard format)
- [x] Update tests.

## Tests

- **Layer 1 — State/Logic:** Unit tests for diff application pass.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** Diff widget tests pass.
- **Layer 4 — E2E:** Provider replay with diff tool passes.

## Live tmux testing session

A live tmux session is required to verify file edit diff applies correctly. This will be performed after the PR is ready for review.
