# User prompt block

## Anatomy

`❯` pointer, prompt body, optional timestamp overlay, wrapping, and Grok
vertical padding when full mode is enabled.

## States

- empty
- typed
- multiline
- history browsing
- timestamped submitted prompt
- compact mode

## Events

Prompt input is owned by the prompt actor; submitted text becomes an event and
the scrollback projection appends a `User` line. Views never mutate core state.

## Acceptance

Use `visual-typed.yaml`, `visual-multiline.yaml`, and the settled `Hey` cast.

## Source-backed vpad rule

Grok's user block uses two vertical padding rows only when the prompt is
non-compact. System, session-event, tool, and activity blocks explicitly
disable vpad. Runie must model this as entry metadata rather than filtering
generic empty `System` rows, because those rows can belong to different
neighboring blocks.
