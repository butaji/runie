# Unified Action System

**Status**: Proposed
**Date**: 2026-06-08

## Context

After studying pi, Crush, gptme, and aider, we need a unified way to handle all user actions:
- Slash commands (`/model`, `/save`)
- Keybindings (Ctrl+P, Ctrl+L, Ctrl+O)
- Command palette selections
- Dialog confirmations
- Mouse clicks

Current runie uses a monolithic `match` in `slash.rs` and hardcoded key handling in `main.rs`. Adding a new action requires touching 3+ files.

## Decision

### One Event Type to Rule Them All

Every user action — slash, keybinding, palette selection, dialog button — **dispatches an `Event`**. No special Action enum. No command handler functions. Just Events flowing into the same update loop.

```rust
// Before: slash.rs has a match, keymap.rs has another match, palette has a third
// After: everything emits Event::*

// Slash: /model gpt-4o  →  Event::SwitchModel { provider, model }
// Key: Ctrl+P            →  Event::CycleModelNext
// Palette: "new session"  →  Event::NewSession
// Dialog: model selected  →  Event::SwitchModel { provider, model }
```

### CommandRegistry = Name → Event Factory

The registry doesn't contain handlers. It contains **event factories**:

```rust
pub struct CommandEntry {
    pub name: String,
    pub description: String,
    pub aliases: Vec<String>,
    pub category: CommandCategory,
    pub factory: CommandFactory,  // fn(&str) -> Option<Event>
    pub completer: Option<CommandCompleter>,
}

pub type CommandFactory = fn(args: &str) -> Option<Event>;
```

This keeps all state mutation in `update/` modules. The registry is pure logic.

### Dialogs are State, Not Actors

Following Crush's pattern, dialogs are **state fields in AppState**, not separate actors:

```rust
pub struct AppState {
    // ... existing fields ...
    pub dialog_stack: Vec<DialogState>,
}

pub enum DialogState {
    CommandPalette { filter: String, selected: usize },
    ModelSelector { filter: String, selected: usize },
    Settings { category: SettingsCategory, selected: usize },
    Sessions { filter: String, selected: usize },
}
```

Dialogs process input and emit Events. The update loop handles the Events. Rendering draws the dialog state.

### Keybindings are Just Event Sources

Keybindings don't call functions. They **emit Events**:

```rust
fn map_key_event(key: KeyEvent, bindings: &HashMap<String, Event>) -> Option<Event> {
    let combo = key_combo(key);
    bindings.get(&combo).cloned()
}
```

The default bindings map:
```json
{
  "ctrl+p": "ToggleCommandPalette",
  "ctrl+l": "ToggleModelSelector",
  "ctrl+o": "ToggleExpand",
  "ctrl+t": "ToggleThinking",
  "shift+tab": "CycleThinkingLevel",
  "ctrl+g": "OpenExternalEditor",
  "alt+up": "Dequeue",
  "alt+enter": "FollowUp"
}
```

### The Update Loop is the Single Source of Truth

```rust
fn update(state: &mut AppState, event: Event) {
    match event {
        // Dialog events
        Event::ToggleCommandPalette => toggle_dialog(state, DialogState::CommandPalette::default()),
        Event::ToggleModelSelector => toggle_dialog(state, DialogState::ModelSelector::default()),
        
        // Command palette selection
        Event::PaletteSelect(name) => {
            if let Some(entry) = state.registry.get(&name) {
                if let Some(evt) = (entry.factory)("") {
                    update(state, evt);  // recurse
                }
            }
        }
        
        // All other events handled by existing modules
        _ => update_module(state, event),
    }
}
```

## Rationale

- **One place for state changes**: The update loop. No scattered handlers.
- **Testable**: Every action is `Event -> State change`. Pure Layer 1/2 tests.
- **Extensible**: New command = register name + factory. No new match arms.
- **Keybindings for free**: Any registered command gets a keybinding slot automatically.
- **Palette for free**: Iterate registry, filter by name/description, emit PaletteSelect.

## Boundaries

| Crate | Responsibility |
|-------|---------------|
| `runie-core` | `Event` enum, `AppState`, `CommandRegistry`, `DialogState`, update loop |
| `runie-tui` | Render functions for each `DialogState` variant |
| `runie-term` | Crossterm events → `Event` mapping, dialog input routing |

## Consequences

- **Positive**: Single flow for all actions. Trivial to test. Trivial to extend.
- **Positive**: Dialogs are first-class state. Can be inspected, persisted, snapshotted.
- **Trade-off**: Some events need args (SwitchModel needs provider+model). Factory pattern handles this.
- **Trade-off**: Recursive update for palette selections. Depth is 1, so stack-safe.
