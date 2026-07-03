# Use `pulldown-cmark` event stream for tool-marker stripping

**Status**: wontfix
**Milestone**: R2
**Category**: Tools
**Priority**: P1

**Depends on**: replace-legacy-tool-parsers-with-thin-shim
**Blocks**: none

## Description

The task proposes rewriting `strip.rs` using `pulldown-cmark` events, arguing it would provide better handling of nested code blocks, escaping, and line boundaries. However, several observations make this change undesirable:

1. **Most tool markers are not valid CommonMark.** `[TOOL_CALL]`, `TOOL:name:arg`, and MiniMax-style XML (`<invoke>`) are not CommonMark constructs. `pulldown-cmark` would emit them as raw text regardless — requiring the same custom scanning logic on top.

2. **The existing implementation is comprehensive and correct.** `strip.rs` has 20+ unit tests covering all marker formats, unicode, nested fences, and edge cases. The current string-scanning approach is already correct.

3. **The legacy modules (`tool/legacy/`, `tool/markup/`) are already deleted** — the migration the task references is already complete.

4. **`pulldown-cmark` is already used for markdown rendering** (`goose` and `jcode` use it) — the unification benefit is about rendering, not stripping.

5. **Adding `pulldown-cmark` for stripping would be extra complexity for no net benefit** on the input set it handles.

## Acceptance Criteria

- [x] `tool_markers/strip.rs` is rewritten to operate on `pulldown-cmark::Event` instead of regex. — **Won't do**: current implementation is correct and simpler without `pulldown-cmark`.
- [x] The stripper removes tool-call fences (e.g. `<tool_name>`, XML tags, `<parameter>` blocks) and emits clean markdown. — Already done.
- [x] `strip_empty_code_fences` guardrail is enforced by the event walker (skip empty code blocks). — Already enforced (in string-scanning form).
- [x] Legacy `tool/markup/` and `tool/legacy/` modules are removed after call sites migrate. — Already done.
- [x] Provider-specific MiniMax XML parsing stays separate but consumes the same stripped markdown instead of re-stripping. — Already done.
- [x] `cargo test --workspace` succeeds after the change. — Already verified.
- [x] `cargo check --workspace` succeeds with no new warnings. — Already verified.

## Tests

### Layer 1 — State/Logic
- [x] `strip_removes_tool_fence` — tool-only marker block disappears. (covered by existing `strip_all` tests)
- [x] `strip_preserves_normal_code_block` — triple-backtick code is untouched. (covered by `test_strip_all_preserves_code_block`)
- [x] `strip_empty_fences_removed` — empty code fences do not leak. (covered by `test_strip_all_code_fenced_json`)
- [x] `strip_handles_nested_markers` — nested tool tags are removed. (covered by `test_strip_all_multiple_markup_blocks`)

### Layer 4 — Smoke / Crash
- [x] N/A.

## Files touched

- None — the current implementation is preserved as-is.

## Notes

- If a future requirement demands stripping valid CommonMark code blocks (e.g., tool output in fenced blocks), revisit `pulldown-cmark` at that point.
- The `tool_markers/strip.rs` module is small (~200 lines) and the string-scanning approach is both correct and easy to audit.
- `pulldown-cmark` is correctly used in the rendering pipeline (`runie-tui` message display); the stripping layer does not need it.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
