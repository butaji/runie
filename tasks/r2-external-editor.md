# External Editor (Ctrl+G)

**Status**: done
**Milestone**: R2
**Category**: Input & Commands

## Description

Open `$EDITOR` with current input text. On save+quit, read back into input.

## Architecture

```rust
// Command factory
fn cmd_external_editor(_args: &str) -> Option<Event> {
    Some(Event::OpenExternalEditor)
}

// Async command (returns Event via channel)
fn spawn_external_editor(text: String, event_tx: mpsc::Sender<Event>) {
    tokio::spawn(async move {
        let editor = std::env::var("EDITOR")
            .unwrap_or_else(|_| "vi".to_string());
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(text.as_bytes()).unwrap();
        
        let status = Command::new(&editor)
            .arg(tmp.path())
            .status()
            .await;
        
        if status.map(|s| s.success()).unwrap_or(false) {
            let content = tokio::fs::read_to_string(tmp.path()).await.unwrap_or_default();
            let _ = event_tx.send(Event::ExternalEditorDone { content }).await;
        }
    });
}
```

### Events

```rust
Event::OpenExternalEditor,           // Ctrl+G
Event::ExternalEditorDone { content: String },  // async result
```

### Keybinding

```json
{
  "app.editor.external": "ctrl+g"
}
```

## Acceptance Criteria

- [ ] `Ctrl+G` opens `$EDITOR` with input text
- [ ] Falls back to `vi` if `$EDITOR` not set
- [ ] Falls back to `notepad` on Windows
- [ ] Creates temp file
- [ ] On save+quit, reads back into input
- [ ] On quit-without-save, input unchanged
- [ ] Temp file cleaned up
- [ ] Non-blocking: uses async command

## Files

| File | Description |
|------|-------------|
| `crates/runie-core/src/event.rs` | `OpenExternalEditor`, `ExternalEditorDone` |
| `crates/runie-core/src/update/mod.rs` | Spawn editor, handle result |
| `crates/runie-term/src/keymap.rs` | Map Ctrl+G |

## Tests

### Layer 2 — Event Handling
- [ ] `ctrl_g_emits_open_editor` — keymap event
- [ ] `editor_done_updates_input` — ExternalEditorDone sets text

### Layer 4 — Smoke
- [ ] `external_editor_no_panic.sh` — tmux: Ctrl+G → :q → no panic
