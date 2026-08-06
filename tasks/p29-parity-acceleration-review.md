# p29 — Per-run parity acceleration review

## Objective

After every pi/Grok parity run, identify the highest-impact missing evidence,
then make the smallest change that either closes that gap or improves the
instrument measuring it. This review is limited to behavior Runie can expose
from pi/core events.

## Run record required for a decision

Each run must retain:

- source revisions and binary versions for pi, Grok, Runie, tmux, and
  asciinema;
- terminal geometry, `TERM`, `COLORTERM`, alternate-screen state, and reported
  color capability;
- the exact event sequence, prompt, tool inputs/outputs, model/usage values,
  elapsed-time inputs, and animation frame marker;
- raw cast plus a decoded cell dump containing glyph, width, foreground,
  background, attributes, and coordinates for every cell, including blanks;
- the selected reference frame and why it was selected;
- mismatch counts split into glyph, geometry, style, color, and dynamic
  telemetry, with the first differing coordinate and surrounding rows.

Missing any of these makes the result diagnostic, not a parity decision.

## Fast review loop

1. Reproduce one scenario at four default geometries with frozen event data.
2. Compare pi event/state output before inspecting pixels.
3. Compare the complete screen, not individual widgets; classify each delta.
4. Consult the source path for the affected state and add a YAML assertion.
5. Make one focused change, rerun the focused scenario, then run `just ci`.
6. Record the new mismatch classification and next missing datum here or in
   the owning parity task.

## Current highest-value missing data (2026-08-06)

The native 80×24 comparison has zero attribute differences after marker
selection, but still differs in dynamic telemetry and frame count. The next
capture needs deterministic elapsed time, token counters, and a stable settled
frame marker on both sides. Until those are present, a full-cast exact claim
would conflate layout parity with clock/usage parity.

The next layout review must also capture pi's measured content height,
viewport offset, and terminal cell-width behavior from `layout-node` and
`scroll-view`; the current YAML contract records final text but not the
measure/layout inputs that explain wrapping and scrolling changes.

## Success metric

The review loop is faster when a run answers “what data is missing?” without
manual inspection and when each iteration produces either a reduced classified
mismatch set or a stronger replay oracle.
