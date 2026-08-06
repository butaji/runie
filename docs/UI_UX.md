# Runie UI / UX

Runie is a keyboard-driven terminal interface using **Model-View-Update (MVU)** at the UI layer: user actions become intents, actors update the model, and the view renders a pure projection of facts.

## Interaction model

- **Feed** — scrollable conversation history.
- **Input** — multi-line editor with history, undo/redo, `@` file references, path completion.
- **Dialog / Panel stack** — palettes, forms, settings, model selector, onboarding, session tree.
- **Status bar** — provider/model, thinking level, trust state, mode, and hints.
- **Vim navigation** — optional nav mode for scrolling, selecting blocks, and copying.

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

- Core behavior and event handling: `crates/runie-core/src/tests/`
- Rendering and TUI integration: `crates/runie-tui/src/tests/`
- Provider-specific agent flows: `crates/runie-agent/tests/` with captured fixtures and mock tool outputs

## Implementation notes

- Prefer Layer 2 for command behavior and Layer 3 for visual confirmation.
- Use validation hooks, mock providers, and captured SSE replay fixtures for provider-specific behavior.
- Never add shell or tmux tests; prefer deterministic Rust tests.
