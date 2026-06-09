# Session Commands

**Status**: done
**Milestone**: R2
**Category**: Sessions

## Description

All session-related commands registered in the CommandRegistry. Includes save, load, list, delete, naming, export, import, new, resume, compact, reset.

## Architecture

All session commands use the same pattern: factory function → Event → update handler.

```rust
// Factory functions in commands/handlers.rs

fn cmd_save(args: &str) -> Option<Event> {
    Some(Event::SaveSession { name: args.trim().to_string() })
}

fn cmd_load(args: &str) -> Option<Event> {
    Some(Event::LoadSession { name: args.trim().to_string() })
}

fn cmd_sessions(_args: &str) -> Option<Event> {
    Some(Event::ToggleSessionsDialog)
}

fn cmd_name(args: &str) -> Option<Event> {
    Some(Event::SetSessionName { name: args.trim().to_string() })
}

fn cmd_export(args: &str) -> Option<Event> {
    Some(Event::ExportSession { path: args.trim().to_string() })
}

fn cmd_import(args: &str) -> Option<Event> {
    Some(Event::ImportSession { path: args.trim().to_string() })
}

fn cmd_new(_args: &str) -> Option<Event> {
    Some(Event::NewSession)
}

fn cmd_resume(_args: &str) -> Option<Event> {
    Some(Event::ResumeSession)
}

fn cmd_compact(args: &str) -> Option<Event> {
    Some(Event::CompactSession { prompt: args.trim().to_string() })
}

fn cmd_reset(_args: &str) -> Option<Event> {
    Some(Event::ResetSession)
}
```

### Update Handlers

```rust
fn update_save(state: &mut AppState, name: &str) -> String {
    let session = state.to_session();
    match session::save(name, &session) {
        Ok(_) => format!("Session saved: {}", name),
        Err(e) => format!("Error: {}", e),
    }
}

fn update_load(state: &mut AppState, name: &str) -> String {
    match session::load(name) {
        Ok(session) => {
            state.from_session(session);
            format!("Session loaded: {}", name)
        }
        Err(e) => format!("Error: {}", e),
    }
}

fn update_new(state: &mut AppState) -> String {
    state.messages.clear();
    state.input_text.clear();
    state.input_cursor = 0;
    state.message_queue.clear();
    state.reset_to_defaults();
    "New session started".to_string()
}
```

### Session Struct Update

```rust
pub struct Session {
    pub name: String,
    pub display_name: Option<String>,     // NEW
    pub theme_name: String,               // NEW
    pub thinking_level: ThinkingLevel,    // NEW
    pub read_only: bool,                  // NEW
    pub created_at: f64,
    pub updated_at: f64,
    pub messages: Vec<ChatMessage>,
    pub provider: String,
    pub model: String,
}
```

## Acceptance Criteria

- [x] `/save <name>` — saves session to JSON
- [x] `/load <name>` — loads session from JSON
- [x] `/sessions` — lists saved sessions (inline; dialog deferred)
- [x] `/delete <name>` — deletes session file
- [x] `/name <display_name>` — sets display name (max 64 chars)
- [x] `/name` without args — shows current display name
- [x] `/export [filename]` — exports to JSON (default: `name_timestamp.json`)
- [x] `/import <path>` — imports from JSON file
- [x] `/new` — clears conversation, resets to defaults
- [x] `/resume` — loads most recent session by `updated_at`
- [x] `/compact [prompt]` — compacts context
- [x] `/reset` — clears messages, keeps provider/model
- [x] All commands registered in `CommandRegistry`
- [x] All commands persisted in `Session` struct

## Files

| File | Description |
|------|-------------|
| `crates/runie-core/src/commands/handlers.rs` | Session command factories |
| `crates/runie-core/src/update/session.rs` | Update handlers |
| `crates/runie-core/src/session.rs` | Updated `Session` struct |
| `crates/runie-core/src/model.rs` | `display_name`, `theme_name`, etc. |

## Tests

### Layer 1 — State/Logic
- [x] `save_creates_file` — save writes JSON
- [x] `load_restores_messages` — load brings back messages
- [x] `new_clears_messages` — new empties messages
- [x] `new_resets_provider` — new restores default provider
- [x] `name_sets_display_name` — display_name field updated
- [x] `name_truncates_long` — 100 char input → 64 + "…"
- [x] `export_creates_file` — export writes JSON
- [x] `import_loads_file` — import reads JSON
- [x] `roundtrip_save_load` — save then load preserves all fields

### Layer 2 — Event Handling
- [x] `slash_save_emits_event` — `/save foo` → SaveSession event
- [x] `slash_load_emits_event` — `/load foo` → LoadSession event
- [x] `slash_new_emits_event` — `/new` → NewSession event

### Layer 3 — Rendering
- [ ] `status_shows_display_name` — footer shows name when set
