# Centralize UI strings module

## Status

`done`

## Description

User-facing strings are scattered across handlers, update helpers, dialog builders, and tool code. Create `runie-core::ui_strings` (or similar) and move copy there.

## Acceptance criteria

- [x] Created `runie-core/src/ui_strings.rs` with centralized string constants and functions.
- [x] Updated `session/mod.rs` to use centralized session strings.
- [x] Updated `session/run.rs` to use centralized session strings.
- [x] Updated `system.rs` to use centralized system strings.
- [x] Updated `model.rs` to use centralized model strings.
- [x] Updated `registry.rs` to use centralized command parsing strings.
- [x] Updated `help.rs` to use centralized help strings.
- [x] Added module to `lib.rs`.
- [x] All tests pass.

## Changes

- Created `crates/runie-core/src/ui_strings.rs` with modules:
  - `session` - session-related strings (save, load, delete, import, export, etc.)
  - `model` - model/provider strings (no providers, model switched, etc.)
  - `system` - system command strings (copy, skills, etc.)
  - `commands` - command parsing strings (invalid syntax, unknown command)
  - `help` - help panel strings
  - `trust` - trust status strings

- Updated handlers:
  - `commands/dsl/handlers/session/mod.rs`
  - `commands/dsl/handlers/session/run.rs`
  - `commands/dsl/handlers/system.rs`
  - `commands/dsl/handlers/model.rs`
  - `commands/dsl/handlers/help.rs`
  - `commands/registry.rs`

## Tests

- `cargo test --workspace` passes.
- All user-facing strings now reference `ui_strings` module.
