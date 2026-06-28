# Replace custom build.rs linter with Clippy / CI

**Status**: todo
**Milestone**: R3
**Category**: Architecture / Tooling
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/build.rs` (432 LOC) and `crates/runie-core/src/build_lint.rs` (122 LOC) implement a hand-rolled Rust tokenizer, brace tracker, function-length counter, and complexity heuristic, plus an allow-list for exempt files. Most of these checks can be expressed as Clippy lints or a tiny CI script. Replacing the custom linter removes ~554 lines and a compile-time dependency on brittle string scanning.

## Acceptance Criteria

- [ ] Replace the function-length and complexity heuristics with `[workspace.lints.clippy]` entries (e.g., `clippy::cognitive_complexity`, `clippy::too_many_lines`, `clippy::too_many_arguments`) or a curated lint set.
- [ ] Replace the 500-line file limit with a short CI script using `find` + `wc` (or a `cargo xtask`).
- [ ] Remove `crates/runie-core/build.rs` linter code and `crates/runie-core/src/build_lint.rs`.
- [ ] Keep the build script only if it has non-lint responsibilities (e.g., embedding resources); otherwise delete the file.
- [ ] Tune lint levels so existing code passes without a massive refactor; file follow-up tasks for any true positives.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `ci_file_limit_passes` — a CI/script check confirms all production `.rs` files are ≤500 lines.
- [ ] `clippy_lints_catch_long_function` — a deliberately long test fixture function triggers the configured Clippy lint.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/build.rs`
- `crates/runie-core/src/build_lint.rs`
- `Cargo.toml` (workspace lints)
- `.github/workflows/` or `scripts/` (CI file-limit check)

## Notes

- `clippy::cognitive_complexity` is deprecated in some versions; use the equivalent lint or a `dylint` plugin if precise complexity counting is required.
- This task requires CI/workflow changes; it is not a pure code change.
- Rejected: keep the custom linter for exact control — the maintenance cost exceeds the precision benefit.
