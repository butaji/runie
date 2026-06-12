# Command Registry — Unified Slash Commands + Command Palette

**Status**: done
**Milestone**: R2
**Category**: Core Architecture / Input & Commands

## Description

Replace the monolithic slash command `match` statement with a registry pattern. This unlocks the command palette, keybinding-to-command mapping, aliases, completers, and makes adding new commands a one-liner.

**Prior art studied:**
- **gptme** (`commands/base.py`): decorator registry with `CommandContext`, completers, aliases, auto-undo
- **crush** (`dialog/commands.go`): `FilterableList` dialog with categorized tabs, `Action` enum
- **pi** (`slash-commands.ts`, `keybindings.ts`): semantic names (`app.model.select`), migration system
- **aider** (`commands.py`): `Commands` class with `cmd_*` methods, docstrings as descriptions

## Architecture

### Registry

```rust
pub struct CommandRegistry {
    commands: HashMap<String, CommandDef>,
    aliases: HashMap<String, String>,
}

pub struct CommandDef {
    pub name: String,
    pub description: String,
    pub aliases: Vec<String>,
    pub category: CommandCategory,
    pub handler: CommandHandler,
    pub completer: Option<CommandCompleter>,
}

pub type CommandHandler = fn(&mut AppState, &str) -> CommandResult;
pub type CommandCompleter = fn(&str) -> Vec<String>;

pub enum CommandResult {
    Message(String),
    Event(Event),
    OpenDialog(Dialog),
    None,
}

pub enum CommandCategory {
    Session,
    Model,
    Tool,
    System,
    Help,
}
```

### Built-in Commands

| Name | Aliases | Category | Description |
|------|---------|----------|-------------|
| `model` | `m` | Model | Switch model (opens selector) |
| `scoped-models` | — | Model | Enable/disable models for cycling |
| `save` | — | Session | Save session |
| `load` | — | Session | Load session |
| `sessions` | — | Session | List sessions |
| `delete` | — | Session | Delete session |
| `name` | — | Session | Set session display name |
| `export` | — | Session | Export session to JSON |
| `import` | — | Session | Import session from JSON |
| `new` | — | Session | Start new session |
| `resume` | — | Session | Resume most recent session |
| `compact` | — | Session | Compact context |
| `reset` | — | Session | Reset conversation |
| `readonly` | `ro` | Tool | Toggle read-only mode |
| `copy` | — | System | Copy last response to clipboard |
| `settings` | — | System | Open settings dialog |
| `reload` | — | System | Reload config, keybindings, themes |
| `changelog` | — | System | Show changelog |
| `hotkeys` | — | System | Show all keyboard shortcuts |
| `help` | `h`, `?` | Help | Show available commands |
| `quit` | `q`, `exit` | Help | Quit application |
| `history` | — | Session | Show recent history |
| `theme` | — | System | Switch theme or list available themes |

### Slash Parsing

```rust
fn handle_slash(state: &mut AppState, input: &str) -> Option<CommandResult> {
    let input = input.trim_start_matches('/');
    let (name, args) = input.split_once(' ').unwrap_or((input, ""));
    match state.registry.get(name) {
        Some(cmd) => Some((cmd.handler)(state, args)),
        None => Some(CommandResult::Message(format!("Unknown command: /{name}. Try /help."))),
    }
}
```

## Acceptance Criteria

- [x] `CommandRegistry` with `register()`, `get()`, `list()` methods
- [x] `CommandDef` with name, description, aliases, category, handler, completer
- [x] `CommandResult` enum: Message, Event, OpenDialog, None
- [x] All existing slash commands migrated to registry
- [x] All 22+ pi-equivalent commands registered
- [x] Aliases work: `/m` → `/model`, `/q` → `/quit`, `/h` → `/help`
- [x] `/help` iterates registry to generate command list
- [x] `CommandCategory` groups commands for palette display
- [x] Registry is `Send + Sync`, constructed once at startup
- [x] Zero runtime cost: handler is `fn` pointer, not `Box<dyn>`

## Files

| File | Change |
|------|--------|
| `crates/runie-core/src/commands/mod.rs` | Registry + `CommandDef` + `CommandResult` |
| `crates/runie-core/src/commands/handlers/session.rs` | `/save`, `/load`, `/sessions`, `/delete`, `/name`, `/export`, `/import`, `/new`, `/resume`, `/compact`, `/reset`, `/history` |
| `crates/runie-core/src/commands/handlers/model.rs` | `/model`, `/scoped-models` |
| `crates/runie-core/src/commands/handlers/tool.rs` | `/readonly` |
| `crates/runie-core/src/commands/handlers/system.rs` | `/copy`, `/settings`, `/reload`, `/changelog`, `/hotkeys`, `/theme` |
| `crates/runie-core/src/commands/handlers/help.rs` | `/help`, `/quit` |
| `crates/runie-core/src/update/slash.rs` | Replaced by registry dispatch |

## Tests

### Layer 1 — State/Logic
- [x] `registry_get_by_name` — `get("model")` returns CommandDef
- [x] `registry_get_by_alias` — `get("m")` resolves to `model`
- [x] `registry_list_returns_all` — `list()` returns 22+ commands
- [x] `registry_list_groups_by_category` — categories preserved
- [x] `handler_model_switches` — `/model gpt-4o` updates state
- [x] `handler_help_generates_list` — `/help` lists all commands
- [x] `handler_quit_sets_flag` — `/quit` sets `should_quit`
- [x] `unknown_command_returns_error` — `/foo` → error message

### Layer 2 — Event Handling
- [x] `slash_event_dispatches_to_registry` — `Event::Submit` with `/model` calls handler
- [x] `alias_event_dispatches_correctly` — `/m` calls model handler
