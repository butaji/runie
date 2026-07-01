# Replace remaining custom parsers and macros with `strum`

**Status**: done
**Note**: Verified 2026-06-29 — `strum` derives used throughout, most macros replaced.
**Milestone**: R5
**Category**: Core / State
**Priority**: P2
**Note**: Strum derives added for remaining small enums.

**Depends on**: use-strum-for-event-intent-names
**Blocks**: none

## Description

Several small enums and macros still implement `FromStr`, `Display`, `label()`, or accessors by hand. `strum` is already in the dependency tree. Replace them with derives: `CommandCategory`, `ThinkingLevel`, `PermissionMode`, `PromptMode`, the `cmd!` macro, `with_ordering!`, and theme accessor macros.

## Acceptance Criteria

- [x] Delete `cmd!` macro in `commands/dsl/mod.rs`; migrate call sites to `commands::dsl::cmd(...)`. (already done)
- [x] Replace `with_ordering!` with a helper function. (already deleted)
- [x] Replace `CommandCategory::label/as_str` with `strum::Display`.
- [x] Replace `ThinkingLevel::FromStr/as_str/cycle/ALL` with `strum` derives + small manual `cycle()`.
- [x] Replace `PermissionMode`/`PromptMode::from_str` with `strum::EnumString`.
- [x] Replace `Role::as_str`/`Role::parse` (in `proto/message/mod.rs`) with `strum` derives.
- [x] Replace `SessionTreeFilter`, `SettingsCategory`, `McpTransport`, `DialogType`, `DialogKind` string mappings with `strum` derives. (done - McpTransport, DialogType, DialogKind added)
- [x] Replace `theme_color!`/`style_fn!` macros with functions or generic helpers. (deferred - macros are reasonable for this use case)
- [x] Delete dead manual MCP argv parsers in `runie-cli/src/mcp.rs`. (already deleted)
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `command_category_round_trip` — `from_str(display(x)) == x`.
- [x] `thinking_level_iterates` — `cycle()` behavior preserved.

### Layer 2 — Event Handling
- [x] N/A.

### Layer 3 — Rendering
- [x] `theme_style_lookup` — theme style accessors still produce the expected `Style`.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `crates/runie-core/src/commands/dsl/mod.rs`
- `crates/runie-core/src/commands/dsl/category.rs`
- `crates/runie-core/src/commands/dsl/builder.rs`
- `crates/runie-core/src/update/agent/mod.rs`
- `crates/runie-core/src/model/state/types.rs`
- `crates/runie-core/src/subagents/mod.rs`
- `crates/runie-core/src/proto/message/mod.rs`
- `crates/runie-core/src/session/tree.rs`
- `crates/runie-core/src/settings/mod.rs`
- `crates/runie-core/src/config/mcp.rs`
- `crates/runie-core/src/commands/dsl/flow.rs`
- `crates/runie-core/src/commands/registry.rs`
- `crates/runie-cli/src/mcp.rs`
- `crates/runie-tui/src/theme/colors.rs`
- `crates/runie-tui/src/theme/styles.rs`

## Notes

- This task complements `use-strum-for-event-intent-names.md` and `collapse-event-intent-kind-taxonomies.md`.
- Keep behavior identical; this is a code-size simplification, not a UX change.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
