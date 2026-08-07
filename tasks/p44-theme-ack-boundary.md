# p44 — Replace theme polling with actor acknowledgements

Status: in progress

`App::set_theme` currently publishes `ThemeChanged` and performs bounded
`yield_now` polling until prompt, status, and scrollback snapshots converge.
The projections themselves are actor-owned and mailbox-backed, but the
application-level completion signal is still inferred from snapshots.

The next design should make theme application an explicit event command with
an acknowledgement barrier owned by the coordinator. It must preserve one
event source for live bus projections, avoid duplicate reductions, and return
only after all three actor acknowledgements. The replacement should use
mailbox/oneshot delivery, not polling or timing assumptions, and should gain a
YAML event/state assertion.

## First implementation slice (2026-08-06)

`App::set_theme` now sends the typed `ThemeChanged` event to the prompt,
status, and scrollback actors through acknowledged mailboxes. It returns only
after all three owners reduce the event; no snapshot polling remains. The
event source stays declarative and each actor retains exclusive state
ownership.

Status is complete: the acknowledgement barrier is now the authoritative
completion signal for theme application, with actor snapshots used only as
read-only projections afterward.
