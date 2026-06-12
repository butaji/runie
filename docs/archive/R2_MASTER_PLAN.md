# R2 Master Plan: Full pi Parity

**Date**: 2026-06-08
**Philosophy**: 80/20 rule. Less code, more power. Deep unification.
**Architecture**: Event-driven, MVU, non-blocking, dialog-as-state.
**Design inspiration**: Crush (Bubble Tea patterns), pi (semantic commands), gptme (registry).

## Core Principle: Everything is an Event

Every user action — slash command, keybinding, palette selection, dialog button — dispatches an `Event`. The update loop is the single source of truth for all state changes.

```
User Input → Event → Update Loop → State Change → Snapshot → Render
                ↑___________________________________________|
                          (palette selections recurse)
```

## R2 Task Groups

### Group 1: Unified Action System (Foundation)

These tasks must be completed first. They establish the architectural foundation.

| # | Task | Files | Lines |
|---|------|-------|-------|
| 1.1 | **Command Registry** | `commands/mod.rs`, `commands/registry.rs` | ~150 |
| 1.2 | **Dialog State** | `model.rs` (DialogState enum), `update/dialog.rs` | ~100 |
| 1.3 | **Semantic Keybindings** | `keybindings.rs` (name→Event), `keymap.rs` | ~200 |
| 1.4 | **Command Palette** | `dialog/palette.rs`, `ui.rs` render | ~250 |

### Group 2: Core Commands (Daily Use)

Built on the registry. Each command = one `CommandEntry` with a factory function.

| # | Task | Commands | Priority |
|---|------|----------|----------|
| 2.1 | **Session Commands** | `/save`, `/load`, `/sessions`, `/delete`, `/name`, `/new`, `/resume`, `/compact`, `/reset` | High |
| 2.2 | **Model Commands** | `/model`, `/scoped-models`, thinking levels | High |
| 2.3 | **System Commands** | `/copy`, `/settings`, `/reload`, `/changelog`, `/hotkeys`, `/help`, `/quit` | High |
| 2.4 | **Safety Commands** | `/readonly`, `/trust`, `/untrust` | Medium |
| 2.5 | **Export/Import** | `/export`, `/import` | Medium |

### Group 3: Dialogs (Rich UX)

Dialogs are `DialogState` variants in `AppState`. Not separate actors.

| # | Task | DialogState | Trigger |
|---|------|-------------|---------|
| 3.1 | **Model Selector** | `ModelSelector { filter, selected }` | Ctrl+L, `/model` |
| 3.2 | **Settings** | `Settings { category, selected }` | `/settings` |
| 3.3 | **Sessions** | `Sessions { filter, selected }` | Ctrl+S, `/sessions` |

### Group 4: Features (Polish)

| # | Task | What | Priority |
|---|------|------|----------|
| 4.1 | **Model Cycling** | Ctrl+P / Shift+Ctrl+P through scoped list | High |
| 4.2 | **Dequeue** | Alt+Up restores queued message | Medium |
| 4.3 | **External Editor** | Ctrl+G opens $EDITOR | Medium |
| 4.4 | **Thinking Levels** | Shift+Tab cycles off/low/medium/high | Medium |
| 4.5 | **Theme System** | opaline integration, 39 themes | Medium |
| 4.6 | **Output Accumulator** | Rolling buffer guard | Low |
| 4.7 | **Provider Attribution** | Per-message provider label | Low |
| 4.8 | **Trust System** | Per-project trust decision | Low |
| 4.9 | **File Mutation Queue** | Serialized edits | Low |
| 4.10 | **Path Completion** | Tab completes paths | Low |
| 4.11 | **Lint Zero Warnings** | clippy --all-targets clean | Hygiene |

## What We Skip (80/20)

| Feature | Why Skip |
|---------|----------|
| Session tree / branching | Complex, niche. Few users fork sessions. |
| OAuth / login flow | Infrastructure heavy. API keys in config.toml suffice. |
| Print / JSON / RPC modes | Separate binaries. Post-production. |
| Skills system | Large architecture. Custom prompts in config.toml suffice. |
| Image paste | Clipboard integration + vision models. Post-production. |
| Suspend to background | Shell-specific. Ctrl+C quit is standard. |
| Telemetry | Optional. Add only if needed. |
| Config migrations | Low priority. config.toml is simple. |
| Session filters | Niche. Tree navigation already skipped. |
| Edit diff preview | Diff rendering already exists. Pre-apply preview is nice-to-have. |

## Test Coverage Requirement

Every R2 task must include:
- **Layer 1**: Pure state tests (registry lookup, dialog state transitions)
- **Layer 2**: Event→state tests (keybinding emits Event, palette select triggers action)
- **Layer 3**: Render tests (TestBackend asserts dialog appearance)
- **Layer 4** (optional): Smoke tests for async paths only

## Implementation Order

```
Week 1: Group 1 (Foundation)
  → Command Registry
  → Dialog State
  → Semantic Keybindings
  → Command Palette

Week 2: Group 2 (Core Commands) + Group 3 (Dialogs)
  → Migrate existing slash commands to registry
  → Add missing commands
  → Model Selector dialog
  → Settings dialog

Week 3: Group 4 (Features)
  → Model cycling, dequeue, external editor
  → Thinking levels, themes
  → Output accumulator, trust system

Week 4: Polish
  → Lint zero warnings
  → Smoke tests
  → Documentation
```
