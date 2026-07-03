# Drop event/aliases.rs type aliases

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/event/aliases.rs` keeps four type aliases that all resolve to a canonical sub-enum:

```rust
pub use super::control::ControlEvent as CommandEvent;
pub use super::control::ControlEvent as ModelConfigEvent;
pub use super::dialog_display::DialogEvent as LoginFlowEvent;
pub use super::io::IoEvent as EditEvent;
```

Used in ~5 update files (`update/command.rs`, `update/agent/model_config.rs`, `update/login_flow.rs`, `update/tools.rs`, plus test imports). The aliases hide that "command", "model config", and "control" are the same `ControlEvent` enum, and that "edit" is `IoEvent` — readers have to chase the alias to know which variants are available. `orchestrator-event-alias-docs` (done) added a doc comment to a different alias; this task reopens whether the four event-category aliases should exist at all.

Either (a) drop the aliases and rename call sites to the canonical name, or (b) keep and document a concrete reason the aliases earn their indirection (e.g. migration stability for downstream crates — but no external consumer imports them).

## Acceptance Criteria

- [ ] Decision made: EITHER
  - (a) **Drop** — `aliases.rs` deleted; `pub use aliases::{...}` removed from `event/mod.rs`; the four aliases removed from `runie-core/src/lib.rs` exports; all call sites rewritten to the canonical name:
    - `CommandEvent` → `ControlEvent` (in `update/command.rs` and test imports)
    - `ModelConfigEvent` → `ControlEvent` (in `update/agent/model_config.rs`)
    - `LoginFlowEvent` → `DialogEvent` (in `update/login_flow.rs`)
    - `EditEvent` → `IoEvent` (in `update/tools.rs`, `commands/dsl/handlers/system.rs`, and test imports); OR
  - (b) **Keep + document** — a concrete reason is written into `aliases.rs` module docs (not "migration" unless a real downstream consumer is named).
- [ ] If (a): `rg "CommandEvent|EditEvent|LoginFlowEvent|ModelConfigEvent" crates/` returns zero hits outside `event/aliases.rs` (which is deleted) and the `lib.rs` re-exports (removed).
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `canonical_event_names_compile` — the four update modules compile against `ControlEvent` / `DialogEvent` / `IoEvent` directly.

### Layer 2 — Event Handling
- [ ] `command_event_handler_still_routes` — `handle_command_event(state, ControlEvent::RunLoadCommand { name })` still loads a session (existing test stays green).
- [ ] `login_flow_event_handler_still_routes` — `login_flow_event(state, DialogEvent::Start)` still starts the flow.
- [ ] `edit_event_handler_still_routes` — `update::tools::update(state, IoEvent::ApproveEdit)` still approves.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_event_dispatch_unchanged` — the full `AppState::update` dispatcher still routes every event category after the rename.

## Files touched

- `crates/runie-core/src/event/aliases.rs` (deleted if option a)
- `crates/runie-core/src/event/mod.rs` (remove `pub use aliases::{...}`)
- `crates/runie-core/src/lib.rs` (remove `CommandEvent, EditEvent, LoginFlowEvent, ModelConfigEvent` from the re-export)
- `crates/runie-core/src/update/command.rs`
- `crates/runie-core/src/update/agent/model_config.rs`
- `crates/runie-core/src/update/login_flow.rs`
- `crates/runie-core/src/update/tools.rs`
- `crates/runie-core/src/commands/dsl/handlers/system.rs`
- `crates/runie-core/src/tests/` (imports of the four aliases)

## Notes

The aliases were created when `Event` was flattened and then re-nested (see done `flatten-event-enum` and the re-nesting in `event/variants/mod.rs`). They may have been a transition aid. If the re-nested enum is now stable, the transition aid can go. If option (b), link justification and close as `wontfix`.
