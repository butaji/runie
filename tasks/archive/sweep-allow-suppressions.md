# Sweep `#[allow(...)]` suppressions

**Status**: in_progress
**Milestone**: R4
**Category**: Core / State
**Priority**: P2

**Depends on**: gate-or-implement-mcp-client, delete-config-reload-shim
**Blocks**: none

## Description

18 `#[allow(...)]` markers ship in production code. Each is either (a) dead code kept "for later" that ships in prod binaries — a YAGNI violation, or (b) a clippy lint suppression that hides a real refactor opportunity. Categorize and resolve each: delete dead code, or refactor to remove the suppression. Niche lints (`unreachable`, `unpredictable_function_pointer_comparisons`, `type_complexity` on a real return type) may stay with a justifying comment. This is the 80/20 of "code that shouldn't be here."

Already covered by other tasks (do not duplicate): `mcp.rs` (4x dead_code) → `gate-or-implement-mcp-client`; `config_reload/types.rs` (3x dead_code) → `delete-config-reload-shim`; test-only allows in `tests/vim_mode.rs`, `runie-tui/src/tests/core/*` → leave (test code).

## Acceptance Criteria

- [ ] Every remaining `#[allow(...)]` in production code either: (a) removed because the suppressed issue is fixed, or (b) has a `// allow: <concrete reason>` comment explaining why the suppression is correct.
- [x] Dead-code allows resolved by deleting the dead item: `dialog/dsl/panel.rs:8` (removed `list` function).
- [x] Niche-lint allows with comments added: `dialog/panel_split/mod.rs:22` (fn pointer comparison), `effects/clipboard.rs:36` (unreachable), `headless.rs:36` (type_complexity already had comment), `model_selector.rs:2` (type_complexity already had comment), `welcome.rs:22` (vec_init_then_push).
- [ ] Dead-code allows resolved: `session_store.rs` (file deleted), `update/dialog/tab_complete.rs` (no longer has dead_code allows), `update/input/support.rs` (no longer has dead_code allows), `keybindings/mod.rs` (used by tests), `terminal_setup.rs` (no dead_code), `render_lines.rs` (function is used), `status_bar.rs` (field is used).
- [ ] `clippy::too_many_arguments` sites either refactored (parameter object / builder) or documented.
- [ ] `cargo test --workspace` succeeds.

## Progress

**Completed:**
- Removed dead `list` function from `dialog/dsl/panel.rs` and its exports
- Added `// allow:` comments to niche lints: `clipboard.rs` (unreachable), `welcome.rs` (vec_init_then_push), `panel_split/mod.rs` (fn pointer comparison)
- Updated `dialog/mod.rs` documentation to remove references to deleted `list` function

**Not yet addressed:**
- `too_many_arguments` refactoring/documentation for: `turn.rs`, `subagent.rs`, `form.rs`, `message/mod.rs`, `snapshot.rs`
- Verify remaining `dead_code` allows are accurate (many files no longer have the allows mentioned in the task)

## Tests

### Layer 1 — State/Logic
- [x] N/A — suppression removal is a compile/clippy check, no new state logic.

### Layer 2 — Event Handling
- [x] `cargo test --workspace` succeeds after changes.

### Layer 3 — Rendering
- [x] N/A — no rendering changes.

### Layer 4 — Smoke / Crash
- [x] `cargo test --workspace` green confirms changes don't break anything.

## Files touched

- `crates/runie-core/src/dialog/dsl/panel.rs` — removed dead `list` function
- `crates/runie-core/src/dialog/dsl/mod.rs` — removed `list` from exports
- `crates/runie-core/src/dialog/mod.rs` — updated documentation
- `crates/runie-core/src/dialog/panel_split/mod.rs` — added allow comment
- `crates/runie-tui/src/effects/clipboard.rs` — added allow comment
- `crates/runie-tui/src/popups/welcome.rs` — added allow comment

## Notes

`too_many_arguments` refactors (parameter objects) can grow into large changes — if a refactor balloons, document the allow instead and file a follow-up. Prefer deleting dead code over "keeping it for later" — if it's truly needed, it will be re-added with tests. The `dead_code` allows in many files were already removed or the referenced lines no longer exist (code has changed since task was authored).
