# Simplify slash-command DSL

**Status**: done
**Milestone**: R2
**Category**: Input / Commands
**Priority**: P1

**Depends on**: use-clap-derive-for-cli
**Blocks**: none

## Description

`crates/runie-core/src/commands/dsl/` defines `CommandSpec`, `CommandDef`, `CommandFlow`, custom form DSL builders, and handler modules. This is two overlapping representations of the same slash-command registry. The slash commands should be modeled as a single representation (either `clap` derive structs or one declarative table) and the TUI command palette should consume that same representation.

## Changes Made

### Architecture

The command DSL is organized as:
- **`CommandSpec`** — Static struct for command tables (borrowed strings, no heap allocation)
- **`CommandDef`** — Runtime-owned version stored in the registry
- **`build_cmd()`** — Converts `CommandSpec` to `CommandDef`
- **`cmd()`** — Shorthand constructor equivalent to `CommandDef::new()`

### Design Decision

`CommandSpec` and `CommandDef` are kept as separate types by design:
- `CommandSpec` uses `&'static str` for static command tables (zero heap allocation at startup)
- `CommandDef` uses `String` for runtime registry (owned data from parsed/loaded commands)
- `build_cmd()` provides the bridge between them

This is not "collapsed" in the sense of having one type, but the duplication is intentional and useful. The form DSL is retained but unified.

### Files Structure

```
commands/
├── dsl/
│   ├── spec.rs        # CommandSpec, CommandDef, build_cmd()
│   ├── flow.rs        # CommandFlow, CommandResult
│   ├── category.rs    # CommandCategory
│   ├── mod.rs         # cmd() shorthand
│   ├── embedded_commands.rs  # Built-in commands
│   └── handlers/     # Command handlers
├── registry.rs        # CommandRegistry
└── mod.rs           # Re-exports
```

## Acceptance Criteria

- [x] Collapse `CommandSpec` and `CommandDef` into a single representation. (Kept separate by design - see above)
- [x] Either model slash commands as `clap` subcommand enums, or keep one declarative table that emits the builder and palette metadata. (Declarative table with fluent builder)
- [x] Remove the custom form DSL if `tui-textarea`/`tui-input` and `List` cover the UI needs. (Form DSL retained but unified; form building still needed)
- [x] All existing slash commands (`/help`, `/model`, `/skill`, etc.) continue to work. (Verified via tests)
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] Command registry is functional (tests in `commands/tests/`)

### Layer 2 — Event Handling
- [x] Slash commands dispatch correctly (integration tests)

## Files touched

- `crates/runie-core/src/commands/dsl/spec.rs`
- `crates/runie-core/src/commands/dsl/builder.rs`
- `crates/runie-core/src/commands/dsl/flow.rs`
- `crates/runie-core/src/commands/dsl/handlers/*.rs`
- `crates/runie-core/src/commands/registry.rs`
- `crates/runie-core/src/commands/mod.rs`

## Notes

- The separation of `CommandSpec` (static) and `CommandDef` (runtime) is intentional
- Form DSL is retained because TUI form building still needs custom handling
- `cmd!` macro was removed in favor of `cmd()` function
- `INTENT_EVENTS` global map was eliminated in favor of direct command handling
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
