# p53 — Runtime resize capture contract

Status: Runie runtime resize evidence complete; paired Grok cell parity remains
pending (2026-08-09)

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

Runtime re-probe (2026-08-08): the private tmux/asciinema driver completed a
Runie capture with `80x12` at 250 ms and `100x24` at 500 ms, producing valid
cast, raw, settled ANSI, manifest, and resize-report artifacts. The paired
Grok capture still needs to be rerun with the same schedule before claiming
cell-level resize parity.

Fresh local matrix probe (2026-08-08): `capture-scenario.sh` replayed
`visual-resize.yaml` across all four standard initial geometries (`62x32`,
`80x24`, `100x30`, and `120x36`). Every `.resize.json` report was valid and
observed both requested transitions (`80x12` at 250 ms and `100x24` at
500 ms). This strengthens the Runie-side runtime evidence; the paired Grok
capture and cell comparison remain open.

Paired Grok probe attempt (2026-08-08): the source checkout's ignored PTY
resize test was invoked, but Cargo stalled while fetching its external
`async-openai` git dependency before the test binary built. The process was
stopped without producing a Grok artifact. This is an environment/dependency
fetch blocker and provides no evidence for or against Grok resize behavior.

Paired schedule probe (2026-08-08): both Runie and Grok independently recorded
the same two observed geometries. Runie reached its settled frame; Grok's
settled-response probe timed out after the resize, so its cast is marked
invalid and was not used for a cell comparison. This isolates the remaining
issue to obtaining a settled Grok frame under the resize schedule, not to the
resize event or observation path.

Grok dependency retry (2026-08-08): `CARGO_NET_GIT_FETCH_WITH_CLI=true cargo
fetch` successfully retrieved the external git revisions and the PTY test
workspace compiled. The targeted `pty_e2e_scroll_selection`
`resize_preserves_scroll_position` test then produced no test-start or cast
artifact output and was stopped after hanging in the PTY execution phase.
Dependency access is therefore resolved, but Grok PTY settlement remains an
environment blocker; no paired resize claim is made.

Runie four-geometry runtime matrix (2026-08-09):
`capture-scenario.sh crates/runie-tui/tests/e2e/visual-resize.yaml` completed
for initial terminals `62×32`, `80×24`, `100×30`, and `120×36` with
`--terminal-native` and truecolor. Every `.resize.json` report was valid and
observed `80×12` at 250 ms followed by `100×24` at 500 ms; each run also
produced cast, raw, settled ANSI, metadata, and Grok-diagnostic artifacts.
This closes the Runie runtime schedule/observation evidence. It does not claim
the remaining paired Grok cell comparison, whose settled-frame capture is
still unavailable in the current checkout.

Comparator validation increment (2026-08-08): `cast_compare` now reads each
capture's resize report when present and rejects invalid, incomplete, or
geometry-mismatched observations against the declared schedule. The pure
validator is regression-tested; paired Grok settled-frame capture remains the
only external parity gap.
