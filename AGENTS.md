# Agent Guidelines

Every change needs fast automatic tests (unit + e2e). No `sleep()` in tests. TUI changes also need a quick live run in tmux or a real terminal.

## Testing Strategy (4 Layers + Black-box Replay)

1. **State/Logic** — pure functions, no Ratatui imports.
2. **Event Handling** — feed `crossterm` events into handlers.
3. **Rendering** — `TestBackend` + `Buffer` assertions.
4. **Provider Replay / Mock-Tool E2E** — captured SSE fixtures and fake tool outputs. Catches async ordering, stale indices, inflight leaks, duplicated `TurnComplete`, stuck timers.
5. **Black-box Replay** — run the real `runie-cli` and `runie-tui` binaries against recorded SSE fixtures via `RUNIE_REPLAY_FIXTURES`. No API keys, no network.

Run layer 4 before every push or when changing async/event logic. Run black-box replay tests when changing CLI/TUI wiring, provider factory behavior, or fixture format.

## Anti-Patterns

| Don't | Why |
|-------|-----|
| Shell or tmux tests | Prefer deterministic Rust tests with mock IO |
| `sleep()` in tests | Non-deterministic |
| Test widget internals | Test rendered output, not structure |
| Mix state + rendering in one test | Hard to debug |

## Architecture Principles

Events-based, single-source-of-truth actors:

- Each state slice is owned by exactly one actor.
- The only change mechanism is events published by the owning actor.
- Handlers, tools, and tests do not mutate another actor's state directly.
- Read-only projections / snapshots are rebuilt from events.
- Every spawned task has an owner (`JoinHandle`, `JoinSet`, or completion event). No orphan `tokio::spawn`.

## File Structure

```
src/
├── app.rs      # State + logic (pure tests)
├── handler.rs  # Event mapping (input tests)
├── ui.rs       # Widgets + layout (render tests)
```

**Rule**: Your `App` should compile without `ratatui` if you strip rendering.

## Linter Rules

`crates/runie-core/build.rs` enforces, across all workspace production `.rs` files:

| Check | Scope |
|-------|-------|
| AppState field access patterns | all `crates/*/src` production code |
| Magic numbers (>= 1000) | all `crates/*/src` production code |
| Orphan `tokio::spawn` calls | all `crates/*/src` production code |

**AppState field access** — internal state fields are accessed through accessors, not directly.

**Magic numbers** — use named constants for buffer sizes, timeouts, and thresholds. Exempt: numbers below 1000, underscore-separated, hex, HTTP/JSON-RPC codes, test code.

**Guidelines**: keep files small, functions focused, and complexity low. Split files around ~400 lines and functions around ~60 lines.
