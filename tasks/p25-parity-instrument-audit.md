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
- `cast_compare --frames`: opt-in indexed frame replay. It retains the full
  cell/style grid after every output event and reports frame counts plus the
  first corresponding-frame divergence, making timing/frame alignment visible
  without weakening the final-screen gate.
  Consecutive events with an unchanged complete VT grid are coalesced, so the
  count represents visible terminal-state transitions rather than transport
  chunks; no cell is discarded from a retained frame.
  Indexed mismatches now also report the first differing frame cell and both
  complete cell attributes. The current 80x24 pair starts with Grok blank at
  `(0,0)` and Runie already showing the header (`c`, bold) there, confirming
  that marker-locked phase alignment must precede feed parity comparison.
- `cast_compare --frames-after=MARKER`: starts indexed replay independently at
  the first visible frame containing `MARKER`, removing startup-frame skew
  before comparing the same scenario phase.
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

The indexed frame comparator was exercised against the existing 80x24 pair:
Grok produced 67 output frames and Runie 15, with the first corresponding
full-cell divergence at frame 2. This confirms that a future exact frame oracle
must align semantic application phases or capture timing before asserting
one-to-one frame identity.

The same indexed probe across the four saved matrix geometries reports:

| geometry | Grok frames | Runie frames | first indexed divergence |
| --- | ---: | ---: | ---: |
| 62x32 | 62 | 23 | 1 |
| 80x24 | 67 | 14 | 1 |
| 100x30 | 98 | 19 | 1 |
| 120x36 | 66 | 20 | 1 |

The mismatch is therefore capture-phase-wide, not a single viewport defect.

**Strict saved-cast probe (2026-08-06):** Replayed the preserved 62×32
same-run pair with `just cast-compare`. The comparator examined all 1,984
cells and reported 190 differences: 170 glyph differences and 20
attribute-only differences. The attribute coordinates are concentrated in the
footer shortcut row (Grok bold versus Runie non-bold), while the glyph rows
remain dominated by response text, reasoning placement, timestamps, usage,
and elapsed time. This is a real non-parity result, not an attribute-checking
gap; no fixture was weakened.

**Footer cell-width correction (2026-08-06):** The live binary's ready footer
advanced its write cursor with UTF-8 byte length, so the three-byte `│`
separator shifted subsequent segments by two terminal cells. The cursor now
advances by character cell count, and a binary regression asserts `Ctrl+x`
starts at the expected cell with bold styling. Full CI and all visual checks
remain green; a fresh same-run cast is still required to remeasure the old
saved-cast delta.

The live binary's deterministic placeholder provider now uses the recorded
Grok `Hey` answer text and a `15K` total-token `Done` usage payload. These
values flow through the normal provider/core event path; the renderer does not
special-case the text or mutate status state. This removes provider-content
and usage nondeterminism from the controlled parity scenario, leaving timing,
timestamps, wrapping, and footer style as independently measurable deltas.

**Fresh Runie recapture (2026-08-06):** A new isolated 62×32 Runie cast was
captured after the footer cell-width correction:
`/tmp/runie-runie-capture.VUt9J4/runie-hey.cast`. Compared with the preserved
Grok reference using the strict full-cell comparator, it reports 246 differing
cells (231 glyph differences and 15 attribute-only differences). The footer
separator spacing is now aligned; the remaining footer attributes are the
Grok-bold versus Runie-regular shortcut labels. The other deltas are visible
scenario differences: the captured Runie response is longer and wraps onto
different rows, its reasoning and completion rows occur later, and its prompt
timestamp is live rather than matching the reference. This confirms the
instrument catches real scenario differences and does not establish pixel
parity from a stale cast pair.

The raw recapture also confirms the footer diagnostic is serialization-level:
the live stream contains `SGR 1`, followed by a reset before the label text,
while the in-memory buffer regression still marks the cells bold. A focused
theme-token projection experiment and a full `Paragraph` span render produced
the same ANSI sequence and the same 15 attribute-only cells, so that change
was discarded. The remaining footer issue belongs in the terminal-diff
serialization/capture instrument, not in a hardcoded color or token change.

**Assistant gutter correction (2026-08-06):** The same fresh `Hey` cast exposed
that the completed-assistant timestamp branch split at the raw width boundary,
so a long first line could place the clock in the middle of a word. The
projection now reserves Grok's timestamp gutter, wraps at a word boundary, and
continues the remainder as normal feed text. The placeholder stream was also
corrected to the exact recorded Grok answer. The strict 62×32 probe changed
from 246 to 223 differing cells; the response text now matches, while the
remaining differences are feed vertical phase, live clock/elapsed values,
header usage, and footer ANSI attributes. A unit test locks the no-midword
timestamp invariant.

**Assistant-start row experiment (2026-08-06):** Full-cell row dumps showed a
duplicated blank separator before the thought summary in the isolated `Hey`
cast. Removing that separator reduced the isolated strict probe from 223 to
75 differing cells and aligned the complete answer block, but the workspace
YAML replay gate rejected the broad event-level change: the recorded tool-feed
frame then shifted by one row. The change was reverted. The result identifies
the next fix as a scenario-aware projection difference, not permission to
weaken the shared event mapping or its Grok tool fixture.

**No-tool-only separator projection experiment (2026-08-06):** Moving the
deduplication into `physical_rows` made the isolated 62×32 `Hey` cast report
75 differing cells (60 glyph, 15 attribute-only), with the entire assistant
response block matching Grok symbol-for-symbol. However, the generic rule
failed three strict full-mode Insta snapshots even though the YAML tool-feed
fixtures passed. It was reverted; no snapshot was regenerated. The next fix
must use an explicit scenario/state discriminator rather than infer behavior
from the absence of tool rows.

**Grok timing ownership audit (2026-08-06):** The authoritative Grok tracker
(`xai-grok-pager/src/acp/tracker.rs`) stores `last_thinking_elapsed_ms`, derives
it from server timestamps attached to thinking chunks, and freezes it when the
thinking block finishes. Empty pre-created thinking blocks are removed rather
than rendered as a fake `0.0s` summary. Runie's `AgentEvent::MessageUpdate` and
`MessageEnd` contracts currently carry no equivalent timing metadata, forcing
the renderer's `0.9s` placeholder and causing the remaining `0.2s`/`0.9s`
strict-cast mismatch. The next parity change belongs in the core event DSL and
actor-owned state projection, with YAML replay values for server timestamps;
the renderer should consume the resulting projection rather than hardcode a
duration.

## Review findings

### Phase-marker validation — 2026-08-06

`cast_compare --frames-after=MARKER` now fails explicitly when the requested
marker is absent from either cast, including the cast path and marker text.
Previously that case produced an empty zero-frame comparison, which could hide
an invalid phase selection behind an ordinary mismatch. The full workspace
gate remains green after this diagnostic hardening.

The comparator now has direct binary tests for both successful marker
selection and missing-marker failure, using the checked-in Grok cast. This
keeps the phase-selection contract executable rather than relying only on
manual CLI probing.

The current broad-marker probe (`--frames-after=❯`) over the checked-in full
casts reports an immediate real divergence: Grok/Runie retain 62/121 visible
frames, and the first differing cell is `(2,0)` (blank versus `m`). This is
evidence that the marker must identify a semantic scenario boundary before
indexed frames can be compared one-to-one; it is not a parity pass.

Phase markers may now be occurrence-qualified as `MARKER#N`; the comparator
counts visible marker transitions, not repeated transport frames. Binary tests
cover first-marker selection, numbered selection, and missing-marker failure.

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
