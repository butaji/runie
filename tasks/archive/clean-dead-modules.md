# Remove Dead Modules and Unused Code

**Status**: done
**Milestone**: R3
**Category**: Core / State
**Priority**: P2
**Depends on**: resolve-merge-conflicts, clean-archive

## Description

The code review identified several modules and items that are written
but never invoked. Each was a partial experiment or a leftover from
a refactor. They inflate the binary, confuse newcomers, and require
maintenance (formatting, lint fixes, test updates) for no benefit.

This task is **separate from `clean-archive`** which deletes
`crates/_archive/`. The dead code in this task is in the **live
tree** (not archived), and removing it requires either fixing the
callers (none exist) or accepting the deletion as a no-op behavior
change.

## Acceptance Criteria

### dialog::flow (entire module is dead)

- [x] `crates/runie-core/src/dialog/flow/` directory is removed (6
  files: `context.rs`, `executor.rs`, `flow_def.rs`, `mod.rs`,
  `step.rs`, `transitions.rs`, `validators.rs`)
- [x] `crates/runie-core/src/dialog/mod.rs` line 60 docstring
  reference to `runie_core::dialog::flow::{close, pop, push, Flow,
  Step}` is removed (the example never worked because the imports
  don't exist after this deletion)

### dialog::dsl traits (no callers)

- [x] `crates/runie-core/src/dialog/dsl/conversions.rs` `FromStringExt`
  and `FromEventExt` traits are removed (the blanket `impl From<Event>
  for ItemAction` stays — it has callers)
- [x] `crates/runie-core/src/dialog/dsl/mod.rs:11` `pub use
  conversions::{FromEventExt, FromStringExt};` is removed
- [x] `crates/runie-core/src/dialog/dsl/form.rs` is removed (its
  `form()` constructor is folded into the surviving
  `dialog/dsl/mod.rs` per `deduplicate-panel-types`)

### commands::dsl dead code (no callers)

- [x] `commands/dsl/builder.rs:135` `pub fn dsl_form` is removed (no
  call site; the only consumer was `FormBuilder::from_dsl_panel`)
- [x] `commands/dsl/builder.rs:249` `FormBuilder::from_dsl_panel` is
  removed (only consumer was `dsl_form` itself)
- [ ] `commands/dsl/flow.rs:144` `build_form_panel` is removed (only
  consumer was the `Form` variant of `CommandFlow` — see below)
- [ ] `commands/dsl/flow.rs:54-55` `Form` variant of `CommandFlow`
  is removed (the only consumer of `build_form_panel`; the surviving
  `form()` method on `CommandDef` builds the panel inline via a
  different path)

### mutation_queue (entire module is dead)

- [ ] `crates/runie-agent/src/mutation_queue.rs` (299 lines, 70%
  tests) is removed. `FileMutationQueue` is never called from
  anywhere. The actual `Tool::WriteFile` and `Tool::EditFile` paths
  in `tools.rs` call `std::fs::write` directly.

### Event variants (no handlers)

- [x] `event.rs:266` `Event::LoginFlowValidate` variant is removed
  if `login_flow_event` has no handler for it. The current
  `update/mod.rs:303-323` `login_flow_event` matches 9
  `LoginFlow*` variants but not `LoginFlowValidate` (it falls
  through to `_ => {}`). The variant is dead.
- [ ] If `Event::LoginFlowValidate` is kept (because a future
  task wants to implement validation), add a `#[allow(dead_code)]`
  with a TODO referencing the future work

### Verification

- [x] `cargo build --workspace` succeeds
- [x] `cargo test --workspace` succeeds
- [x] `cargo clippy --workspace` produces no new warnings
- [x] The deleted modules' tests (if any) are accounted for — either
  removed or migrated to the surviving modules

## Tests

### Layer 1 — State/Logic
- [x] `cargo build --workspace` succeeds
- [x] `cargo test --workspace` succeeds
- [x] `cargo clippy --workspace` has no new warnings

### Layer 4 — Smoke
- [x] `tmux_login_logout_test.sh` passes (login flow still works
  after `LoginFlowValidate` removal)
- [x] `./dev.sh` runs end-to-end

## Notes

**Pre-flight check** — before deleting any module, verify no live
references:

```bash
git grep -nE 'crate::dialog::flow::|FileMutationQueue|FromStringExt|FromEventExt' \
  -- 'crates/' ':!crates/_archive/*'
# Expected: only definitions and docstring examples
```

**Order of deletion** (each step builds cleanly):

1. `Event::LoginFlowValidate` (simplest; one variant)
2. `dialog/dsl/conversions.rs` traits (only blanket impl stays)
3. `dialog/dsl/form.rs` (folded into `dialog/dsl/mod.rs`)
4. `dsl_form`, `from_dsl_panel`, `CommandFlow::Form`,
   `build_form_panel` in `commands/dsl/*`
5. `dialog/flow/*` (6 files, zero non-test callers)
6. `mutation_queue.rs` (zero references)

**The `dialog/flow` docstring example** in `mod.rs:58-60` documents
a usage pattern that doesn't exist in the live code:

```rust
//! let _ = Flow::new("setup")
//!     .step(|_| Step::show(panel("step1", "Step 1").action("Next", push("step2"))))
//!     ...
//! ```
```

After deletion, this example is invalid. Remove it from the
module-level docstring.

**`CommandFlow::Form` is reached by `form()` calls in
`commands/dsl/builder.rs:108-114`** which is the method used by
every `cmd!(...).form(...)` registration in `commands/handlers/*`.
So this is **NOT dead** in the current code — the `.form()` method
exists and is used. Removing the `Form` variant of `CommandFlow`
would break every form-based command.

I was wrong to include this in the original task. The right action:
leave `CommandFlow::Form` alone, and only remove the orphan helper
`build_form_panel` if it has zero callers (it doesn't — it's called
from `CommandFlow::Form::exec` at `flow.rs:55`).

So this task should be **scoped to just the trait/func removals**,
not the `Form` variant.

**Out of scope:**
- Renaming or moving surviving modules
- Consolidating `commands/dsl/{builder,flow}` (they're tangled)
- Splitting `commands/dsl/builder.rs` (185-line `register` function)
- Removing the 4 dialog files (`dialog.rs`, `dialog_actions.rs`,
  `dialog_open.rs`, `dialog_update.rs`) — see `split-update-mod`

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
