# Deduplicate dialog::Panel and dialog::dsl::Panel

**Status**: todo
**Milestone**: R1
**Category**: Core Architecture
**Priority**: P0
**Depends on**: resolve-merge-conflicts

## Description

Two structs named `Panel` coexist:

- `crates/runie-core/src/dialog/panel.rs:15` — 497 lines, the "core" panel
- `crates/runie-core/src/dialog/dsl/panel.rs:9` — 413 lines, the "DSL" panel

Both have the same fields (`id`, `title`, `items`, `selected`, `filter`,
`filterable`, `keep_open_on_activate`, `form_values`, `view`), the same
~80% of methods, and are re-exported under different module paths
(`crate::dialog::Panel` vs `crate::dialog::dsl::Panel`). Code that needs
to convert between them uses awkward aliases
(`commands/dsl/builder.rs:4` `use crate::dialog::dsl::Panel as DslPanel;`,
`commands/dsl/flow.rs:3` `use crate::dialog::{Panel as CorePanel, ...}`).

The DSL abstraction was meant to provide a fluent builder on top of the
core type, but in practice it's a near-clone. The only piece of
"translation" between the two is `FormBuilder::from_dsl_panel(&panel)`
in `commands/dsl/builder.rs:265`, which extracts `FormField` items.

## Acceptance Criteria

- [ ] `crates/runie-core/src/dialog/dsl/panel.rs` is deleted (or emptied of the `Panel` struct — keep the `panel()`, `list()`, `form()` constructors)
- [ ] `crate::dialog::dsl::Panel` no longer exists; all call sites use `crate::dialog::Panel`
- [ ] The `panel()`, `list()`, `form()` constructors in `dialog/dsl/mod.rs` are preserved (they're the public DSL surface) but they return `crate::dialog::Panel` values
- [ ] `FormBuilder::from_dsl_panel` is simplified: with one `Panel` type, the conversion is just a field-by-field read of `FormField` items, or the method is removed entirely if the call site is the only consumer
- [ ] `commands/dsl/builder.rs:4` and `commands/dsl/flow.rs:3` no longer have alias imports
- [ ] `dialog/dsl/conversions.rs` `FromStringExt` / `FromEventExt` traits are removed (they have zero call sites; the blanket `impl From<Event> for ItemAction` stays)
- [ ] `dialog/dsl/form.rs` is removed (its `form()` constructor is folded into the surviving `dialog/dsl/mod.rs` constructors and returns a `dialog::Panel`)
- [ ] `dialog/dsl/mod.rs` shrinks to a thin re-export module
- [ ] The number of `pub struct Panel` declarations in the `runie-core` crate is exactly 1

## Tests

### Layer 1 — State/Logic
- [ ] `cargo build --workspace` succeeds after the consolidation
- [ ] All `Panel` test cases still pass: `cargo test -p runie-core --lib dialog::`
- [ ] `cargo test -p runie-core --lib commands::dsl::` passes (commands tests construct panels via the DSL)

### Layer 2 — Event Handling
- [ ] `cargo test -p runie-core --lib update::dialog` passes (dialog event handlers receive `PanelStack<dialog::Panel>` only)
- [ ] `cargo test -p runie-core --lib update::login_flow` passes (login flow panel builders produce the consolidated type)

### Layer 4 — Smoke
- [ ] `tmux_login_logout_test.sh` end-to-end passes (login flow renders the provider picker / key input / model selector — all panels)

## Notes

**Why this is safe to do now:** the two `Panel` types have ~95% identical
behavior. A careful diff shows the differences are:
- DSL `Panel` has `action()`, `item()`, `section()`, `group()` builder methods (the core uses `item()` only)
- Core `Panel` has `form_field()`, `form_field_value()`, `form_submit()` (the DSL has `field()`, `field_value()`)
- DSL `Panel` has `sep()`; core has `separator()`

**Migration: keep the core `Panel` and merge in the DSL-only methods.**

The core `Panel` is the survivor because:
1. It's the one that ends up in `PanelStack` (used by `DialogState`)
2. It's the one `update/mod.rs` operates on
3. It has `form_field` / `form_submit` which the login flow uses

The DSL `Panel` adds the following methods that the core lacks:
- `action(label, action) -> Self` — duplicates core's `item(label, action)`; merge
- `item(action) -> Self` — auto-generates label from `ItemAction::default_label()`; add to core
- `section(header, f)` / `group(f)` — convenience builders; add to core
- `sep()` — alias for `separator()`; add as a thin alias
- `field()`, `field_value()` — already exist in core as `form_field()`, `form_field_value()`; route the DSL `field()` calls to the core method

After the merge, the DSL `panel()` / `list()` / `form()` constructors in
`dialog/dsl/mod.rs` are the *only* remaining DSL — they return
`dialog::Panel` values. The `dsl::Panel` struct is gone.

**Out of scope:**
- Removing `dialog/flow/*` (the `Flow`/`Step`/`push`/`pop`/`close` state machine, also unused — see `clean-dead-modules`)
- The form panel view vs list panel view split (`PanelView::Form` vs `List`) — this is correct as-is
- Renaming `PanelItem` variants

**Verification:**
```bash
# Should print 1
git grep -c "^pub struct Panel " -- 'crates/runie-core/src/'

# No more aliases
git grep -nE 'Panel as (DslPanel|CorePanel)' -- 'crates/'

# Build + tests clean
cargo build --workspace
cargo test -p runie-core --lib
```
