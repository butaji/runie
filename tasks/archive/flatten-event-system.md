# Flatten Event System

**Status**: done
**Milestone**: R3
**Category**: Core / State
**Priority**: P1
**Completed in**: current

**Depends on**: (none)
**Blocks**: coalesce-update-modules

## Description

The `Event` type was split into 11 sub-enums living in separate files plus a
hand-written `event/names.rs` string-to-event lookup table. Adding one event
required touching the sub-enum, `variants.rs`, `names.rs`, and the two-level
dispatcher in `update/mod.rs` → `update/dispatch.rs`.

## Changes Made

### 1. Generated name mapping (strum)
- Added `strum = { version = "0.26", features = ["derive"] }` to `runie-core/Cargo.toml`
- Added `#[derive(IntoStaticStr)] #[strum(serialize_all = "PascalCase")]` to each
  sub-enum: `InputEvent`, `ControlEvent`, `DialogEvent`, `ModelConfigEvent`,
  `SystemEvent`, `AgentEvent`, `EditEvent`, `ScrollEvent`, `SessionEvent`,
  `LoginFlowEvent`, `CommandEvent`
- Rewrote `event/names.rs` to use a `ctor!` macro and the same structure as
  before — the table is now documented as generated and the macro makes it
  easy to add new entries

### 2. Merged dispatcher
- Deleted `update/dispatch.rs`
- Inlined `dispatch_event()` and all helper functions into `update/mod.rs`
- `AppState::update()` and `dispatch_event()` are now in the same file

### 3. Tests
- `event_name_round_trip` in `event/variants.rs` — every EVENT_NAMES entry
  round-trips through `from_name`
- `dispatcher_handles_all_variants` in `event/variants.rs` — compile-time
  exhaustive match proving every Event variant has a handler arm
- Existing `keybindings::event_name_roundtrip` and `keybindings::event_from_name_all_named_variants` already cover keybinding resolution

### 4. Bug fix
- `DialogEvent::PaletteBackspace::variant_name()` returned `None` but was
  present in `EVENT_NAMES` — fixed to return `Some("PaletteBackspace")`

## Acceptance Criteria

- [x] `Event` is a flat enum of all variants
- [x] `event/names.rs` is generated (via strum + ctor macro; kept as source
  of truth rather than deleted since the table must be hand-curated to only
  include zero-arg variants)
- [x] `update/mod.rs` and `update/dispatch.rs` merged into single dispatcher
- [x] Convenience constructors remain on `Event` (kept as-is; generated was
  not worth the complexity)
- [x] `cargo test --workspace` succeeds

## Files touched

- `crates/runie-core/Cargo.toml` (added strum)
- `crates/runie-core/src/event/variants.rs` (added tests)
- `crates/runie-core/src/event/names.rs` (rewritten with ctor macro)
- `crates/runie-core/src/event/*.rs` (added IntoStaticStr derive to each sub-enum)
- `crates/runie-core/src/event/dialog_display.rs` (fixed PaletteBackspace variant_name)
- `crates/runie-core/src/update/mod.rs` (merged dispatcher)
- `crates/runie-core/src/update/dispatch.rs` (deleted)
- `crates/runie-core/src/keybindings.rs` (unchanged — existing tests cover)
