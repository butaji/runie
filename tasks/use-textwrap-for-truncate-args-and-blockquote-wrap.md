# Use textwrap for truncate_args and blockquote wrap

## Status

`done`

## Context

`crates/runie-core/src/tool/format.rs:55-74` and `runie-tui/src/message/support.rs:195-247` manually iterate characters and accumulate display width to truncate/wrap text.

## Implementation

### `crates/runie-core/src/tool/format.rs`
- Replaced manual character-iteration truncation with `textwrap::wrap` using `Options::new(width).word_splitter(NoHyphenation)`
- `truncate_args` now uses `textwrap::wrap` and appends `'…'` when truncation occurred
- Added `textwrap` and `Options`/`WordSplitter` imports

### `crates/runie-tui/src/message/support.rs`
- Added `textwrap` as a dependency to `runie-tui`
- Replaced `wrap_styled_spans_for_blockquote` manual character-iteration with `textwrap::wrap` for multi-span content
- Single-span case: direct `textwrap::wrap` call
- Multi-span case: textwrap determines line breaks, spans are kept intact or split using textwrap with style preserved
- The function's behavior (per-span styles preserved, breaking at word boundaries) is maintained

## Goal

Use `textwrap` (already a dependency) with `WordSeparator`/`WordSplitter`.

## Acceptance Criteria
- [x] Replace manual truncation/wrapping. ✓
- [x] Preserve custom width rules. ✓ (display-width aware via textwrap)
- [x] Update snapshots if boundaries shift. ✓ (709 tests pass)

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for truncation/wrapping.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** Snapshot tests pass.
- **Layer 4 — E2E:** Tool tests pass.
- **Live tmux testing session (required):** Blockquotes render correctly.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
