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

Implementation progress (2026-08-06): `runie-tui` now exposes a pure
`dense_tool_group_members` policy helper and the named
`GROK_GROUP_MAX_VISIBLE` source default. Reducer/renderer integration and the
YAML assertion are intentionally still pending; the helper is covered by
unit tests and keeps the next change from embedding grouping rules in paint
code.

The decisive replay oracle is now present as
`visual-activity-dense-groups.yaml`: twelve ordered Bash members are split by
an assistant delta into two six-member groups, and the fixture asserts all
twelve typed blocks plus representative rendered rows. It currently passes
the discovery/replay harness and establishes the event data needed to add
Grok's ten-member `N more` projection without conflating group boundaries.

Projection progress (2026-08-06): the scrollback view now applies the named
ten-member budget to consecutive unique tool ids, hides the oldest member
output with its member, keeps the newest ten, and emits `╶╶ N more`. The
12-member single-group oracle now checks that source-backed shape; existing
mixed-activity coverage remains the boundary oracle. Full workspace tests pass.
The next missing datum is Grok's exact reveal interaction and whether the
hidden count is navigable as one header or individual members. Runie's current
selection path now reveals the containing group when a hidden member is
targeted, marks that member as the selected entry for the renderer's
actor-owned visibility handoff, and has a focused reducer/render test. Exact
centered offset parity remains to be captured.

The YAML dense-group fixture now drives eleven `Down` selections and asserts
that the previously hidden first member is visible afterward. This closes the
reveal interaction oracle without recompiling a scenario-specific test; the
remaining evidence gap is the exact centered offset value.

Reveal viewport progress (2026-08-06): selecting a hidden member now sets a
one-shot actor-owned centering intent. The renderer clamps the selected row to
the viewport midpoint on the next frame, matching Grok's
`scroll_to_entry_center` phase while retaining normal visibility-following for
ordinary selection. Focused and full gates pass.

## Success metric

The review loop is faster when a run answers “what data is missing?” without
manual inspection and when each iteration produces either a reduced classified
mismatch set or a stronger replay oracle.
