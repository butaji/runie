# Simplify slash-command DSL

**Status**: todo
**Milestone**: R2
**Category**: Input / Commands
**Priority**: P1

**Depends on**: use-clap-derive-for-cli
**Blocks**: none

## Description

`crates/runie-core/src/commands/dsl/` defines `CommandSpec`, `CommandDef`, `CommandFlow`, custom form DSL builders, and handler modules. This is two overlapping representations of the same slash-command registry. `clap` subcommands already provide typed dispatch, validation, and help text. The slash commands should be modeled as a single representation (either `clap` derive structs or one declarative table) and the TUI command palette should consume that same representation.

## Acceptance Criteria

- [x] Collapse `CommandSpec` and `CommandDef` into a single representation.
- [x] Either model slash commands as `clap` subcommand enums, or keep one declarative table that emits the builder and palette metadata.
- [x] Remove the custom form DSL if `tui-textarea`/`tui-input` and `List` cover the UI needs. *(Form DSL retained but unified; form building still needed)*
- [x] All existing slash commands (`/help`, `/model`, `/skill`, etc.) continue to work.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `command_registry_has_single_representation` — only one command descriptor type remains.
- [ ] `all_slash_commands_registered` — every existing command is reachable through the new registry.

### Layer 2 — Event Handling
- [ ] `slash_command_parses_typed_args` — `/model claude-sonnet` parses into the expected command value.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/commands/dsl/spec.rs`
- `crates/runie-core/src/commands/dsl/builder.rs`
- `crates/runie-core/src/commands/dsl/flow.rs`
- `crates/runie-core/src/commands/dsl/handlers/*.rs`
- `crates/runie-core/src/commands/registry.rs`
- `crates/runie-core/src/commands/mod.rs`

## Notes

- Depends on `use-clap-derive-for-cli.md` because the CLI and slash-command models should converge.
- The TUI command palette needs display names, descriptions, and aliases; ensure the new representation can produce them.
- Remove the redundant `cmd!` macro in `commands/dsl/mod.rs` and migrate call sites to the existing `commands::dsl::cmd(...)` function.
- Eliminate the leaked global intent map (`INTENT_EVENTS` in `declarative/register.rs`) used to map declarative commands to intent events.
- Unify the two `CommandCategory` enums (`commands/dsl/category.rs` and `declarative/types.rs`).
- Rejected: keep both `CommandSpec` and `CommandDef` for “flexibility” — the duplication is the main source of complexity.
