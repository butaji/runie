# Standardize session persistence on JSONL

## Status

`todo`

## Description

Session persistence currently mixes JSONL with a custom `SqliteStore`. SQLite is deferred for now; JSONL must be the single canonical runtime store. Remove or fold `SqliteStore` into the JSONL session path so there is one persistence format.

## Acceptance criteria

- `SqliteStore` is removed or merged into the JSONL session store.
- Session metadata, messages, and durable events round-trip through JSONL.
- No dual persistence paths remain in production code.

## Tests

### Layer 1 — State/Logic
- Store round-trips metadata, messages, and durable events through JSONL.

### Layer 2 — Event Handling
- `SessionLoaded`, `SessionSaved`, `SessionDeleted` events still work.

### Layer 4 — Provider Replay / Mock-Tool E2E
- A saved session replays deterministically after cleanup.
