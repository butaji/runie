# TUI history-loaded event handling

## Objective

Fix the TUI so pressing `UP` in an empty input recalls persisted session history. Currently pressing `UP` in an empty input does nothing because `UiActor` does not handle `Event::HistoryLoaded`.

## Root cause

`runie/crates/runie-core/src/actors/session/ractor_session_actor.rs` emits `Event::HistoryLoaded` when history is loaded from disk, but `runie/crates/runie-tui/src/actors/ui/actor.rs` (`UiActor::handle_event_inner()`) has no branch for this event. The `InputActor` never receives `InputMsg::HistoryLoaded`, so input history recall via `UP` is broken for persisted sessions.

## Required runie changes

1. In `UiActor::handle_event_inner()`, add a branch for `Event::HistoryLoaded`.
2. Forward the event to `InputActor` by sending `InputMsg::HistoryLoaded(history)`.
3. In `InputActor`, handle `InputMsg::HistoryLoaded` to populate the input history buffer.
4. Verify `tests/dialog_navigation.rs::up_arrow_recalls_persisted_history_in_empty_input` passes.

## Test scenario

1. **Persisted history recall via UP**
   - `AppTest::mock().with_history(vec!["persisted command"])`.start()
   - Press `UP` with empty input
   - Assert: input contains "persisted command"

## Dependencies

- `session_management`

## Acceptance checklist

- [ ] `UiActor::handle_event_inner()` branches on `Event::HistoryLoaded`
- [ ] `InputActor` handles `InputMsg::HistoryLoaded`
- [ ] `up_arrow_recalls_persisted_history_in_empty_input` passes
- [ ] No `sleep()` in the test
