# Consolidate Two Parallel TUI Test Hierarchies

**Status**: todo
**Milestone**: R2
**Category**: TUI Rendering
**Priority**: P1

## Description

`crates/runie-tui/src/` has two parallel test hierarchies:

**Hierarchy 1** (`src/tests.rs` + `src/tests/`):
- `src/tests.rs` (488 lines) — declares `mod code_blocks; mod color_restraint; mod colors; mod markdown; mod status_right; mod style_dsl; mod theme;`
- `src/tests/code_blocks.rs`, `tests/color_restraint.rs` (264 lines), `tests/colors.rs` (191 lines), `tests/markdown.rs`, `tests/status_right.rs` (357 lines), `tests/style_dsl.rs`, `tests/theme.rs`
- `src/tests/render/` (10 sub-files): `form.rs` (247 lines), `input.rs`, `input_box.rs` (215 lines), `mod.rs`, `panel_list.rs` (242 lines), `popup_bg.rs` (234 lines), `scoped_models.rs`, `scrollbar.rs` (183 lines), `timestamps.rs`, `transient.rs`

**Hierarchy 2** (`src/tui/tests/`):
- `src/tui/tests.rs` (or `mod tests` inside `src/tui.rs`) — different root
- `src/tui/tests/` (10 sub-dirs: `mode_transitions/`, `comprehensive_suite/`, `e2e_flow_tests/`, `reducer/`, `snapshot_regression_tests/`, `agent_events/`, plus 4 sub-sub-dirs of `agent_events/`)
- 30+ test files including 930-line `grok_parity_tests.rs`, 824-line `snapshot_regression_tests/grok_parity_tests.rs`, 783-line `tui/update/slash_tests.rs`, 748-line `tui/update/palette_tests.rs`, 742-line `session_management_tests.rs`, 713-line `grok_element_tests.rs`, 668-line `login_logout.rs`, 630-line `message_list/render/messages_test.rs`, 574-line `components/top_bar/tests.rs`
- Also `src/tui/tests_onboarding.rs` at the top level (a `#[cfg(test)]` module inside `tui.rs` itself)
- Also `src/tui/tests_statusbar.rs`
- Also `src/tui/tests_hotkeys/` directory
- Also `src/tui/tests/agent_events/{lifecycle,message_flow,tool_execution}.rs` (each 300-555 lines)

**Estimated test count:** ~400+ across both hierarchies, with significant
overlap (e.g. both hierarchies test palette behavior, both test
scrollbar metrics, both test message rendering).

The two hierarchies exist because there were at least 2 refactors that
moved tests around, and neither deleted the previous location.

## Acceptance Criteria

- [ ] One of the two test hierarchies is chosen as canonical
- [ ] All files in the deleted hierarchy are either:
  - Moved (with a one-line `git mv` history preserved) to the surviving hierarchy, OR
  - Deleted if their test cases are fully covered by files in the surviving hierarchy
- [ ] The module declaration in the surviving `mod.rs` (or `lib.rs`) lists every test module once and only once
- [ ] `cargo test -p runie-tui` runs the same set of test cases before and after the consolidation (zero net loss of coverage; no test names disappear from the output)
- [ ] No two test files share a `#[test] fn` name (catches a real risk: `tests::test_scroll_to_bottom` in `tui/tests/scrollbar.rs` and `tests::test_scrollbar` in `tests/render/scrollbar.rs` likely overlap)
- [ ] The test file layout follows the project convention: `mod.rs` + sibling `tests.rs` per module, with test cases organized by feature (e.g. `tests/scrollbar.rs` covers `scrollbar_metrics`, `tests/feed.rs` covers message feed, etc.)

## Tests

### Layer 1 — State/Logic
- [ ] `cargo test -p runie-tui` runs to completion with the same total test count as before (compare `cargo test -p runie-tui 2>&1 | grep -c '^test '` before and after)
- [ ] `cargo test -p runie-tui --no-run` succeeds (compiles)

### Layer 4 — Smoke
- [ ] `cargo test -p runie-term --test e2e -- --ignored` runs (verifies the e2e tests are not broken by the consolidation)

## Notes

**Recommendation:** keep `src/tests.rs` + `src/tests/` as canonical,
because:
1. It mirrors the `runie-core/src/tests/` convention
2. The 488-line `tests.rs` already has a sensible `mod` declaration
3. The `tests/render/` sub-directory is well-organized by widget

**The `src/tui/tests/` directory is the dead one.** It's reachable only
via the `mod tests { ... }` block inside `src/tui.rs:12231` (12k file),
which suggests `tui.rs` is also dead. But `crates/runie-tui/src/ui.rs`
is the actual `view` entry point (491 lines), and it's called from
`runie-term/src/main.rs:130` (`runie_tui::ui::draw_snapshot`). So
`ui.rs` is alive; `tui.rs` is not.

**Migration plan:**

1. Run `cargo test -p runie-tui 2>&1 | grep '^test ' | sort -u > /tmp/before.txt` to get the list of all test names.
2. Pick `src/tests.rs` + `src/tests/` as canonical.
3. For each test file in `src/tui/tests/`, check if a test with the same name exists in `src/tests/`. If yes, skip; if no, copy the file to the new location.
4. Update `src/tui.rs` to remove the `mod tests { ... }` declaration.
5. Run `cargo test -p runie-tui 2>&1 | grep '^test ' | sort -u > /tmp/after.txt`.
6. `diff /tmp/before.txt /tmp/after.txt` should be empty.

**The `src/tui/tests/grok_parity_tests.rs` files (930 + 824 = 1754
lines total)** look auto-generated. They are likely regression tests
that compare the TUI output against a Grok reference implementation.
Verify before deleting: are they still being maintained? Is there a
script that regenerates them? If yes, they should stay but be moved to
`src/tests/parity/`.

**Out of scope:**
- Merging the `grok_parity_tests.rs` files with each other
- Renaming tests to follow a single convention
- Splitting the 700+ line `tui/tests/session_management_tests.rs` into per-feature files
- The `replay`/`paint`/`pipe` rendering pipeline — these have tests in
  the `tui/` hierarchy; if `tui.rs` is dead, the render pipeline might
  be too. Separate task.

**Verification:**
```bash
# Same test count before and after
cargo test -p runie-tui 2>&1 | grep -c '^test ' > /tmp/before
# ... do the consolidation ...
cargo test -p runie-tui 2>&1 | grep -c '^test ' > /tmp/after
diff /tmp/before /tmp/after
```
