# Use `clap` derive macros for CLI argument parsing

**Status**: todo
**Milestone**: R1
**Category**: Input / Commands
**Priority**: P0

**Depends on**: none
**Blocks**: simplify-slash-command-dsl

## Description

`crates/runie-cli/src/main.rs` currently parses arguments by hand (`match args[1].as_str()`). `goose`, `jcode`, and `openfang` all use `clap` derive macros for typed CLI dispatch. Switching to `clap` removes the manual parser, gives automatic `--help`, validation, and subcommand routing, and makes it trivial to wire CLI flags through `RactorConfigActor` later.

## Acceptance Criteria

- [ ] Replace manual argument parsing in `crates/runie-cli/src/main.rs` with a `clap` derive `Cli` struct and subcommand enums.
- [ ] Preserve all existing commands and flags (`run`, `inspect`, `config`, `mcp`, `--provider`, `--model`, etc.).
- [ ] Add typed validation where it is free (e.g., numeric ports, existing file paths).
- [ ] Ensure `--help` and `--version` work.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `cli_parses_run_subcommand` — `runie run "prompt" --provider x --model y` parses into the expected struct.
- [ ] `cli_rejects_unknown_subcommand` — unknown subcommands produce a typed error.
- [ ] `cli_help_includes_all_commands` — help text mentions every documented command.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-cli/src/main.rs`
- `crates/runie-cli/Cargo.toml`
- `crates/runie-cli/src/lib.rs` (if commands are exposed)

## Notes

- `clap` is already a workspace dependency; no new crate is needed.
- Coordinate with `route-cli-config-through-configactor.md`: CLI commands should eventually call `RactorConfigActor` rather than touching files directly.
- Rejected: keep manual parsing to avoid a derive macro — the line reduction and UX gain is immediate.
