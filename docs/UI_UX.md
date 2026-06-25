# Runie UI / UX

Runie is a keyboard-driven terminal interface. It follows a **Model-View-Update (MVU)** pattern at the UI layer: user actions become intents, actors update the model, and the view renders a pure projection of actor-emitted facts.

## Interaction model

- **Feed** — scrollable conversation history (user messages, assistant streaming, tool calls, system messages).
- **Input** — multi-line editor with history, undo/redo, `@` file references, and path completion.
- **Dialog / Panel stack** — palettes, forms, settings, model selector, onboarding, session tree.
- **Status bar** — current provider/model, thinking level, trust state, mode, and contextual hints.
- **Vim navigation** — optional nav mode for scrolling, selecting blocks, and copying without leaving the keyboard.

Every user action produces an **intent**. Actors own the authoritative model, apply intents, and publish **facts**. The UI layer projects those facts into a `Snapshot` and renders it. The render function is pure: `draw(&mut Frame, &Snapshot)`.

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

- Handlers are pure builders of intents. They do not mutate `AppState`.
- Actors are the only source of truth for their state slice.
- Facts are the only way the UI learns about state changes.
- Rendering is a pure function of the snapshot.

## Testing layers for UI

See [AGENTS.md §Testing Strategy](../AGENTS.md#testing-strategy-4-layers) for the full 4-layer test taxonomy.

## Existing coverage by area

### Onboarding / Login
- L1/L2: provider picker, key input, validation success/failure/retry, model selection, save/cancel navigation.
- L3: provider list, key field, validating panel, model toggles, error rendering.
- E2E: full first-run flow, invalid key retry, paste, backspace, space toggle, second provider, reset preserving model.

### Command palette
- L2/L3: open/close, `/` and `Ctrl+P`, filtering, selection wrapping, back-stack with sub-dialogs.

### Settings dialog
- L2/L3: open/close, navigation, toggles, provider/theme/model, truncation fields, vim navigation.

### Model selector
- L1/L2: filter by name/provider, recent models capped/deduped, grouped rendering, select/switch, wrap.
- L3: grouped dialog rendering with current-model star.

### Slash commands
- Core/session: `help`, `quit`, `reset`, `save/load/delete/sessions`, `session`, `export/import`, `name`, `fork`, `tree`.
- Model: `/model` with no args, provider/model args, and plain model name.
- Safety: `/readonly`, `/trust`, `/untrust`.
- System: `/theme`, `/spawn`, `/agents`, `/skills`, `/prompt`, `/hotkeys`, `/reload`, `/copy`, `/diagnostics`.

### Input
- L1/L2: history, undo/redo, word navigation, cursor, grapheme handling, flash.
- L2/L3: typing, backspace, submit, reset via palette, external editor, quit commands, timer behavior.

### Tools
- L3: tool running/done/summary rendering with spinner, checkmark, bytes, error icon, duration.
- L2: multi-turn tool flows, queue delivery, no stuck timers.

### Vim navigation
- L1/L2: mode on/off, `j/k/g/G` scroll, `Esc` enters nav, `Space/i` exit, `y/Y` copy, hints, nav at lowest element exits.
- L3: status hints, scroll rendering, nav-mode disabled input style, accent bracket, selection wrap mapping.

### Collapse / expand
- L2/L3: global toggle collapses thoughts/tools, respects global flag for new items, running tools stay expanded.

## High-value gaps

| Priority | Area | Missing coverage |
|----------|------|------------------|
| P1 | Onboarding E2E | First-run flow end-to-end with validation and model save |
| P1 | Provider management | Disconnect/reconnect flows, provider list rendering |
| P1 | Settings persistence | Truncation values, theme/provider changes persist and render |
| P1 | Slash commands | `/new`, `/history`, `/resume`, `/share`, `/compact`, `/workflow`, `/steer`, `/approve`, `/reject`, `/team`, `/solo` |
| P1 | Session tree | Navigation, filter cycle, selecting a branch to switch conversation |
| P1 | `@` picker | Open, filter, select file, render `[path]` in input |
| P1 | Tool errors | Error state visible with ✗ icon and text |
| P1 | Streaming abort | Abort during streaming then submit new message, no stuck timer |
| P1 | Queue | `Alt+Enter` follow-up, `Alt+Up` dequeue, queued message rendering |
| P2 | Model selector | Zero configured providers, unconfigured model arg, `Ctrl+M` cycling |
| P2 | Input advanced | Shift+Enter multiline, bracketed paste, long input scrolling, placeholder when scrolled up |
| P2 | Resize stress | Resize during streaming and open dialogs |
| P2 | Trust banner | Untrusted project banner and `/trust` flow |
| P3 | Mouse | Scroll wheel, click focus, click palette/model item |

## Test placement conventions

- Core behavior and event handling: `crates/runie-core/src/tests/`
- Rendering and TUI integration: `crates/runie-tui/src/tests/`
- Provider-specific agent flows: `crates/runie-agent/tests/` with captured fixtures and mock tool outputs

## Implementation notes

- Prefer Layer 2 for command behavior and Layer 3 for visual confirmation.
- Use validation hooks, mock providers, and captured SSE replay fixtures for provider-specific behavior.
- Never add shell or tmux smoke tests; prefer deterministic Rust tests.
- When adding new slash command coverage, place L1/L2 tests in `crates/runie-core/src/tests/slash/` or `commands/tests/`, and L3 tests in `crates/runie-tui/src/tests/render/`.
