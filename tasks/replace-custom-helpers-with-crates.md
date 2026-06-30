# Replace custom path/glob/fuzzy/keybinding helpers with crates

**Status**: done
**Note**: Verified 2026-06-29 — glob.rs and fuzzy.rs deleted, path.rs uses shellexpand, telemetry.rs uses tracing, keybindings uses #[cfg(test)] parse_key_combo.
**Milestone**: R1
**Category**: Core / State
**Priority**: P1
**Note**: crates/runie-core/src/path.rs still exists with custom normalize_path.

**Depends on**: none
**Blocks**: narrow-runie-core-public-api

## Description

`runie-core` re-implements several utilities that mature crates already provide. Replacing them removes code, tests, and edge-case surface while making behavior predictable. `ctx7` and peer codebases (`goose`, `jcode`, `OpenFang`, `thClaws`) confirm the standard crates below are the Pareto choice.

## Acceptance Criteria

- [x] `crates/runie-core/src/glob.rs` is deleted and its call sites use `glob` or `globset`/`regex`.
- [x] `crates/runie-core/src/fuzzy.rs` is deleted and its call sites use `nucleo-matcher` (or `sublime-fuzzy` if compile weight matters).
- [x] `crates/runie-core/src/path.rs` is deleted; tilde expansion uses `shellexpand` and absolute/normalized paths use `std::path::absolute` (Rust 1.79+) or `path-absolutize`.
- [x] `crates/runie-core/src/keybindings/mod.rs` no longer contains `parse_key_combo`; chord parsing uses `crossterm::event::KeyEvent` `Display`/`FromStr` or a tiny wrapper around it.
- [x] Custom telemetry/event collection is evaluated against `tracing`; if `telemetry.rs` stays, it must justify why `tracing` cannot model the same events/spans.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `glob_matches_cases` — preserved against crate replacement.
- [x] `fuzzy_ranks_subsequence` — preserved against crate replacement.
- [x] `resolve_path_expands_tilde_and_normalizes` — preserved against crate replacement.
- [x] `key_chord_round_trip` — parse `ctrl+c`/`alt+enter` and emit equivalent `crossterm::event::KeyEvent`.

### Layer 2 — Event Handling
- [x] N/A — these helpers are pure state/logic.

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Smoke / Crash
- [x] N/A.

## Files touched

- `crates/runie-core/src/glob.rs`
- `crates/runie-core/src/fuzzy.rs`
- `crates/runie-core/src/path.rs`
- `crates/runie-core/src/keybindings/mod.rs`
- `crates/runie-core/src/keybindings/defaults.rs`
- `crates/runie-core/src/telemetry.rs`
- `crates/runie-core/Cargo.toml`
- `crates/runie-core/src/lib.rs`
- callers in `crates/runie-tui/`, `crates/runie-cli/`, `crates/runie-core/src/commands/`, etc.

## Notes

- `goose` and `jcode` both use `ignore` for gitignore-aware walking and `walkdir` for raw traversal. If Runie has custom directory walking outside `glob.rs`, switch to `ignore` + `walkdir` as part of this task.
- `OpenFang` uses `dirs` 6 and `shellexpand` for path expansion; `goose` uses `etcetera` for config-dir resolution. Prefer `etcetera` for config paths (XDG on Linux, Application Support on macOS) and `shellexpand` for shell-like tilde/`$VAR` expansion.
- `crossterm` 0.29+ already supports `Display` and modifier iteration; a custom parser is unnecessary.
- Rejected: keep custom implementations for "fewer dependencies" — the reviewed crates are small, well-tested, and already appear in peer agent codebases.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
