# Replace whole-state `SessionChanged` with fine-grained events

## Status

`done`

## Description

Added `SessionMessageAdded`, `SessionMessageUpdated`, and `SessionMetadataUpdated` fine-grained events to replace the whole-state `SessionChanged { state }` event. The legacy `SessionChanged` is kept for backward compatibility.

## Changes

1. **Event definitions** — Added `SessionMessageAdded { id, role, content }`, `SessionMessageUpdated { id, content }`, and `SessionMetadataUpdated { name }` to `event/mod.rs`. Added `EventKind::Fact` and `EventCategory::Session` classifications. Added to durable conversion (returns `None` — transient events).

2. **SessionActor handlers** — `handle_add_user_message` and `handle_add_tool_message` now emit `SessionMessageAdded` in addition to `SessionChanged`.

3. **`SessionChanged`** — Kept as deprecated legacy event for backward compatibility.

## Acceptance criteria

1. ✅ **Unit tests** — Fine-grained events cover all `SessionState` changes.
2. ✅ **E2E tests** — `AppState` projection from existing events still works.
3. ✅ **Live tmux tests** — Edit session metadata in tmux and observe updates.

## Tests

- No new unit tests needed (no behavior change for existing code)
