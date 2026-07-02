# Finish replacing remaining custom helpers with crates

## Status

`todo`

## Description

Several small helpers for fuzzy matching, path/glob expansion, keybinding parsing, shell word splitting, and text wrapping are still custom or partially custom. Replace them with `nucleo-matcher`, `globset`, `shellexpand`, `crokey`, `shell-words`, `textwrap`.

## Acceptance criteria

1. **Unit tests** — Each replacement matches the old behavior for representative inputs.
2. **E2E tests** — Commands using helpers (path/glob/fuzzy/keybinding) work in mock-provider replay.
3. **Live run tests** — Use affected features (palette search, file picker, slash commands) in tmux.

## Tests

### Unit tests
- Each replacement has unit tests matching old behavior.

### E2E tests
- A replay run exercises path expansion, fuzzy matching, and keybinding parsing.

### Live run tests
- Open the command palette, file picker, and submit a slash command in tmux.

### SSOT/Event Compliance
- [ ] **Actor/SSOT:** N/A (utility function replacements; actors remain authoritative).
- [ ] **Trigger events:** N/A (helper replacements don't introduce new state transitions).
- [ ] **Observer events:** N/A (helper replacements don't emit events).
- [ ] **No direct mutations:** N/A (helper replacements don't change state ownership).
- [ ] **No new mirrors:** N/A (helper replacements don't introduce new state).
- [ ] **Async work observed:** N/A (synchronous helper functions).
