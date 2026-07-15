# DSL permission dialog helpers

## Objective

Add DSL helpers for triggering and resolving tool permission dialogs so tests
stop duplicating the same sequence.

## Why this matters

Many tests repeat:

```rust
.type_text("list files").await?
.press(keys::ENTER).await?
.expect_text("Permission Required").await?
```

followed by allow/deny selection. A shared helper reduces duplication and makes
tests read at the scenario level.

## Proposed helpers

Implemented in `src/app_test.rs`:

- `request_tool_permission(prompt_text: &str)` — types the prompt and submits
  it, then waits for the dialog.
- `allow_permission_once()` / `allow_permission_always()` / `deny_permission()`
  — select the option and confirm.

`expect_permission_dialog(tool_name)` can be composed from
`expect_text("Permission Required")` and `expect_selected_row(tool_name)`.

## Files that will benefit

- `tests/mock_list_files.rs`
- `tests/tool_permissions.rs`
- `tests/permission_dialog_navigation.rs`
- `tests/tool_output_rendering.rs`
- `tests/core_mock_loop.rs`
- `tests/dialog_navigation.rs`
- `tests/tui_replay_conversations.rs`

## Dependencies

- `tool_permissions`
- `black_box_replay_dsl`

## Acceptance checklist

- [x] Helpers exist and are documented in `AGENTS.md`.
- [x] At least three test files are converted to use them.
- [x] No duplication of the trigger → dialog → resolve sequence remains.
