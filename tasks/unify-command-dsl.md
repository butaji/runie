# Unify Command DSL

**Status**: todo
**Milestone**: R3
**Category**: Core Architecture
**Priority**: P1

**Depends on**: (none)
**Blocks**: (none)

## Description

Slash commands are defined in one DSL (`commands/dsl/`) and then executed
through a separate handler layer (`commands/handlers/`). Panel construction
for command output is repetitive, especially in
`commands/handlers/agents.rs` (441 lines). The two-layer design forces
contributors to edit both a DSL file and a handler file to add a `/foo`
command.

## Acceptance Criteria

- [ ] A command is defined in one place: its metadata, argument parsing,
  execution logic, and optional form dialog.
- [ ] The separate `commands/handlers/` directory is merged into the DSL or
  deleted.
- [ ] Common panel/palette item construction is extracted into a small set
  of helpers.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `command_registry_lists_all_commands` — every command is registered
  exactly once.
- [ ] `command_exec_returns_message` — a simple command produces the
  expected `CommandResult`.

### Layer 2 — Event Handling
- [ ] `slash_command_dispatches_to_handler` — typing `/hello` produces the
  correct `CommandEvent` and executes the unified handler.

## Files touched

- `crates/runie-core/src/commands/dsl/*.rs`
- `crates/runie-core/src/commands/handlers/*.rs`
- `crates/runie-core/src/commands/registry.rs`
- `crates/runie-core/src/commands/mod.rs`

## Notes

The DSL should remain declarative for simple commands, but the execution
path should live next to the declaration.
