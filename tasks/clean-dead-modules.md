# Remove Dead Modules and Unused Code

**Status**: todo
**Milestone**: R3
**Category**: Core Architecture
**Priority**: P2
**Depends on**: resolve-merge-conflicts

## Description

The code review identified several modules and items that are written
but never invoked. Each was a partial experiment or a leftover from
a refactor. They inflate the binary, confuse newcomers, and require
maintenance (formatting, lint fixes, test updates) for no benefit.

## Acceptance Criteria

- [ ] **`crates/runie-agent/src/mutation_queue.rs` (299 lines, 70%
  tests) is removed.** `FileMutationQueue` is never called from
  anywhere in the codebase. The actual `Tool::WriteFile` and
  `Tool::EditFile` paths in `tools.rs` call `std::fs::write`
  directly.
- [ ] **`crates/runie-core/src/dialog/flow/` (6 files, ~600 lines
  total) is removed.** The `Flow`, `Step`, `push`, `pop`, `close`
  primitives in `flow_def.rs`, `executor.rs`, `step.rs`,
  `transitions.rs`, `validators.rs`, `context.rs` have zero
  non-test call sites. The `mod.rs:58` docstring example
  references the API but no production code uses it.
- [ ] **`crates/runie-core/src/dialog/dsl/conversions.rs` traits
  `FromStringExt` and `FromEventExt` are removed** (but the blanket
  `impl From<Event> for ItemAction` stays — it has callers).
- [ ] **`crates/runie-core/src/dialog/dsl/form.rs` (126 lines) is
  removed** (its `form()` constructor is folded into the surviving
  `dialog/dsl/mod.rs` per `deduplicate-panel-types`).
- [ ] **`commands/dsl/builder.rs:152` `dsl_form` method is removed**
  (no call site; only `FromStringExt::into_action` chain reaches it
  via `FormBuilder::from_dsl_panel`).
- [ ] **`commands/dsl/builder.rs:265` `FormBuilder::from_dsl_panel`
  is removed** (only consumer was `dsl_form`).
- [ ] **`commands/dsl/flow.rs:113` `build_form_panel` is removed**
  if its only consumer (the `Form` variant of `CommandFlow`) is
  itself removed — the existing `form()` method on `CommandDef`
  builds the panel inline; `build_form_panel` is dead.
- [ ] **`event.rs:255` `Event::LoginFlowValidate` variant is
  removed** if `update/login_flow.rs` has no handler for it (the
  current code has `_ => {}` fallthrough, per `deduplicate-login-flow`).
- [ ] **`event.rs` `EventCategory` variants that are unreachable from
  the dispatch table are removed** — verify each variant in
  `EventCategory` is actually used in `update/mod.rs:99-110`.
- [ ] **`crates/runie-tui/src/tui.rs` (12k file) is removed** if its
  `mod tests { ... }` block is the only content — the file appears
  to be a single `#[cfg(test)] mod tests` block, which is dead
  weight.
- [ ] **`crates/runie-tui/src/actors/` directory is removed** if the
  actor pattern lives in `runie-term` (per `main.rs:80-110`), not
  in `runie-tui`.
- [ ] **`crates/runie-tui/src/replay/` directory is removed** if
  `scenario_replay.rs` is the only consumer and the consumer is
  itself a test binary.
- [ ] **`crates/runie-tui/src/paint/`, `pipe/`, `style/`,
  `theme/themes/` directories are removed** if their functions are
  not called from the surviving render path in
  `crates/runie-tui/src/ui.rs` (the only `pub fn draw_snapshot` is
  in `ui.rs`).
- [ ] `cargo build --workspace` succeeds after the deletions
- [ ] `cargo test --workspace` succeeds
- [ ] `cargo clippy --workspace` produces no new warnings

## Tests

### Layer 1 — State/Logic
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --workspace` succeeds with the same or higher test count (the deleted tests were mostly internal to the dead modules, so test count should drop by 20-50 but the surviving test suite should be 100% green)
- [ ] `cargo clippy --workspace` has no new warnings

### Layer 4 — Smoke
- [ ] `tmux_login_logout_test.sh` passes
- [ ] `./dev.sh` runs end-to-end

## Notes

**Pre-flight check** — before deleting any module, run:

```bash
# Find every non-test reference
git grep -nE 'crate::dialog::flow::|mutation_queue::|FileMutationQueue' \
  -- 'crates/' ':!*test*' ':!*tests/*'
```

If a non-test reference exists, the deletion is not safe.

**Conservative order of deletion** (each step builds cleanly):

1. `mutation_queue.rs` (zero references)
2. `dialog/dsl/conversions.rs` traits (only the blanket impl stays)
3. `dialog/dsl/form.rs` (folded into `dialog/dsl/mod.rs`)
4. `dialog/flow/*` (6 files, zero non-test callers)
5. `dsl_form`, `from_dsl_panel`, `build_form_panel` in `commands/dsl/*`
6. `Event::LoginFlowValidate` variant
7. `runie-tui/src/tui.rs`, `actors/`, `replay/`, `paint/`, `pipe/`, `style/`, `theme/themes/` — verify each has no callers in `runie-term/src/main.rs` first

**`crates/runie-tui/src/tui.rs` (12k file) is suspicious.** A
12k-line file in a TUI crate is huge. The current file likely
contains the `mod tests { ... }` block that declares
`tui/tests::comprehensive_suite`, `tui/tests::agent_events`, etc. If
those test modules are also dead (i.e. the tests don't add coverage
beyond the `src/tests/` hierarchy), the whole `tui/` sub-tree is
dead.

**`crates/runie-tui/src/bin/` has 3+ binaries** (grok_parity_test,
scenario_replay, scenario_fasthot, runie-dspec). These are
*binaries*, not tests in the strict sense, but they live in `bin/`
and get built. They each have an `ALLOWED_FILES_OVER` entry in
`build.rs` (or did — the allow-list is now minimal). If the binaries
are smoke-test scaffolding, keep them but exclude from
`cargo test`. If they're abandoned, delete.

**Out of scope:**
- Renaming or moving surviving modules
- Consolidating `commands/dsl/{builder,flow}` (they're tangled)
- Splitting `commands/dsl/builder.rs` (185-line `register()` function)

**Verification:**
```bash
# Build clean
cargo build --workspace

# Same-or-fewer warnings
cargo clippy --workspace 2>&1 | grep -c warning

# Tests still pass
cargo test --workspace

# Smoke
./dev.sh
```
