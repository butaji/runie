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

## Live tooling validation (2026-08-08)

The first successful PTY validation used the supported terminal-native command:

```sh
scripts/tmux-asciinema-capture.sh 80 24 /tmp/resize/80x24.cast \
  'target/debug/runie --terminal-native' resize C-q '' \
  '250,80,12;500,100,24'
```

The private session produced valid cast/raw/settled/manifest artifacts and
`80x24.resize.json` recorded both requested geometries (`80×12` at 250 ms and
`100×24` at 500 ms). This proves the resize schedule and observation plumbing;
paired Grok-vs-Runie cell comparisons during the same schedule remain the
final parity evidence.

The settled probe now accepts Grok's compact-width `Enter:send` and
`Type your message` footer variants in addition to the full-mode `Worked for`
marker, while retaining the `Esc:cancel` active-turn exclusion. This removes a
false timeout source when a resize changes footer vocabulary.

Paired schedule probe (2026-08-08): both Runie and Grok independently recorded
the same two observed geometries. Runie reached its settled frame; Grok's
settled-response probe timed out after the resize, so its cast is marked
invalid and was not used for a cell comparison. This isolates the remaining
issue to obtaining a settled Grok frame under the resize schedule, not to the
resize event or observation path.
