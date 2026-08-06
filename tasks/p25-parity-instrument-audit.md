# p25 — Parity instrument audit

## Review role

This task is the independent review pass for parity work: every implementation
change must be checked against the capture/replay instruments before being
called parity.

## Current instruments

- `scripts/tmux-asciinema-capture.sh`: private tmux PTY, fixed geometry,
  bounded prompt/completion probes, `.cast` plus raw replay stream.
- `scripts/capture-matrix.sh`: 62×32, 80×24, 100×30, and 120×36 matrix.
- `cast_compare --dump`: VT replay with glyph, fg, bg, bold, italic,
  underline, inverse, geometry, and full-cell JSON.
- YAML `reference.exact_screen`: strict reference-frame symbol oracle.
- YAML `reference.exact_attributes`: strict style/color oracle.

## Review findings

1. Same-run Grok/Runie captures are required for dynamic clock, usage, model
   response, and elapsed fields; stale casts cannot prove pixel parity.
2. Reference frame selection must be phase-specific. `frame_contains` alone
   can select an earlier/later frame with the same activity marker.
3. YAML now supports an explicit zero-based `frame_index`; it takes precedence
   over broad marker matching. Diagnostics include expected/actual complete
   rows and attributes.
4. No visual assertion may be weakened to make the workspace gate green.

5. The committed compact-autoscroll change was rechecked after reverting an
   experimental full-mode spacing adjustment: the YAML fixture-discovery gate
   passes, while the independent `visual_snapshots` suite still reports the
   known full-mode spacing/resize differences. This keeps the two evidence
   classes separate instead of treating a narrow YAML pass as global parity.

6. A full-mode spacing experiment was rejected by the instruments: it reduced
   several legacy snapshot diffs but moved the locked `grok-rich.cast` activity
   frame and broke the narrow turn-summary gutter unit test. The source remains
   unchanged, and the 111-test unit gate plus YAML replay gate are green again.

7. The full `visual_snapshots` run currently fails six strict fixtures
   (`submitted`, `reasoning`, `error`, `tool`, `resize`, and `scroll`). Their
   shared diff is an extra user-entry vertical-padding pair that shifts the
   prompt/footer rows; the content rows remain present. Generated `.snap.new`
   files were discarded, and no reference snapshot was weakened or re-recorded.
   This isolates terminal-height-aware vpad projection as the next fix.

## Status

In progress. Full-cell dumps, row diagnostics, and phase-locked frame
selection are implemented. `visual-grok-feed` is now locked to Grok output
frame 81; its mismatch remains visible as a real parity failure.
