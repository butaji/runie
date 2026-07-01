# Replace custom bash-safety heuristic with `shell-words`

**Status**: done
**Milestone**: R2
**Category**: Tools
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/bash_safety.rs` implements a hand-rolled quote stripper, variable expansion detector, segment splitter, and a long chain of destructive-pattern checks (~180 LOC). `goose` uses `shell-words` for shell tokenization. Runie should tokenize the command with `shell-words` and apply a small static deny-list, making the safety check easier to audit and extend.

## Acceptance Criteria

- [x] Replace the custom parser in `bash_safety.rs` with `shell_words::split` for tokenization.
- [x] Preserve the existing destructive-command detection logic as a static deny-list applied to tokens.
- [x] Remove the hand-rolled quote stripping / variable expansion / segment splitting code.
- [x] All existing bash-safety tests pass.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `destructive_commands_still_flagged` — `rm -rf /`, `> file`, etc. are rejected.
- [x] `safe_commands_allowed` — `ls`, `git status`, etc. are allowed.
- [x] `quoted_arguments_parsed` — `echo "hello world"` is treated as one argument.

### Layer 2 — Event Handling
- [x] N/A.

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `crates/runie-core/src/bash_safety.rs`
- `crates/runie-core/Cargo.toml`
- `Cargo.toml` (workspace)

## Notes

- `goose` uses `shell-words` for similar shell input handling.
- If variable expansion or command substitution needs to remain detectable, use a small regex on the original string after tokenization.
- Rejected: keep the custom parser for performance — `shell-words` is tiny and the safety check is not hot.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
