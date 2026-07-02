# Replace production `expect`/`unwrap` panics with `Result` propagation

## Status

`todo`

## Description

Production code panics in `tool/shim/mod.rs`, `model/compaction.rs`, `session/tree.rs`, and `runie-provider/openai/stream.rs`. Replace with `Result` propagation or `LazyLock` for regexes; document remaining invariants.

## Acceptance criteria

1. **Unit tests** — No new panics; regex lazy initialization works; parse failures return errors.
2. **E2E tests** — Malformed tool markup and compaction inputs are handled gracefully.
3. **Live tmux tests** — Paste malformed input or trigger edge cases in tmux; app stays alive.

## Tests

### Unit tests
- Regex construction and parser error paths.

### E2E tests
- Malformed shim/tool input replay.

### Live tmux tests
- Submit a message that triggers a parser edge case.

### SSOT/Event Compliance
- [ ] **Actor/SSOT:** N/A (error handling change; actors remain authoritative).
- [ ] **Trigger events:** N/A (error handling doesn't introduce new state transitions).
- [ ] **Observer events:** Parse errors may emit `Error` events or return `Result`.
- [ ] **No direct mutations:** N/A (error handling doesn't change state ownership).
- [ ] **No new mirrors:** N/A (error handling doesn't introduce new state).
- [ ] **Async work observed:** Errors in async contexts must be propagated, not silently dropped.
