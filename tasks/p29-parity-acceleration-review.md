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

The capture prerequisite is now closed at the instrument boundary: the private
tmux harness advertises RGB and removes inherited `NO_COLOR` for the captured
process. Fresh Grok output contains truecolor SGR. The next valid comparison
must still use identical semantic events and a frozen elapsed/usage boundary;
comparing a live Grok network answer to Runie's placeholder stream measures
state divergence, not renderer parity.

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

Intrinsic-height regression (2026-08-09): `layout.rs` now directly proves
that a multiline prompt's measured height is consumed by the declarative
stack allocator: the prompt grows by the supplied intrinsic delta, the
scrollback grow region loses exactly that delta, and status moves with the
prompt. This covers the content-dependent allocation boundary used by the
live `PromptSnapshot::render_height` path without adding renderer state or
making the YAML runner infer geometry from fixed constants.

## Focused audit: grouped activity sizing (2026-08-06)

The source comparison found a concrete Grok policy: `group_max_visible`
defaults to `10`. Grok applies dense-run truncation: groups larger than the
threshold render an `N more` header, hidden members have zero layout height,
and navigation/reveal can restore them. Setting the value to `0` disables the
pass. Grok tests cover the threshold, disabled mode, group breaks, and reveal.

Runie now derives bounded dense groups from the actor-owned transcript and
folds non-selected members at the physical-row boundary when
`activity_expanded=false`; single-tool cards and selected members remain
visible. The actor retains Grok's tool-mode state independently of the group
fold. The decisive YAML oracle uses twelve ordered tool events, a grouped
summary, representative member exclusions, and screen assertions at five
geometries.

Acceleration result: the dense collapsed fixture now distinguishes collapsed
activity from Grok's bounded visible-tail behavior and runs through the
runtime YAML matrix without recompilation. Remaining dense parity is limited
to interactive reveal and viewport-preservation variants.

Group-break replay increment (2026-08-09):
`visual-activity-dense-break.yaml` places assistant text between two
six-member tool runs and asserts separate hidden-prefix markers, visible tail
members, and hidden first members. This closes the documented group-break
coverage gap through runtime YAML discovery; interactive reveal and
viewport-preservation behavior remain separately event-driven.

Implementation progress (2026-08-06): `runie-tui` now exposes a pure
`dense_tool_group_members` policy helper and the named
`GROK_GROUP_MAX_VISIBLE` source default. Reducer/renderer integration is now
covered by the dense YAML screen oracle and renderer regression tests; the
remaining follow-up is broader group-break/reveal coverage.

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

Centered-selection YAML oracle (2026-08-06): `VisualAssertions` now accepts
`center_revealed_entry` and checks the actor snapshot after visual `steps` are
reduced but before drawing. `visual-activity-dense-groups.yaml` asserts the
one-shot centering intent after eleven `Down` events, closing the previous
compiled-code-only evidence gap without conflating event-state assertions with
the later visual interaction phase.

## Success metric

The review loop is faster when a run answers “what data is missing?” without
manual inspection and when each iteration produces either a reduced classified
mismatch set or a stronger replay oracle.

## Lifecycle adapter checkpoint (2026-08-06)

The ordinary-tool running-state experiment established a reproducible harness
boundary: `replay_scenario_events` drives `EventRenderer::apply_actor_event`,
while YAML compatibility setup can also seed `ScrollbackMsg::ToolStart` rows.
Those paths share tool IDs but not row ownership. Promoting all starts to
`ToolRunning`, or settling all matching IDs, corrupts the existing mixed-tool
header oracle. The acceptance condition for the next implementation is a
single actor-owned live-row identity (or an explicit compatibility-row marker)
that proves `ToolExecutionStart -> ToolRunning -> ToolExecutionEnd` without
merging fixture-seeded rows. Until that condition is met, the current green
YAML suite is the authoritative regression baseline.
## Tool-update settle datum (2026-08-06)

Repeated replay of `visual-tool-update.yaml` shows the semantic
`ToolExecutionStart → Update → End` projection can be present while the
80×24 screen alternates between the settled tool row and a viewport that has
already advanced past it. The fixture's existing event and semantic-header
assertions remain authoritative; adding output-row or screen-text assertions
without a deterministic settle/viewport boundary would encode a flaky
capture. The missing datum is the exact Grok post-tool reveal/follow phase,
which must be represented as an explicit YAML event before tightening this
oracle.
## Follow-up decision: settle must remain event-driven (2026-08-06)

The YAML runner currently has no generic `settle` event; adding one that merely
awaits scheduler turns would violate the no-sleep, event-based test contract.
The next implementation should model Grok's post-tool viewport reveal as an
owned reducer event (with an explicit target/offset), then assert the resulting
snapshot and screen. Until that event contract is source-backed, the existing
tool-update fixture is intentionally not tightened.

## Current audit checkpoint (2026-08-06)

The source inventory was rerun against the authoritative Pi and Grok
checkouts; it produced the expected 790 classified entries. The local gate is
green, but this proves only the modeled event/replay contracts. The remaining
high-value evidence gap is a single actor-owned identity across
`ToolExecutionStart → ToolExecutionUpdate → ToolExecutionEnd` when a feed row
was not pre-seeded by a compatibility fixture. The next parity change must add
that event-driven row identity and a YAML trace before tightening screen-cell
assertions. No full-parity claim is made from the current component or saved
cast checks.

The existing `visual-tool-update.yaml` now asserts the settled live reducer
row identity (`tool_header_row_ids`) and inactive lifecycle state after the
real start/update/end sequence. A separate attempted assertion using an
update placed before tool activation was rejected by the replay result: that
sequence is provider-side, not a post-start tool execution update. The next
fixture must place any additional update after an explicit active boundary.

**Explicit latest reveal (2026-08-06):** Grok's follow/goto-bottom behavior is
now represented by the renderer-neutral `ScrollbackMsg::RevealLatest`. The
immutable `FeedSnapshot` carries `autoscroll` across the actor-to-view
boundary, fixing the prior case where the reducer reached the tail but the
widget adapter silently reset follow state. `visual-scroll-reveal.yaml`
asserts the event-driven transition and the adapter has a focused snapshot
regression. No scheduler wait or sleep is involved.

The Grok source audit also confirms that automatic follow is triggered by
prompt submission (`send_prompt → follow_new_turn`), not by tool completion.
Runie therefore keeps `RevealLatest` explicit for a user who has detached the
viewport; wiring it unconditionally to `ToolExecutionEnd` would diverge from
Grok by stealing an intentional scroll position. The next lifecycle parity
work is the richer prompt page-flip/preserve policy, not a tool-end jump.

## Empty tool-result contract (2026-08-06)

Pi normalizes a tool that returns no content to `content: []`. Runie's former
generic result fallback serialized that protocol envelope into the feed,
diverging from Grok's zero-row card. `tool_result_text` now preserves explicit
error text but maps an empty content array to an empty body.
`visual-read-empty.yaml` drives the complete lifecycle and asserts
`Read empty.txt (0 lines)`, one tool card, zero output rows, and no leaked
`"content"` envelope. Image payload transport remains covered by the media
fixture; Grok's inline-image rendering is outside terminal text-cell parity.

## Error-result card parity (2026-08-06)

Grok's expanded tool blocks retain error text as red status rows rather than
discarding the tool result body. Runie's live `ToolExecutionEnd` projection
now emits non-empty error content as `LineKind::ToolError`; the existing
`visual-tool-error.yaml` fixture asserts the ordered header/status rows and
the preserved `tool failed` text while the collapsed full-screen view keeps
the same Grok error-card surface. Actor and live paths now retain the same
error payload semantics.
