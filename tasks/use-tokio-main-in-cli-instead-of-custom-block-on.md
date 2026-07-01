# Use tokio::main in CLI instead of custom block_on

## Status

`done`

## Context

`crates/runie-cli/src/main.rs:101-119` built a new `current_thread` tokio runtime for each async subcommand.

## Changes

- `main.rs`: Replaced `fn main()` with `#[tokio::main(flavor = "multi_thread")] async fn main()`. Removed custom `block_on` helper. All subcommands (`run_print`, `run_inspect`, `run_json`, `run_server`, `run_mcp`) are now `async fn` and awaited directly.
- `print.rs`: Made `run()` async and removed internal runtime creation.
- `inspect/mod.rs`: Made `run()` async and removed internal runtime creation.

## Acceptance Criteria
- [x] Remove custom `block_on` helper.
- [x] Update async subcommand entry points.
- [x] `cargo check -p runie-cli` passes.
- [x] `cargo test -p runie-cli` passes (23 tests).

## Tests

- **Layer 4 — E2E:** All 23 CLI tests pass.
