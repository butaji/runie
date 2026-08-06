# p25 — Parity instrument audit

## Review role

This task is the independent review pass for parity work: every implementation
change must be checked against the capture/replay instruments before being
called parity.

## Current instruments

- `scripts/tmux-asciinema-capture.sh`: private tmux PTY, fixed geometry,
  bounded prompt/completion probes, `.cast` plus raw replay stream.
- `scripts/capture-matrix.sh`: 62×32, 80×24, 100×30, and 120×36 matrix.
- `scripts/compare-matrix.sh`: paired Grok/Runie matrix gate over all four
  geometries; each pair must pass the full-cell `cast_compare` check. It now
  reports every geometry before returning failure, so one mismatch cannot hide
  evidence from the remaining viewport sizes.
- `cast_compare --dump`: VT replay with glyph, fg, bg, bold, italic,
  underline, inverse, geometry, and full-cell JSON.
- YAML `reference.exact_screen`: strict reference-frame symbol oracle.
- YAML `reference.exact_attributes`: strict style/color oracle.

The paired matrix is an evidence tool, not a fixture substitute. Reference
casts must come from the same scenario and capture run; dynamic usage, timing,
and model output differences must be investigated before snapshots are
updated.

Fresh matrix capture audit (2026-08-05) produced valid paired casts at all
four geometries. Full-cell differences were: 62×32 — 182 cells; 80×24 — 226;
100×30 — 191; 120×36 — 194. Each report includes glyph and attribute counts;
the dominant deltas remain provider response/wrapping, reasoning placement,
usage/timestamps, and completion timing. The matrix now proves that these
differences persist across width and height rather than being a single-
viewport artifact.

The apparent full-mode user-vpad offset was experimentally moved from before
to after the user row. A fresh 80×24 run improved the live diff from 226 to
218 cells, but it failed the locked `grok-rich.cast` frame-81 exact oracle by
moving the deterministic user prompt one row too early. The change was
reverted; the cast-backed oracle remains authoritative until frame selection
and live capture phase are reconciled.

ANSI stream audit (2026-08-05) then confirmed Runie emits `SGR 1` around both
ready-footer shortcut labels. The comparator still reports those cells as
non-bold in its selected frame, so the 20 attribute-only footer differences
are now classified as a VT replay/frame-selection issue, not evidence that the
theme-token styling is absent. The raw stream must be reconciled with the
selected settled frame before changing widget styling.

The ready footer is now rendered only once in the live binary: the ready status
projection is not painted and then overwritten by the binary footer. The footer
segments are written directly with Opaline-derived styles, avoiding Ratatui
style-diff transitions caused by painting the same row twice. A fresh 80x24
capture reduced footer attribute differences from 25 to 9; glyph, timing, and
provider-content differences were unchanged. The remaining nine cells are
isolated to the first bold shortcut label and remain a separate backend-style
investigation.

The cast comparator no longer caches an arbitrary “last non-empty” frame. It
now compares the final application screen after replaying all output up to
alternate-screen exit, while retaining empty/trailing cells and attributes.
This removes a source of false style diagnostics without weakening exactness.

## Review findings

### Fresh 62×32 paired capture — 2026-08-05

The installed Grok binary and `just tui` were captured independently with the
same tmux/asciinema harness and prompt (`Hey`):

- Grok: `/tmp/runie-grok-matrix-62x32.cast`
- Runie: `/tmp/runie-runie-matrix-62x32.cast`
- command: `just cast-compare` (full 62×32 cell grid)
- result: 190 differing cells — 170 glyph differences and 20 attribute-only
  differences

The row diagnostics identify the remaining live mismatch classes precisely:
provider-generated answer text and wrapping, reasoning-row placement, dynamic
usage (`15K` versus `0`), prompt timestamps, and elapsed time. The comparison
does not support claiming pixel parity for this live run. Deterministic exact
parity requires the same response/usage/time inputs, or a reference contract
that explicitly freezes those values; color and blank-cell attributes are now
being compared rather than silently ignored.

The six legacy source-backed Insta snapshots were refreshed after this audit.
Their diffs were the intentional vpad/blank-row projection already covered by
the strict Grok YAML and full-cell checks; no reference cast or attribute
assertion was weakened.

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

8. The declarative `visual-hey.yaml` scenario is now replayed through the
   Runie renderer at `62x32`, `80x24`, `100x30`, and `120x36`. Each matrix
   case verifies dimensions and the rendered user cursor, ensuring the
   terminal-height path is exercised across the same capture sizes used by
   `scripts/capture-matrix.sh`.

9. The workspace run after execute-card and geometry changes passes all core,
   TUI unit, YAML replay, and cast-backed tests. Six legacy Insta snapshots
   still fail only on the source-backed user-vpad row shift; their generated
   `.snap.new` files were discarded. These references require fresh same-run
   Grok captures before acceptance, so they remain strict and unchanged.

## Status

In progress. Full-cell dumps, row diagnostics, and phase-locked frame
selection are implemented. `visual-grok-feed` is now locked to Grok output
frame 81; its mismatch remains visible as a real parity failure.
