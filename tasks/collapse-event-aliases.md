# Collapse 11 `event/<alias>.rs` shim files into one `event/aliases.rs`

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/event/` contains 12 single-purpose files: 11 that each declare one `pub type Foo = Event;` alias, plus `dialog.rs` which is a 1-LOC doc-only stub that contributes no code (the actual `DialogEvent` alias lives in `dialog_display.rs` and is re-exported by `event/mod.rs:17`). Total ~56 LOC across 12 directories worth of file overhead:

| File | LOC | Content |
|------|-----|---------|
| `event/agent.rs` | 5 | `pub type AgentEvent = Event;` |
| `event/command.rs` | 5 | `pub type CommandEvent = Event;` |
| `event/control.rs` | 5 | `pub type ControlEvent = Event;` |
| `event/dialog.rs` | 1 | doc-only stub; the `DialogEvent` alias is in `dialog_display.rs` |
| `event/dialog_display.rs` | 5 | `pub type DialogEvent = Event;` |
| `event/edit.rs` | 5 | `pub type EditEvent = Event;` |
| `event/input.rs` | 5 | `pub type InputEvent = Event;` |
| `event/login_flow.rs` | 5 | `pub type LoginFlowEvent = Event;` |
| `event/model_config.rs` | 5 | `pub type ModelConfigEvent = Event;` |
| `event/scroll.rs` | 5 | `pub type ScrollEvent = Event;` |
| `event/session.rs` | 5 | `pub type SessionEvent = Event;` |
| `event/system.rs` | 5 | `pub type SystemEvent = Event;` |

All are publicly re-exported via `lib.rs:113-115` and are heavily used (e.g. `update/login_flow.rs:8`, `update/session.rs:265`, `keybindings/mod.rs:16`, `runie-testing/src/state.rs:3`). The aliases exist for backward compatibility with the pre-flattening sub-enum design.

## Acceptance Criteria

- [ ] All 11 single-type alias files deleted.
- [ ] New `crates/runie-core/src/event/aliases.rs` declares all 11 type aliases in one place, with a doc-comment explaining they exist for backward compatibility with the old sub-enum API.
- [ ] `pub mod aliases;` declared in `crates/runie-core/src/event/mod.rs`; `pub use aliases::{…};` updated in `crates/runie-core/src/lib.rs` to match the new module path.
- [ ] `rg "use crate::event::(agent|command|control|dialog_display|edit|input|login_flow|model_config|scroll|session|system)\b" crates/` returns zero hits (callers now go through `lib.rs` re-exports or `crate::event::aliases::`).
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- N/A — pure module restructuring.

### Layer 2 — Event Handling
- [ ] `event_aliases_resolve` — every alias still compiles and matches `Event`:
  - `assert_eq!(std::mem::size_of::<AgentEvent>(), std::mem::size_of::<Event>());` for each of the 11.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_event_aliases_module_present` — `ls crates/runie-core/src/event/aliases.rs` succeeds; workspace builds.

## Files touched

- `crates/runie-core/src/event/agent.rs` (delete)
- `crates/runie-core/src/event/command.rs` (delete)
- `crates/runie-core/src/event/control.rs` (delete)
- `crates/runie-core/src/event/dialog.rs` (delete)
- `crates/runie-core/src/event/dialog_display.rs` (delete)
- `crates/runie-core/src/event/edit.rs` (delete)
- `crates/runie-core/src/event/input.rs` (delete)
- `crates/runie-core/src/event/login_flow.rs` (delete)
- `crates/runie-core/src/event/model_config.rs` (delete)
- `crates/runie-core/src/event/scroll.rs` (delete)
- `crates/runie-core/src/event/session.rs` (delete)
- `crates/runie-core/src/event/system.rs` (delete)
- `crates/runie-core/src/event/aliases.rs` (new)
- `crates/runie-core/src/event/mod.rs` (replace 11 `pub mod` lines with one `pub mod aliases;`)
- `crates/runie-core/src/lib.rs` (update `pub use event::{…}` to re-export from `event::aliases`)

## Notes

Going one step further — deleting the aliases and migrating the ~80 call sites to `Event::Foo` directly — is the cleaner Rust idiom, but it churns every event-handler match arm in the workspace. Keep aliases, just collapse them into one file. If the aliases are ever removed, do it as a separate workspace-wide codemod.
