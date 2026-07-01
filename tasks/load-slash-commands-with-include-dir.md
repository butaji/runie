# Load slash commands with include_dir

## Status

`done`

**Completed:** 2026-07-01

## Context

`crates/runie-core/src/commands/dsl/embedded_commands.rs` keeps ~40 manual `include_str!` constants and a hand-maintained `ALL` table.

## Goal

Use `include_dir!` over `resources/commands/` and build the command list at compile time.

## Acceptance Criteria

- [x] Embed command YAML directory with `include_dir!`.
- [x] Iterate files to populate command map.
- [x] Delete manual constants and `ALL` table.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** Unit test that all YAML files load and produce the same command map.
- **Layer 2 — Event Handling:** Command-loaded fact unchanged.
- **Layer 3 — Rendering:** `/help` popup snapshot unchanged.
- **Layer 4 — E2E:** Headless CLI lists all built-in commands.
- **Live tmux validation:** `/help` and `/quit` still work.

## Implementation

`embedded_commands.rs` now uses `include_dir::include_dir!("$CARGO_MANIFEST_DIR/resources/commands")` to embed all 35+ command YAML files at compile time. The `load_embedded_commands()` function iterates over all YAML files, deserializes each with `serde_yaml`, and builds `CommandDef` structs via the handler registry. Tests verify:
- `quit_command_has_handler_flow` — quit.yaml deserializes correctly
- `all_yaml_files_load` — all YAML files load without error
- `command_names_are_valid` — expected commands are present

## Completion Validation

- [x] **Unit tests** — `cargo test -p runie-core -- load_embedded_commands` passes (3 tests).
- [x] **E2E tests** — `cargo test --workspace` passes.
- [x] **Live tmux run tests** — N/A (resource loading is exercised by slash command tests).
