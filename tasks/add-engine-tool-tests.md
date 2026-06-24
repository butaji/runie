# Add tests for `runie-engine` tool implementations

**Status**: done
**Milestone**: R4
**Category**: Tools
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-engine/src/lib.rs` is a single module file that re-exports the built-in tools (`read_file`, `write_file`, `edit_file`, `bash`, `grep`, `find`, `fetch_docs`, `list_dir`, `find_definitions`). It carries 2,466 LOC of production code and zero tests. Every test that exercises these tools lives in `runie-core/src/tests/` or `runie-agent/src/tests/`, which is the wrong layer: tool behavior (path normalization, edit semantics, bash safety, search ranking) belongs at the tool crate.

This task adds Layer 1 + Layer 2 test coverage for each tool in `runie-engine/src/tool/`, matching the four-layer testing philosophy in `AGENTS.md`.

## Acceptance Criteria

- [ ] Each tool module in `crates/runie-engine/src/tool/*.rs` has a `#[cfg(test)] mod tests` block with at least:
  - happy-path input/output (Layer 1)
  - tool input shape + JSON schema (Layer 2) — covered by `input_schema()` tests
- [ ] `runie-engine` is dev-deps-clean of any test-only dependencies it doesn't already need (no new top-level deps).
- [ ] `cargo test -p runie-engine` runs the new suite.
- [ ] No existing tests break.

## Tests

The new tests live in `crates/runie-engine/src/tool/<name>_tests.rs` (or inline `#[cfg(test)] mod tests`) per the testing philosophy.

### Layer 1 — State/Logic (per tool)
- [ ] `read_file_returns_file_contents`
- [ ] `read_file_missing_path_errors`
- [ ] `list_dir_lists_entries_sorted`
- [ ] `list_dir_missing_path_errors`
- [ ] `write_file_creates_file`
- [ ] `write_file_overwrites_existing`
- [ ] `edit_file_applies_unified_diff`
- [ ] `edit_file_rejects_malformed_diff`
- [ ] `bash_runs_command_captures_stdout_stderr`
- [ ] `bash_blocks_disallowed_command`
- [ ] `grep_finds_pattern_in_files`
- [ ] `grep_returns_empty_when_no_match`
- [ ] `find_locates_files_by_glob`
- [ ] `find_definitions_finds_symbol_in_rust_file`
- [ ] `fetch_docs_fetches_url` (mock the HTTP layer via the `fff-search`/reqwest swap that `canonicalize-tool-call` will require, or use a recorded fixture)

### Layer 2 — Event Handling
- N/A — tools are not event-driven.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- N/A — covered by `runie-agent` and `runie-testing` already.

## Files touched

- `crates/runie-engine/src/tool/mod.rs`
- `crates/runie-engine/src/tool/{read_file,list_dir,write_file,edit_file,bash,grep,find,find_definitions,fetch_docs}.rs`
- Possibly a new `crates/runie-engine/tests/` directory if the suite grows.

## Notes

- Coordinate with `introduce-tool-definition-macro` (P1): once tools are declared declaratively, the schema/round-trip tests can be a single macro-driven check instead of per-tool repetition.
- For `bash`, prefer running commands in a tempdir and stubbing the dangerous-command policy through `PermissionGate`. Do not hit the network.
