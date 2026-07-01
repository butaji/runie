# Migrate session persistence to `sqlx` with migrations

## Status

`todo`

## Description

Session persistence currently mixes JSONL with a custom `SqliteStore` using `rusqlite`. Standardize runtime session state on `sqlx` + migrations. Keep JSONL only as an event-log export format.

## Acceptance criteria

- `SqliteStore` uses `sqlx` with `migrate!` macros.
- Schema migrations are version-controlled.
- JSONL is export-only, not the primary store.

## Tests

### Layer 1 — State/Logic
- Store round-trips metadata, messages, and durable events.

### Layer 2 — Event Handling
- `SessionLoaded`, `SessionSaved`, `SessionDeleted` events still work.

### Layer 4 — Provider Replay / Mock-Tool E2E
- A saved session replays deterministically after the migration.
