# Round 4 — TUI Input, Rendering & Event Loop

## Findings

### 1. Direct `AppState` mutation inside the UI actor

Even though `UiActor` owns `AppState`, state changes should be event-driven:

- `crates/runie-tui/src/ui_actor.rs:378-379` — clears permission request directly.
- `crates/runie-tui/src/ui_actor.rs:597-606` — writes `input_mut()` directly for autocomplete triggers.
- `crates/runie-tui/src/ui_actor.rs:685` — clears open dialog directly.
- `crates/runie-tui/src/ui_actor.rs:678-679,695-696` — sets view scroll/dirty directly after command/form handling.

### 2. Duplicated input-event mapping

- `crates/runie-tui/src/main.rs:124-205` — `input_forwarder_task` maps keys to `InputMsg`.
- `crates/runie-tui/src/ui_actor.rs:358-466` — `handle_input_event` repeats the same mapping.

Only a subset of events is forwarded; the rest are handled twice. There should be one canonical converter.

### 3. Custom rendering code replaceable by crates

- `crates/runie-tui/src/popups/panel/form.rs:17-447` — custom form rendering. Replaceable with `tui-textarea`/`tui-input` for fields and `ratatui::widgets::List` for buttons.
- `crates/runie-tui/src/message/support.rs:196-247` — `wrap_styled_spans_for_blockquote` reimplements wrapping; use `textwrap`.
- `crates/runie-tui/src/ui/input.rs:64-286` — custom multi-line input box. Replaceable with `tui-textarea`/`tui-input`.
- `crates/runie-tui/src/status_bar.rs:66-70` — manual braille symbol extraction to mirror an inverted spinner. Use `throbber_widgets_tui` directly.

### 4. UI-side `agent_running` flag duplicates `TurnState`

- `crates/runie-tui/src/ui_actor.rs:271-344` — `agent_running` guard is a second copy of "is a turn running."

Derive this from `TurnState` events and an in-flight counter owned by `TurnActor`.

## Recommended changes

1. Route autocomplete, permission clearance, and dialog close through `InputActor`/`PermissionActor`/events, not direct field writes.
2. Have one canonical `Event → InputMsg` converter used by both the forwarder and `UiActor`.
3. Replace custom form/input rendering with `tui-textarea`/`tui-input`/`textwrap`/`throbber_widgets_tui`.
4. Derive `agent_running` from `TurnState` events.

## Task mapping

| Finding | Task file | Status |
|---------|-----------|--------|
| Route autocomplete through events/InputActor | `tasks/route-tui-autocomplete-through-inputactor-events.md` | **new** |
| Route permission clearance through PermissionActor | `tasks/route-permission-clearance-through-permissionactor.md` | **new** |
| Single input-event converter | `tasks/deduplicate-input-event-mapping-between-forwarder-and-uiactor.md` | **new** |
| Replace custom form rendering | `tasks/replace-custom-form-rendering-with-tui-textarea.md` | **new** |
| Use `textwrap` for blockquote | `tasks/use-textwrap-for-blockquote-word-wrap.md` | **new** |
| Replace custom input box | `tasks/replace-custom-input-box-with-tui-textarea.md` | **new** |
| Fix throbber inversion | `tasks/fix-throbber-inversion-and-use-throbber-widgets-tui.md` | **new** |
| Derive agent-running from TurnState | `tasks/derive-agent-running-flag-from-turnstate-events.md` | **new** |
