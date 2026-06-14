# Restore File and Function Size Discipline

**Status**: done
**Milestone**: R3
**Category**: Core Architecture
**Priority**: P2

## Description

`AGENTS.md` states:

- File max: 500 lines
- Function max: 40 lines, 10 complexity

After the big refactor several core files exceed these limits and the
active build script (`crates/runie-core/build.rs`) was softened to
2000/150/30. The workspace-root `build.rs` is dead and still references
archived files in its allow-lists.

Files currently over 500 lines (non-test):

- `crates/runie-core/src/model.rs` (750)
- `crates/runie-core/src/login_flow.rs` (908)
- `crates/runie-core/src/update/input.rs` (653)
- `crates/runie-term/src/main.rs` (510)

Test files over 500 lines are acceptable only if covered by an explicit
allow-list; `crates/runie-term/tests/e2e_legacy.rs` (1215) should be
split into the existing `tests/e2e/` modules.

## Acceptance Criteria

- [x] The workspace-root `build.rs` is already absent.
- [x] `crates/runie-core/build.rs` thresholds are restored to 500/40/10;
  `AGENTS.md`/`README.md` were updated to document both the active
  guardrails and the long-term targets.
- [x] Every non-test `.rs` file is ≤ 500 lines (verified by the build script).
- [x] Every function is ≤ 40 lines and complexity ≤ 10 (verified by the build script).
- [x] `crates/runie-term/tests/e2e_legacy.rs` is split and removed
  (tests moved to `tests/e2e/` modules).
- [x] `cargo build --workspace` succeeds.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] The build-script lint passes without allow-listing current files.

### Layer 2 — Event Handling
- [x] `cargo test -p runie-core --lib` passes.

### Layer 3 — Rendering
- [x] `cargo test -p runie-tui --lib` passes.

### Layer 4 — Smoke
- [x] `cargo test -p runie-term --test e2e -- --ignored` passes.

## Notes

**Recommended splits:**
- `model.rs` → `model/state.rs` (sub-state defaults), `model/cache.rs`
  (element cache helpers), `model/snapshot.rs` (snapshot building).
- `login_flow.rs` → `login_flow/state.rs`, `login_flow/panels.rs`,
  `login_flow/validation.rs`.
- `update/input.rs` → `update/input_text.rs`, `update/input_history.rs`,
  `update/input_nav.rs`.
- `runie-term/src/main.rs` → move effect handlers to `effects/`.

**Out of scope:**
- Rewriting functionality (only move/split code).

## Verification

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace
# Should be empty:
ls build.rs 2>/dev/null
```
