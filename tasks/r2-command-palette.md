# Command Palette

**Status**: todo
**Milestone**: R2
**Category**: TUI Rendering

## Description

A fuzzy-filtered popup that lists all registered commands. The primary way users discover actions. Opened with `Ctrl+P` (or `/` in the input field).

**Design inspiration:**
- **Crush**: `FilterableList` with text input, categorized items, keybinding display
- **VS Code**: `Cmd+Shift+P` — canonical command palette
- **pi**: Ctrl+P opens "Commands" dialog

## Architecture

### Dialog State

```rust
// In AppState.dialog_stack: Vec<DialogState>
pub enum DialogState {
    CommandPalette {
        filter: String,
        selected: usize,
    },
    // ... other dialogs
}
```

### Events

```rust
Event::ToggleCommandPalette,     // Ctrl+P
Event::PaletteFilter(char),      // typed while open
Event::PaletteBackspace,         // backspace in filter
Event::PaletteUp,
Event::PaletteDown,
Event::PaletteSelect,            // Enter — emits the selected command's Event
Event::PaletteClose,             // Esc
```

### Selection Logic

```rust
fn palette_select(state: &mut AppState) {
    let DialogState::CommandPalette { filter, selected } = state.dialog_top() else { return };
    let items = filter_commands(&state.registry, filter);
    let Some(entry) = items.get(*selected) else { return };
    
    state.dialog_close();  // close palette first
    if let Some(evt) = (entry.factory)("") {
        state.update(evt);   // then execute
    }
}
```

### Visual Design

```
┌─ Commands ───────────────────────────────────┐
│ > comp                                       │
├──────────────────────────────────────────────┤
│  Model                                       │
│   compact        Compact context              │
│  ──────────────────────────────────────────  │
│  Session                                      │
│   save           Save session           Ctrl+S│
│   load           Load session                 │
│  ──────────────────────────────────────────  │
│  System                                       │
│   copy           Copy last response           │
└──────────────────────────────────────────────┘
```

- Centered popup, 60 chars wide, max 18 lines
- Filter bar at top with `>` prompt
- Items grouped by category
- Selected item: accent background
- Keybinding shown right-aligned (if any)

### Filter Algorithm

```rust
fn filter_commands(registry: &CommandRegistry, query: &str) -> Vec<&CommandEntry> {
    let q = query.to_lowercase();
    registry.list()
        .into_iter()
        .filter(|e| {
            e.name.to_lowercase().contains(&q) ||
            e.description.to_lowercase().contains(&q)
        })
        .collect()
}
```

## Acceptance Criteria

- [ ] `Ctrl+P` opens palette overlay
- [ ] Typing filters commands in real-time
- [ ] Arrow Up/Down navigates (wraps at boundaries)
- [ ] Enter executes selected command
- [ ] Esc closes without action
- [ ] Background dimmed while open
- [ ] Shows command name, description, and keybinding
- [ ] Groups by category with headers
- [ ] Empty filter shows all commands
- [ ] No matches shows "No commands found"

## Files

| File | Lines | Description |
|------|-------|-------------|
| `crates/runie-core/src/model.rs` | +20 | Add `DialogState::CommandPalette` |
| `crates/runie-core/src/event.rs` | +6 | Palette events |
| `crates/runie-core/src/update/dialog.rs` | ~80 | Toggle, filter, select, close |
| `crates/runie-tui/src/ui.rs` | ~120 | `render_command_palette()` |
| `crates/runie-term/src/keymap.rs` | +1 | Map Ctrl+P |

## Tests

### Layer 1 — State/Logic
- [ ] `filter_empty_shows_all` — no filter = all commands
- [ ] `filter_matches_name` — "comp" matches "compact"
- [ ] `filter_matches_description` — "copy" matches "Copy last response"
- [ ] `filter_case_insensitive` — "COMP" matches "compact"
- [ ] `select_wraps_up` — Up at 0 → last
- [ ] `select_wraps_down` — Down at last → 0

### Layer 2 — Event Handling
- [ ] `toggle_opens_palette` — ToggleCommandPalette pushes dialog
- [ ] `select_closes_then_executes` — Enter closes, then emits command Event
- [ ] `close_pops_dialog` — Esc removes from stack

### Layer 3 — Rendering
- [ ] `palette_renders_centered` — TestBackend shows popup
- [ ] `palette_shows_categories` — headers visible
- [ ] `palette_highlights_selected` — accent bg on selected row
- [ ] `palette_shows_keybinding` — shortcut right-aligned

### Layer 4 — Smoke
- [ ] `palette_no_panic.sh` — tmux: Ctrl+P → type → Enter → Esc

## Notes

- **Palette executes commands, not functions**. It calls `entry.factory("")` to get an Event, then feeds it back into the update loop. No special-casing.
- **Dialog stack is Vec**. Multiple dialogs can be open (palette → model selector). Esc closes the top one.
- **Input events route to top dialog**. When `dialog_stack` is non-empty, input goes to `dialog_top().handle_event()` instead of the editor.
