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

Implementation progress (2026-08-06): Runie's view DSL now contains a pure
`ScrollState` projection with follow-end, explicit user scroll handoff, and
viewport/content clamping. Its focused reducer tests pass; stack measurement
and YAML exposure of these layout inputs remain open.

The stack resolver is now used by the live chat adapter, and
`visual-resize.yaml` records/validates all five region rectangles. This closes
the first layout-measurement oracle; wider geometries and content-dependent
intrinsic-height assertions remain open.

The `Hey` scenario now carries a four-geometry `layout_matrix`, and the
runner executes each case through the same live adapter. This removes the
previous gap where the four-size test existed only in compiled Rust code.

## Focused audit: grouped activity sizing (2026-08-06)

The source comparison found a concrete Grok policy: `group_max_visible`
defaults to `10`. Grok applies dense-run truncation: groups larger than the
threshold render an `N more` header, hidden members have zero layout height,
and navigation/reveal can restore them. Setting the value to `0` disables the
pass. Grok tests cover the threshold, disabled mode, group breaks, and reveal.

Runie currently has a coarser projection: `activity_expanded=false` hides all
tool rows in the activity projection, while `true` renders all rows. It has no
bounded dense-run policy, `N more` header, zero-height hidden-member model, or
reveal state. This is a genuine multi-member parity gap, but it is not safe to
patch from a single `Hey` frame: the decisive missing data is the ordered
tool-event sequence, group break conditions, configured threshold, and
selected/reveal interaction.

Acceleration decision: add a YAML trace with 12 ordered tool calls, one
non-groupable break, and threshold assertions before implementing the
projection. This is the smallest oracle that distinguishes collapsed activity
from Grok's `N more` truncation. Until then, the three-worker lifecycle frame
is a fixture-specific pass, not generalized group parity.

## Success metric

The review loop is faster when a run answers “what data is missing?” without
manual inspection and when each iteration produces either a reduced classified
mismatch set or a stronger replay oracle.
