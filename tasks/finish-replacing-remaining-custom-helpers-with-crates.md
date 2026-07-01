# Finish replacing remaining custom helpers with crates

## Status

`todo`

## Description

Several small helpers for fuzzy matching, path/glob expansion, keybinding parsing, shell word splitting, and text wrapping are still custom or partially custom. Replace them with `nucleo-matcher`, `globset`, `shellexpand`, `crokey`, `shell-words`, `textwrap`.

## Acceptance criteria

- No custom fuzzy/path/glob/keybinding/text helpers remain in production code.
- Behavior is preserved or improved.

## Tests

### Layer 1 — State/Logic
- Each replacement has unit tests matching old behavior.
