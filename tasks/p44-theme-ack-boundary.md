# p44 — Replace theme polling with actor acknowledgements

Status: planned

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
