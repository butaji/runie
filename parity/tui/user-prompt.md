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
