# Dequeue (Alt+Up)

**Status**: done
**Milestone**: R2
**Category**: Input & Commands

## Description

Pop the last queued message back into the input field. Lets users edit or resubmit accidentally queued messages.

## Architecture

```rust
// Command factory
fn cmd_dequeue(_args: &str) -> Option<Event> {
    Some(Event::Dequeue)
}

// Update handler
fn update_dequeue(state: &mut AppState) {
    if let Some((content, _id)) = state.message_queue.last() {
        state.input_text = content.clone();
        state.input_cursor = state.input_text.len();
        state.message_queue.pop();
    } else {
        state.flash_input_border();
    }
}
```

### Keybinding

```json
{
  "app.message.dequeue": "alt+up"
}
```

## Acceptance Criteria

- [ ] `Alt+Up` pops last queued message into input
- [ ] Message removed from queue (not just copied)
- [ ] Cursor positioned at end
- [ ] Non-empty input is replaced
- [ ] Empty queue flashes input border
- [ ] Works for steering and follow-up queues

## Files

| File | Description |
|------|-------------|
| `crates/runie-core/src/event.rs` | `Dequeue` event |
| `crates/runie-core/src/update/queue.rs` | `update_dequeue()` |
| `crates/runie-term/src/keymap.rs` | Map Alt+Up |

## Tests

### Layer 1 — State/Logic
- [ ] `dequeue_restores_last` — last message → input, removed
- [ ] `dequeue_sets_cursor_end` — cursor at len
- [ ] `dequeue_replaces_input` — old input cleared
- [ ] `dequeue_empty_flashes` — flash when nothing to dequeue
- [ ] `dequeue_lifo` — multiple queued, pops in reverse

### Layer 2 — Event Handling
- [ ] `alt_up_emits_dequeue` — keymap event
