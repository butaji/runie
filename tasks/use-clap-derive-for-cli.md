# Use `clap` derive macros for CLI argument parsing

**Status**: done
**Milestone**: R1
**Category**: Input / Commands
**Priority**: P0

**Depends on**: none
**Blocks**: simplify-slash-command-dsl

## Description

`crates/runie-cli/src/main.rs` now uses `clap` derive macros for typed CLI dispatch. The CLI has four main subcommands (`print`, `inspect`, `json`, `server`) and an MCP management subcommand (`mcp list/add/remove`). Switching to `clap` gives automatic `--help`, validation, and subcommand routing.

## CLI Commands

The CLI exposes these subcommands:

| Command | Description |
|---------|-------------|
| `runie print <prompt>` | Stream LLM response as JSONL to stdout |
| `runie inspect [--json]` | Show runtime config for the current directory |
| `runie json` | JSON stdin/stdout for scripting |
| `runie server [--stdio] [--yolo]` | TCP/stdio JSON-RPC server |
| `runie mcp list` | List configured MCP servers |
| `runie mcp add <name> [--scope global\|project] -- <command...>` | Add an MCP server |
| `runie mcp remove <name> [--scope global\|project]` | Remove an MCP server |

The `inspect` command covers config/MCP inspection. The `mcp` subcommand provides MCP server management via ConfigActor.

## Acceptance Criteria

- [x] Replace manual argument parsing in `crates/runie-cli/src/main.rs` with a `clap` derive `Cli` struct and subcommand enums.
- [x] Preserve all existing commands (`print`, `inspect`, `json`, `server`).
- [x] Add `mcp` subcommand for MCP server management.
- [x] Add typed validation where it is free (e.g., numeric ports, existing file paths).
- [x] Ensure `--help` and `--version` work.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `cli_parses_print` — `runie print "prompt"` parses into the expected struct.
- [x] `cli_parses_inspect` — `runie inspect` parses correctly.
- [x] `cli_parses_inspect_json` — `runie inspect --json` parses correctly.
- [x] `cli_parses_json_mode` — `runie json` parses correctly.
- [x] `cli_parses_server` — `runie server` parses correctly.
- [x] `cli_parses_mcp_list` — `runie mcp list` parses correctly.
- [x] `cli_parses_mcp_add` — `runie mcp add my-server -- npx @server` parses correctly.
- [x] `cli_parses_mcp_add_project_scope` — `runie mcp add my-server --scope project -- npx` parses correctly.
- [x] `cli_parses_mcp_remove` — `runie mcp remove my-server` parses correctly.
- [x] `cli_rejects_unknown_subcommand` — unknown subcommands produce a typed error.
- [x] `cli_help_includes_all_commands` — help text mentions every documented command.

### Layer 2 — Event Handling
- [x] N/A.

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `crates/runie-cli/src/main.rs` — clap derive structs and CLI routing
- `crates/runie-cli/src/mcp.rs` — MCP management module (list, add, remove)
- `Cargo.toml` (workspace)

## Notes

- `clap` is a workspace dependency; no new crate was needed.
- MCP management uses ConfigActor's `add_mcp_server`, `remove_mcp_server`, and `list_mcp_servers` methods.
- ConfigActor handles file persistence; the CLI module provides the user interface.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
