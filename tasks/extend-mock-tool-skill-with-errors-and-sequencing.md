# Extend MockToolSkill with errors and sequencing

## Status

`done`

## Context

`crates/runie-testing/src/mock_tool_skill.rs:16-43` only supports fixed `SkipWithOutput` success. It cannot simulate errors, verify call order, or inspect `ToolCallCtx`.

## Goal

Add a builder API:
```rust
MockToolSkill::new()
    .when("bash", ToolResult::Success("hello"))
    .when("read_file", ToolResult::Error("not found"))
    .expect_calls(vec!["list_dir", "read_file"]);
```

## Acceptance Criteria

- [x] Add `ToolResult` enum (Success/Error).
- [x] Add `when` and `expect_calls` builder methods.
- [x] Update existing tests to use builder.
- [x] All tests pass.

## Design Impact

No change to TUI element design or composition. Only test helpers change.

## Tests

- **Layer 1 — State/Logic:** Unit tests for builder expectations.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Agent tests with mock tools pass.
- **Live tmux validation:** N/A.

## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
