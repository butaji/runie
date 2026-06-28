# Replace custom bash-safety heuristic with `shell-words`

**Status**: todo
**Milestone**: R2
**Category**: Tools
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/bash_safety.rs` implements a hand-rolled quote stripper, variable expansion detector, segment splitter, and a long chain of destructive-pattern checks (~180 LOC). `goose` uses `shell-words` for shell tokenization. Runie should tokenize the command with `shell-words` and apply a small static deny-list, making the safety check easier to audit and extend.

## Acceptance Criteria

- [ ] Replace the custom parser in `bash_safety.rs` with `shell_words::split` for tokenization.
- [ ] Preserve the existing destructive-command detection logic as a static deny-list applied to tokens.
- [ ] Remove the hand-rolled quote stripping / variable expansion / segment splitting code.
- [ ] All existing bash-safety tests pass.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `destructive_commands_still_flagged` — `rm -rf /`, `> file`, etc. are rejected.
- [ ] `safe_commands_allowed` — `ls`, `git status`, etc. are allowed.
- [ ] `quoted_arguments_parsed` — `echo "hello world"` is treated as one argument.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/bash_safety.rs`
- `crates/runie-core/Cargo.toml`

## Notes

- `goose` uses `shell-words` for similar shell input handling.
- If variable expansion or command substitution needs to remain detectable, use a small regex on the original string after tokenization.
- Rejected: keep the custom parser for performance — `shell-words` is tiny and the safety check is not hot.
