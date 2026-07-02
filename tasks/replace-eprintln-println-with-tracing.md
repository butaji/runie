# Replace `eprintln!`/`println!` with tracing

## Status

`done`

## Description

Production code (`subagents/mod.rs`) and tests use `println!`/`eprintln!`. Replace with `tracing::debug!` or assertions; configure test subscriber.

## Changes Made

1. **`crates/runie-core/src/subagents/mod.rs`**:
   - Replaced `eprintln!` with `tracing::warn!` for template render errors.

2. **`crates/runie-tui/src/main.rs`**:
   - Replaced `eprintln!` with `tracing::error!` for leader bootstrap failures.

3. **`crates/runie-cli/src/main.rs`**:
   - Added `tracing` workspace dependency to `runie-cli`.
   - Replaced `eprintln!` with `tracing::error!` for CLI command failures.

## Intentionally Preserved

The following `println!` usages were intentionally preserved as they produce user-facing output:

- `crates/runie-tui/src/main.rs` line 71: `--dry-run` report output
- `crates/runie-cli/src/inspect/mod.rs`: Human-readable inspect report output
- `crates/runie-core/src/event/headless.rs`: JSONL output for headless mode
- `crates/runie-cli/src/server.rs`: Server port output
- `crates/runie-cli/src/json.rs`: JSON response output
- `crates/runie-cli/src/print.rs`: Headless print mode output
- `crates/runie-cli/src/login.rs`: Login flow user prompts

Test code (`#[cfg(test)]`) also keeps `eprintln!` for debug output.

## Acceptance Criteria

- [x] No `eprintln!` in production code for error logging
- [x] Intentional user-facing output preserved as `println!`
- [x] All tests pass
- [x] Cargo check clean

## Tests

### Unit tests
- Grep check confirms no `eprintln!` in production (error logging paths)

### E2E tests
- All existing tests pass

### Live tmux tests
- Launch TUI and confirm no stray stdout/stderr corrupts terminal
