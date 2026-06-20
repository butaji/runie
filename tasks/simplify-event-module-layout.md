# Simplify event module aliases and variants layout

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: simplify-event-vocabulary
**Blocks**: none

## Description

The `crates/runie-core/src/event/` module has two sources of file-count bloat not covered by `simplify-event-vocabulary` (which targets the enum nesting, not the file layout):

**(a) Five 1-to-5-line alias shim files.** Each is a single `pub type X = Y;` (or a 1-line `pub use`) re-exporting a sibling sub-enum under a backward-compatible name. Total 21 LOC across 5 files:

| File | LOC | Content |
|------|-----|---------|
| `command.rs` | 5 | `pub type CommandEvent = ControlEvent;` |
| `edit.rs` | 5 | `pub type EditEvent = IoEvent;` |
| `login_flow.rs` | 5 | `pub type LoginFlowEvent = DialogEvent;` |
| `model_config.rs` | 5 | `pub type ModelConfigEvent = ControlEvent;` |
| `dialog.rs` | 1 | `//! Re-export the flat DialogEvent alias.` |

They exist for legacy import paths. All 5 are re-exported from `lib.rs:113` as `runie_core::{CommandEvent, EditEvent, LoginFlowEvent, ModelConfigEvent, ...}`. External callers exist: `update/command.rs`, `update/tools.rs`, `update/login_flow.rs`, `update/agent/model_config.rs`, `update/dialog/form_tests.rs`. The aliases themselves are useful; the dedicated files are not.

**(b) `variants/` split across 4 files + loose `variants_tests.rs`.** `event/variants.rs` (79) declares the `Event` enum; `event/variants/{constructors,name,to_durable}.rs` (123 + 31 + 62) hold its impls; `event/variants_tests.rs` (220) sits **outside** the `variants/` dir and is wired via `mod variants_tests;` from `event/mod.rs`. The `*_tests.rs` living next to the parent module (rather than inside the dir it tests) breaks the convention used elsewhere in the crate.

## Acceptance Criteria

- [ ] The 5 alias shim files are collapsed into a single `event/aliases.rs` (or inlined at the top of `event/mod.rs` if small enough). The `pub type X = Y;` declarations and the `lib.rs` re-exports stay.
- [ ] `event/variants_tests.rs` moves into `event/variants/tests.rs` and is wired via `#[cfg(test)] mod tests;` from `event/variants/mod.rs` (or `variants.rs` if the dir is collapsed).
- [ ] Consider merging `variants/{constructors,name,to_durable}.rs` (216 LOC total) into `variants.rs` (79 LOC) → 295 LOC, under the 500-line file limit. Do this if no function exceeds the 40-line limit; otherwise leave the split.
- [ ] All `use runie_core::event::{CommandEvent, EditEvent, LoginFlowEvent, ModelConfigEvent, DialogEvent}` imports still resolve.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds (the `variants_tests` assertions stay green).

## Tests

### Layer 1 — State/Logic
- [ ] `event_aliases_resolve_after_collapse` — `runie_core::event::ModelConfigEvent` and the 4 sibling aliases still resolve to the same underlying type (compile-time check via `fn _assert() { let _: () = <CommandEvent as IsType<ControlEvent>>::ASSERT; }` or a simple assignment test).
- [ ] `event_size_test_still_passes` — the existing `event_size_reduced` test in `variants_tests.rs` still passes after the move.

### Layer 2 — Event Handling
- [ ] `command_event_routes_to_command_handler` — a `CommandEvent` still reaches `update::command::handle_command_event` after the alias collapse.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_event_module_compiles` — `cargo check -p runie-core` green after the file consolidation.

## Files touched

- `crates/runie-core/src/event/command.rs` (delete or merge)
- `crates/runie-core/src/event/edit.rs` (delete or merge)
- `crates/runie-core/src/event/login_flow.rs` (delete or merge)
- `crates/runie-core/src/event/model_config.rs` (delete or merge)
- `crates/runie-core/src/event/dialog.rs` (delete or merge)
- `crates/runie-core/src/event/aliases.rs` (new, optional)
- `crates/runie-core/src/event/mod.rs` (update `mod` declarations)
- `crates/runie-core/src/event/variants_tests.rs` → `event/variants/tests.rs`
- `crates/runie-core/src/event/variants.rs` → `event/variants/mod.rs` (if dir is created)
- `crates/runie-core/src/event/variants/{constructors,name,to_durable}.rs` (optionally merged)

## Notes

Depends on `simplify-event-vocabulary` (in_progress) so the alias collapse lands after the enum nesting stabilizes — otherwise the two refactors collide in `event/mod.rs`. The aliases themselves are not deprecated; only the dedicated shim files are. `orchestrator-event-alias-docs` tracks a separate `OrchestratorEvent` alias and is unrelated.
