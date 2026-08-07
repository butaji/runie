# Runie UI / UX

Runie is a keyboard-driven Grok-style terminal interface using
**Model-View-Update (MVU)** at the UI layer. Its presentation surface follows
Grok; its behavior is limited to features supplied by pi-agent-core.

## Interaction model

- **Feed** — scrollable conversation history.
- **Input** — multi-line editor with history, undo/redo, `@` file references, path completion.
- **Dialog / Panel stack** — palettes and panels supported by the core feature set.
- **Status bar** — model, usage, waiting state, mode, and hints.
- **Navigation** — scrolling, selection, and copying for core transcript blocks.

Terminal input is delivered by an owned asynchronous event-stream worker. The
render loop selects on the input mailbox and its animation tick independently;
input handling never calls crossterm polling or reads another actor's state.

Reset clears transient prompt, feed, and status facts while preserving the
selected theme and model caption, matching the actor-owned configuration
contract.

## MVU flow

```text
User input
    │
    ▼
Input handler (pure) ──► Intent event
    │
    ▼
Owning actor ──► authoritative state update ──► Fact event
    │
    ▼
UiActor projection (pure) ──► Snapshot
    │
    ▼
RenderActor (pure) ──► Frame
```

Rules:

- Handlers are pure builders of intents; they do not mutate `AppState`.
- Actors are the only source of truth for their state slice.
- Facts are the only way the UI learns about state changes.
- Rendering is a pure function of the snapshot.

## Testing layers

See [AGENTS.md §Testing Strategy](../AGENTS.md#testing-strategy-4-layers) for the 4-layer test taxonomy.

## Test placement

- Core behavior and event handling: `crates/runie-core/src/` unit tests and `tests/traces/`
- Rendering and TUI integration: `crates/runie-tui/src/` tests and `tests/e2e/*.yaml`
- Full-screen parity: `parity/` captures and visual replay tests

## Implementation notes

- Prefer Layer 2 for command behavior and Layer 3 for visual confirmation.
- Use validation hooks, mock providers, and captured SSE replay fixtures for
  provider-boundary behavior.
- Capture tools may use tmux/asciinema, but assertions must be deterministic
  and compare complete terminal cells.
