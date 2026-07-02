# Use `tree-sitter` for `find_definitions`

## Status

`todo`

## Description

`find_definitions` uses ad-hoc language detection with `starts_with` checks. Replace with `tree-sitter` parsers for supported languages, or a single regex table as a fallback.

## Acceptance criteria

1. **Unit tests** — Definition detection is correct for Rust/Python/TS sample files.
2. **E2E tests** — A replay turn asking for definitions returns accurate symbols.
3. **Live tmux tests** — Ask the agent "find definitions of X" in tmux.

## Tests

### Unit tests
- Sample files for each supported language.

### E2E tests
- Replay fixture requesting definitions.

### Live tmux tests
- Open a codebase and request definitions.
