# Provider Attribution

**Status**: todo
**Milestone**: R2
**Category**: TUI Rendering / Core

## Description

Show which provider served each response.

## Architecture

```rust
pub struct ChatMessage {
    // ... existing ...
    pub provider: String,
}
```

### Display

```
→ claude-3-5-sonnet (anthropic) · 2.3s
  Here's the code...
```

## Acceptance Criteria

- [ ] `ChatMessage.provider` field
- [ ] Provider shown in message header
- [ ] Persisted in `Session`
- [ ] Defaults to current provider

## Files

| File | Description |
|------|-------------|
| `crates/runie-core/src/model.rs` | `provider: String` on ChatMessage |
| `crates/runie-core/src/session.rs` | Persist provider |
| `crates/runie-tui/src/ui.rs` | Show provider in header |

## Tests

### Layer 1
- [ ] `message_stores_provider` — field set on response
- [ ] `session_persists` — save/load keeps it

### Layer 3
- [ ] `message_shows_provider` — header includes name
