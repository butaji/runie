# Sweep `#[allow(...)]` suppressions

**Status**: todo
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
- [ ] Dead-code allows resolved by deleting the dead item: `session_store.rs:21`, `dialog/dsl/panel.rs:8`, `update/dialog/tab_complete.rs:106,140`, `update/input/support.rs:81`, `keybindings/mod.rs:26`, `runie-tui/src/terminal_setup.rs:202`, `runie-tui/src/ui/render_lines.rs:15`, `runie-tui/src/status_bar.rs:150`.
- [ ] `clippy::too_many_arguments` sites either refactored (parameter object / builder) or documented: `runie-agent/src/turn.rs:158`, `runie-agent/src/subagent.rs:34`, `runie-tui/src/popups/panel/form.rs:81,119,267`, `runie-tui/src/message/mod.rs:82,119`, `runie-core/src/snapshot.rs:348`.
- [ ] Niche-lint allows kept with comment: `dialog/panel.rs:22` (fn pointer comparison), `runie-tui/src/effects/clipboard.rs:36` (unreachable), `runie-agent/src/headless.rs:36` + `update/dialog/model_selector.rs:2` (type_complexity), `runie-tui/src/popups/welcome.rs:22` (vec_init_then_push).
- [ ] `cargo clippy --workspace` passes with zero warnings after removing allows.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] N/A — suppression removal is a compile/clippy check, no new state logic.

### Layer 2 — Event Handling
- [ ] `tab_complete_without_dead_code_still_completes` — if any `update/dialog/tab_complete.rs` dead item was used in tests, those tests still pass.

### Layer 3 — Rendering
- [ ] N/A — no rendering changes.

### Layer 4 — Smoke / Crash
- [ ] `cargo clippy --workspace` green confirms suppressions were correctly resolved (not just hidden).

## Files touched

- `crates/runie-core/src/session_store.rs` — resolve line 21 allow
- `crates/runie-core/src/dialog/dsl/panel.rs` — resolve line 8 allow
- `crates/runie-core/src/update/dialog/tab_complete.rs` — resolve lines 106, 140
- `crates/runie-core/src/update/input/support.rs` — resolve line 81
- `crates/runie-core/src/keybindings/mod.rs` — resolve line 26
- `crates/runie-core/src/snapshot.rs` — refactor or document line 348
- `crates/runie-core/src/dialog/panel.rs` — add comment to line 22
- `crates/runie-tui/src/terminal_setup.rs` — resolve line 202
- `crates/runie-tui/src/ui/render_lines.rs` — resolve line 15
- `crates/runie-tui/src/status_bar.rs` — resolve line 150
- `crates/runie-tui/src/popups/panel/form.rs` — refactor or document lines 81, 119, 267
- `crates/runie-tui/src/message/mod.rs` — refactor or document lines 82, 119
- `crates/runie-agent/src/turn.rs` — refactor or document line 158
- `crates/runie-agent/src/subagent.rs` — refactor or document line 34
- `crates/runie-agent/src/headless.rs` — add comment to line 36
- `crates/runie-tui/src/effects/clipboard.rs` — add comment to line 36
- `crates/runie-tui/src/popups/welcome.rs` — add comment or refactor line 22
- `crates/runie-core/src/update/dialog/model_selector.rs` — add comment to line 2

## Notes

`too_many_arguments` refactors (parameter objects) can grow into large changes — if a refactor balloons, document the allow instead and file a follow-up. Prefer deleting dead code over "keeping it for later" — if it's truly needed, it will be re-added with tests. The 8 `dead_code` allows are the highest-confidence deletions. Do NOT touch test-only allows (in `tests/` dirs or under `#[cfg(test)]`).
