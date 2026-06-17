# WriteFileTool Parent Directory Error Handling

**Status**: todo
**Milestone**: R3
**Category**: Tools
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

`WriteFileTool::ensure_parent_dirs` returns `Ok(ToolOutput { status: Success, ... })` on success and `Ok(output_error(...))` on `create_dir_all` failure. The caller does `ensure_parent_dirs(...)?;`, which unwraps the `Ok` and discards the returned `ToolOutput`, then proceeds to `write_and_return(...)`. If creating parent directories fails, the error is lost and a write is still attempted.

## Acceptance Criteria

- [ ] `ensure_parent_dirs` returns a plain `Result<(), ToolOutput>` or `Result<()>`.
- [ ] Directory-creation failure aborts the write and surfaces the error to the caller/LLM.
- [ ] Success still proceeds to `write_and_return`.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `write_file_fails_when_parent_dir_creation_fails` — mock or restrict perms to force `create_dir_all` failure and assert error output.
- [ ] `write_file_succeeds_when_parent_dirs_exist` — existing path still writes.

### Layer 2 — Event Handling
N/A — pure tool logic.

### Layer 3 — Rendering
N/A — covered by generic tool error rendering.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-engine/src/tool/write_file.rs`

## Notes

This is a silent failure bug: the LLM believes the write succeeded because the error output is discarded.
