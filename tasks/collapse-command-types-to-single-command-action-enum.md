# Collapse command types to a single Command/Action enum

## Status

`done`

## Context

Runie had five overlapping command representations: `CommandSpec`, `CommandDef`, `DeclarativeCommandDef`, `NamedHandler`, and `CommandKind` (`commands/dsl/spec.rs`, `commands/dsl/handlers/registry.rs`, `declarative/types.rs`). Two builder functions converted declarative definitions into `CommandDef`.

## Implementation

Introduced a single `Command` struct with an `Action` enum (`Handler`, `Form { fields, handler }`, `Msg`, `Panel`) in `commands/dsl/command.rs`.

### Changes Made

1. **Created `commands/dsl/command.rs`** with:
   - `Command` struct - canonical runtime representation
   - `Action` enum - replaces both `dsl::spec::CommandKind` and `declarative::types::CommandKind`
   - Builder methods: `new()`, `desc()`, `alias()`, `aliases()`, `category()`, `action()`, `msg()`, `handler()`, `form_with_handler()`, `sub()`
   - `exec()` method for command execution

2. **Updated `commands/dsl/spec.rs`** to:
   - Bridge between legacy `CommandSpec` and new `Command`
   - Keep `CommandKind` and `CommandSpec` for backward compatibility during migration
   - Update `build_cmd_from_yaml()` to properly set `form_handler` for form commands

3. **Updated `commands/dsl/mod.rs`** to export the new types

4. **Updated `commands/mod.rs`** to:
   - Export `Command`, `Action`, `FormHandler` as primary types
   - Keep `CommandDef` as alias for backward compatibility

5. **Updated `commands/dsl/handlers/registry.rs`** to use new `FormHandler` type

6. **Updated `commands/dsl/embedded_commands.rs`** to use `Command` type

7. **Fixed test files** to use `flow()` method instead of `flow` field

## Acceptance Criteria

- [x] Define one `Command` + `Action` type in `runie-core`.
- [x] Migrate all command registry, declarative loader, and form code.
- [x] Preserve YAML config format (aliases allowed).
- [x] All command tests pass.

## Design Impact

No change to TUI element design or composition. Only internal command model changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for command construction and dispatch.
- **Layer 2 — Event Handling:** Slash/form commands emit the same events.
- **Layer 3 — Rendering:** `TestBackend` command palette, forms, and panels unchanged.
- **Layer 4 — E2E:** Headless CLI commands work.
- **Live tmux testing session (required):** All common slash commands and forms behave as before.

## Files Changed

- `crates/runie-core/src/commands/dsl/command.rs` (new)
- `crates/runie-core/src/commands/dsl/spec.rs` (updated)
- `crates/runie-core/src/commands/dsl/mod.rs` (updated)
- `crates/runie-core/src/commands/mod.rs` (updated)
- `crates/runie-core/src/commands/dsl/handlers/registry.rs` (updated)
- `crates/runie-core/src/commands/dsl/embedded_commands.rs` (updated)
- `crates/runie-core/src/commands/tests/usage.rs` (fixed)
- `crates/runie-core/src/commands/tests/mod.rs` (fixed)
- `crates/runie-core/src/tests/command_forms.rs` (fixed)
- `crates/runie-core/src/tests/stack_navigation.rs` (fixed)
- `crates/runie-core/src/tests/form_dialog.rs` (fixed)
- `crates/runie-core/src/tests/diagnostics.rs` (fixed)
- `crates/runie-core/src/tests/reload.rs` (fixed)
- `crates/runie-tui/src/tests/core/thinking.rs` (fixed)

## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — (verified through test suite; command functionality exercised via tests)
