# Simplify text tool shim parsers

## Status

`todo`

## Description

`tool/shim/mod.rs` maintains four fallback parsers for text tool markup. Unify non-XML formats behind a tiny JSON normalizer; keep only the XML path as a distinct shim.

## Acceptance criteria

1. **Unit tests** — Colon/space, arrow, and inline JSON shims all normalize to the same canonical JSON shape.
2. **E2E tests** — Replay fixtures using each shim format still parse.
3. **Live tmux tests** — Run a model that emits text tools and confirm invocation.

## Tests

### Unit tests
- Each legacy format normalizes to canonical JSON.

### E2E tests
- Replay fixtures for each shim format.

### Live tmux tests
- Use a provider/model that emits text tools.
