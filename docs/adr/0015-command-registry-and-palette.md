# Command Registry + Command Palette Architecture

## Context

pi has 22 slash commands and a command palette (Ctrl+P/Ctrl+L) that provides fuzzy-filtered access to all commands. runie currently uses a monolithic `match` statement in `slash.rs` for 8 commands.

We studied patterns from:
- **gptme** — decorator-based `@command(name, aliases, completer)` registry with `CommandContext`
- **crush** — `Commands` dialog with `FilterableList`, categorized tabs, `Action` enum per item
- **pi** — semantic keybinding names (`app.model.select`), migration system, keybinding-to-command mapping
- **aider** — `Commands` class with `cmd_*` methods, docstrings as descriptions

## Decision

### 1. Command Registry Pattern

Use a **registry with builder/decorator pattern** instead of a monolithic match:

```rust
// In runie-core — pure domain logic, no TUI
pub struct CommandRegistry {
    commands: HashMap<String, CommandDef>,
}

pub struct CommandDef {
    pub name: String,
    pub description: String,
    pub aliases: Vec<String>,
    pub handler: fn(&mut AppState, &str) -> CommandResult,
    pub completer: Option<fn(&str) -> Vec<String>>,
}

pub enum CommandResult {
    Message(String),           // Show message in chat
    Event(Event),              // Emit event to event loop
    OpenDialog(Dialog),        // Open a TUI dialog
    None,                      // Silent success
}
```

Registration at module load time (no runtime discovery needed):

```rust
impl CommandRegistry {
    pub fn builtins() -> Self {
        let mut reg = Self::new();
        reg.register(CommandDef {
            name: "model".into(),
            description: "Switch model".into(),
            aliases: vec!["m".into()],
            handler: cmd_model,
            completer: Some(complete_model),
        });
        // ... more commands
        reg
    }
}
```

### 2. Command Palette = Filterable Popup

The command palette is a **generic TUI component** that shows all registered commands:

```rust
// In runie-tui
pub struct CommandPalette {
    filter: String,
    selected: usize,
    items: Vec<CommandItem>,  // filtered from registry
}

pub struct CommandItem {
    pub name: String,
    pub description: String,
    pub shortcut: Option<String>,  // keybinding if any
}
```

- **Ctrl+P** opens the palette (not model cycling — see below)
- **Ctrl+L** opens model selector (dedicated dialog, not palette)
- Fuzzy filter as you type
- Arrow keys navigate, Enter selects
- Each item shows its keybinding if mapped

### 3. Keybindings Map to Commands

Keybindings don't duplicate command logic — they **invoke the same handlers**:

```rust
// keybindings.json
{
  "app.model.select": "ctrl+l",
  "app.session.save": "ctrl+s",
  "app.command.palette": "ctrl+p"
}

// Runtime: key event → action name → find command → execute handler
fn resolve_key_action(name: &str, state: &mut AppState) {
    match name {
        "app.model.select" => cmd_model_selector(state, ""),
        "app.session.save" => cmd_save(state, ""),
        "app.command.palette" => state.open_command_palette(),
        _ => {}
    }
}
```

### 4. Semantic Namespaces

Following pi's pattern, all actions use hierarchical names:

```
app.model.*          — model switching, cycling, selector
app.session.*        — save, load, new, resume, fork, tree
app.tool.*           — expand, read-only toggle
app.message.*        — follow-up, dequeue
app.editor.*         — external editor
app.thinking.*       — toggle, cycle level
app.command.*        — palette, help
```

### 5. Slash = One Entry Point

Slash parsing becomes trivial:

```rust
fn handle_slash(state: &mut AppState, input: &str) -> String {
    let input = input.trim_start_matches('/');
    let (name, args) = input.split_once(' ').unwrap_or((input, ""));
    
    match state.commands.get(name) {
        Some(cmd) => match (cmd.handler)(state, args) {
            CommandResult::Message(msg) => msg,
            CommandResult::Event(evt) => { state.update(evt); String::new() }
            CommandResult::None => String::new(),
            _ => String::new(),
        },
        None => format!("Unknown command: /{name}. Try /help."),
    }
}
```

### 6. TUI Dialogs for Complex Commands

Some commands open dedicated dialogs instead of immediate execution:

| Command | Behavior |
|---------|----------|
| `/model` | Opens model selector dialog (Ctrl+L) |
| `/settings` | Opens settings dialog (not just message) |
| `/tree` | Opens session tree dialog |
| `/sessions` | Opens session list dialog |

These return `CommandResult::OpenDialog(Dialog)` which the TUI layer handles.

## Rationale

- **Less code**: One registry instead of a growing match statement. Adding a command = one `reg.register()` call.
- **More power**: Commands get completers, aliases, keybinding integration for free.
- **Command palette for free**: The palette just iterates the registry — no manual listing.
- **Testable**: Each handler is a pure function `(&mut AppState, &str) -> CommandResult`. Layer 1 tests are trivial.
- **Extensible**: New commands added via `registry.register()` — no plugin system needed.

## Files

| File | Change |
|------|--------|
| `crates/runie-core/src/commands/` | New module: `mod.rs`, `registry.rs`, `handlers/` |
| `crates/runie-core/src/commands/handlers/` | One file per command category: `session.rs`, `model.rs`, `tool.rs`, `system.rs` |
| `crates/runie-core/src/event.rs` | Add dialog events |
| `crates/runie-core/src/model.rs` | Add `command_palette_open`, `dialog_stack` |
| `crates/runie-tui/src/ui.rs` | Render command palette popup |
| `crates/runie-term/src/keymap.rs` | Map semantic names to command invocations |

## Boundaries

- `runie-core` owns the registry, handlers, and `CommandResult` type. No TUI types.
- `runie-tui` owns `CommandPalette` widget rendering and dialog overlays.
- `runie-term` owns key event → semantic name → command invocation mapping.
