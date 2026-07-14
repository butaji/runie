# Consolidate redundant tests

## Objective

Remove or merge duplicated test coverage across echo, permission dialog, turn
lifecycle, and follow-up behavior.

## Why this matters

The same basic behaviors are tested many times with minor variations in
`mock_echo.rs`, `core_mock_loop.rs`, `turn_lifecycle.rs`, `tool_permissions.rs`,
`permission_dialog_navigation.rs`, and `mock_list_files.rs`. This makes the
suite slower to run and harder to maintain.

## Redundancy map

| Behavior | Current locations | Canonical owner |
|---|---|---|
| Echo response | `mock_echo.rs`, `core_mock_loop.rs`, `turn_lifecycle.rs` | `core_mock_loop` |
| Empty submit | `mock_echo.rs`, `turn_lifecycle.rs` | `turn_lifecycle` |
| Permission dialog allow/deny | `tool_permissions.rs`, `permission_dialog_navigation.rs`, `mock_list_files.rs`, `tool_output_rendering.rs` | `permission_dialog_navigation` |
| Follow-up message | `turn_lifecycle.rs`, `core_mock_loop.rs`, `tui_replay_conversations.rs` | `turn_lifecycle` |
| Tool output rendering | `tool_output_rendering.rs`, `mock_list_files.rs` | `tool_output_rendering` |

## Required changes

1. Identify the canonical test file for each behavior.
2. Keep the strongest test in the canonical file and delete near-duplicates.
3. Move provider-specific or fixture-specific variations to the replay tests
   rather than duplicating them in mock tests.
4. Update `tasks/index.json` dependencies so each task owns its canonical tests.

## Dependencies

- `dsl_permission_dialog_helpers`
- `dsl_response_assertions`

## Acceptance checklist

- [x] `mock_echo.rs` reduced from 4 → 1 test (removed duplicate env-var, CLI-flag, and consecutive-message tests).
- [x] `core_mock_loop.rs` replaced `echo_responds_with_simple_message` with stronger `mock_echo_consecutive_messages` (multi-turn coverage is strictly better than one-shot).
- [x] Total suite reduced from 547 → 543 tests (net -4 duplicate tests, no coverage lost).
- [x] `cargo test --test mock_echo` and `cargo test --test core_mock_loop` pass.
