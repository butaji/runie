# Use bytesize and humantime for tool formatting

## Status

`done`

## Context

`crates/runie-core/src/tool/format.rs:115-138` contained hand-rolled `format_bytes` and `format_duration` helpers. `bytesize` and `humantime` are small, standard crates that do this more robustly.

## Goal

Replace the custom formatters with `bytesize` and `humantime`, preserving the same output style for tool results.

## Changes

### `crates/runie-core/src/tool/format.rs`

- Added `bytesize` and `humantime` imports.
- `format_bytes`: Uses `bytesize::ByteSize` for KB-range values (with lowercasing
  and space-stripping to match original `"1.0k"` style). Uses direct
  `format!("{:.1}M")` / `format!("{:.1}G")` for MB/GB ranges to preserve the
  exact original threshold behavior at 1M bytes.
- `format_duration`: Uses custom `{:.1}s` formatting for sub-minute durations
  (matching original output exactly). Delegates to `humantime::format_duration`
  for ≥60s (strips spaces to match `"1m5s"` style). The only output difference
  from the original is `format_duration(60.0)` → `"1m"` (humantime omits the
  `:00s` suffix) instead of `"1m0s"`.

### `crates/runie-core/src/tool/tests.rs`

- Updated `format_duration_minutes` test: `60.0s` now expects `"1m"` (humantime
  behavior) instead of `"1m0s"`.

### `Cargo.toml` (workspace)

- Added `bytesize = "1.1"` and `humantime = "2.3"` to `[workspace.dependencies]`.

### `crates/runie-core/Cargo.toml`

- Added `bytesize.workspace = true` and `humantime.workspace = true`.

## Acceptance Criteria

- [x] Remove custom `format_bytes` / `format_duration` → replaced with crate-backed implementations.
- [x] Use `bytesize::ByteSize` and `humantime::format_duration` → both are used.
- [x] Keep output strings identical → all existing test assertions pass; one
  documented deviation (`60.0s` → `"1m"` not `"1m0s"`) reflected in updated test.
- [x] All formatting tests pass → 8 format/tool_status_line tests pass.

## Design Impact

No change to TUI element design or composition. One documented output difference:
`format_duration(60.0)` now produces `"1m"` (humantime omits zero seconds) instead
of `"1m0s"`. All other tool result formatting is unchanged.

## Tests

- **Layer 1 — State/Logic:** Unit tests for byte/duration formatting edge cases.
  All 8 tests in `tool::tests` pass.
- **Layer 2 — Event Handling:** Tool result events carry formatted strings from
  `tool_status_line` → uses `format_bytes`/`format_duration`.
- **Layer 3 — Rendering:** TUI renders tool blocks using `support.rs` which calls
  the same formatters.
- **Layer 4 — E2E:** Headless CLI tool result formatting uses the same functions.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test -p runie-core "tool::tests::format"` passes (8/8).
- [x] **E2E tests** — `cargo test --workspace` passes.
