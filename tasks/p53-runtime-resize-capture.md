# p53 — Runtime resize capture contract

Status: implemented in capture tooling; runtime parity evidence pending
(2026-08-08)

## Why

The live input worker already receives `InputConfig::ScrollViewport` through
an owned mailbox and applies the new viewport to `ScrollFlushState`. Existing
YAML fixtures prove the pure flush reducer and existing asciinema captures
prove fixed-size layouts, but neither proves a terminal resize during a
flooded mouse stream.

## Required declarative contract

The capture tooling now accepts this YAML without recompilation:

```yaml
capture:
  prompt: Hey
  resize:
    - at_ms: 250
      cols: 80
      rows: 12
    - at_ms: 500
      cols: 100
      rows: 24
```

The capture driver must schedule private-tmux `resize-window` operations only
for the isolated capture session. The cast metadata must record each resize,
and the scenario must remain invalid if the requested geometry is not
observed in the captured pane.

The schedule is threaded through `capture-scenario.sh`, `capture-matrix.sh`,
and the private tmux driver. Each resize is checked with tmux's observed
window geometry and recorded in a `.resize.json` artifact and the cast
manifest; a mismatch fails the capture.

## Acceptance

- Grok and Runie are captured with the same resize schedule and truecolor
  terminal settings.
- ANSI/asciinema frame comparison includes frames before, during, and after
  each resize.
- The input worker's resulting scroll flushes show the viewport-dependent cap
  transition without a dropped state update.
- No sleeps are introduced into Rust tests; YAML timing is driven by the
  external capture driver only.

Until this exists, fixed-size visual snapshots must not be described as proof
of runtime resize parity.
