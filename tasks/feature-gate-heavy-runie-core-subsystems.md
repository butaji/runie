# Feature-gate heavy `runie-core` subsystems

## Status

`done`

## Description

`runie-core` compiles ~80 dependencies unconditionally. Add feature flags for MCP, keyring, git status, file watching, clipboard, markdown YAML, and model catalog YAML.

## Acceptance Criteria

1. ✅ Each feature compiles independently (`--no-default-features --features <name>`).
2. ✅ Default build passes `cargo check --workspace` and `cargo test --workspace`.
3. ✅ Minimal build (`--no-default-features`) also compiles.
4. ✅ All features together (`--all-features`) also compiles.

## Tests

### Unit tests
- Feature matrix compiles.

### E2E tests
- Default feature smoke tests.

### Live tmux tests
- Run stripped-down build in tmux.

### SSOT/Event Compliance
- [ ] **Actor/SSOT:** N/A (feature gates; actors remain authoritative).
- [ ] **Trigger events:** N/A (feature gates don't introduce state transitions).
- [ ] **Observer events:** N/A (feature gates don't emit events).
- [ ] **No direct mutations:** N/A (feature gates don't change state ownership).
- [ ] **No new mirrors:** N/A (feature gates don't introduce new state).
- [ ] **Async work observed:** N/A (feature gates don't introduce async work).
