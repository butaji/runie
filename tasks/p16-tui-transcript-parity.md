# p16 — TUI: transcript rendering parity (verb-group activity folding, markdown, tool cards, reasoning fold)

**Latest parity note (2026-08-05):** Grouped activity now retains Grok's
failure suffix (`· N failed`) after failed directory/file/command tools
complete. `visual-tool-error.yaml` exercises the contract through the YAML
runner while individual tool rows continue to render `✗`.

- **tmux/asciinema Grok audit (2026-08-05):** A local `grok` session inspected
  `artifacts/grok-rich.cast` alongside the renderer. It identified runtime
  turn-status telemetry (elapsed time, token usage, and stop reason), plus
  queue-key/footer variants, as the important parity boundary. The current
  event-driven runtime now publishes usage/stop-reason data through typed
  assistant `Done` events and owns deterministic elapsed ticks in the status
  actor. Remaining work is capture-level validation of dynamic values and
  queue/footer variants, not a missing runtime event path.

- **Reactive turn telemetry (2026-08-05):** `StatusBar` now owns the turn
  usage/stop-reason projection, resets it on `TurnStart`, and consumes typed
  `MessageUpdate::Done` data through `EventRenderer`. The production loop
  renders `TurnStatus` from that pure projection; a focused test pins token
  and stop-reason formatting without wall-clock sleeps.
- **Deterministic elapsed chrome (2026-08-05):** The status projection now
  advances elapsed tenths from its owned animation cadence and formats Grok's
  `N.Ns ⇣tokens [stop]` contract with exact wire stop-reason labels. This is
  deterministic and remains independent of wall-clock timing.
- **Completed-turn duration (2026-08-05):** `AgentEnd` now emits the owned
  projection's `Worked for N.Ns` transcript line. `visual-submitted.yaml`
  asserts the completion-row contract through the YAML runner, and a unit
  test pins the deterministic elapsed conversion.
- **Table completion border (2026-08-05):** Pipe tables now emit Grok's
  closing `└─┴─┘` row after the final data row, with widths derived purely from
  the table cells. Unit and rich-markdown YAML coverage pin the box geometry.
- **Completion-row gutter (2026-08-05):** `Worked for N.Ns` now uses a
  dedicated `TurnSummary` transcript kind with five-space indentation, placing
  the first glyph at Grok's column 6 instead of the generic system `*` gutter.
  Updated visual snapshots and a focused cell-geometry test pin the contract.

**Parity target:** grok scrollback rendering.

- **Settled `Hey` geometry pass (2026-08-05):** Same-run 62×32
  tmux/asciinema captures now align the user, thinking, assistant, and
  `Worked for` rows; Grok's blank separator rows are projected from the
  message lifecycle. Remaining oracle deltas are telemetry placement/values
  and full-frame chrome, tracked by p19.
- **Local timestamp and overlay pass (2026-08-05):** Prompt timestamps now
  use the host-local clock like Grok, and completed assistant timestamps are
  rendered as a first-content-line overlay with reserved wrapping width. The
  event-owned assistant text is no longer mutated to append display chrome.
- **Per-width block construction (2026-08-05):** The scrollback projection
  now models Grok's full-mode user-block vertical padding at widths `>= 70`,
  uses the compact assistant rail below that breakpoint, and suppresses the
  full-mode summary separator in narrow viewports. The strict suite confirms
  the 80×24 feed advances past its prior prompt-row mismatch.

- **Compact autoscroll replay (2026-08-05):** Fixed the responsive viewport
  projection so actor-owned autoscroll follows newly appended wrapped rows,
  including the long-overflow lead used by the 40×12 `visual-scroll.yaml`
  scenario. The full YAML fixture gate now passes; the lead is expressed as
  named layout tokens rather than a render magic number.

- **Responsive completion gutter (2026-08-05):** Full-mode `TurnSummary`
  rows now use Grok's five-column gutter while compact rows retain the narrow
  three-column gutter. The recorded completion-row cell oracle and YAML replay
  both pass with this width-specific token.

- **Verification boundary (2026-08-05):** The YAML replay gate remains green
  on the committed compact-autoscroll implementation. The separate
  `visual_snapshots` suite still exposes pre-existing full-mode spacing and
  resize differences, so those snapshots are not being silently updated; the
  remaining work is to reconcile them against a fresh, frame-locked Grok
  capture before claiming full transcript parity.

## Grok reference

`~/Code/agents/grok-build/crates/codegen/xai-grok-pager/src/scrollback/render.rs`
- **Verb-group activity folding**: consecutive tool calls of the same verb fold into a header row — `"Read 2 files"`, `"Listed 1 dir"`, `"Ran 1 subagent"`, combined `"Read 1 file, Ran 1 subagent"` (render.rs:1508,1628,1743,1802). Folding keeps a live ` — activity` suffix while running (render.rs:1757-1759).
- Tool cards: name + args + spinner while running, success `✓`/error `✗` marker, structured output rows (from `tool_execution_*` events).
- Reasoning: dim/italic transcript style, collapsible (fold closed/open).
- Markdown: grok uses the `xai-grok-markdown` renderer (`crates/codegen/xai-grok-markdown`) for bold, bullets, headings, code, links.

## Current runie state

`~/Code/GitHub/runie-tests/runie/crates/runie-tui/src/widgets/scrollback.rs` + `event_renderer.rs`
- Has `Line`/`LineKind` (User, Assistant, Tool, ToolOutput, Activity, Reasoning, System), reasoning fold, tool cards, activity grouping (`"◈ Listed 1 dir, Read 1 file"`), markdown bold/bullets (via `markdown_spans`).

## Adapt to runie

1. **Verb-group folding parity**: verify the activity line format matches grok's `"Read 2 files"` / `"Listed 1 dir"` verb-group headers (runie currently renders a combined `"Listed N dir, Read N files"`). Adjust to grok's folded-header + live-activity layout and add collapsible member rows for grouped tool outputs.
2. **Markdown completeness**: add code blocks, fenced code, headings, links, inline code, lists to the markdown renderer (align with `xai-grok-markdown`). Currently bold + bullets only.
3. **Tool cards**: render name + args + spinner + `✓`/`✗` + structured output rows from `ToolExecutionStart/Update/End` (runie has basic version; verify error marker and update lines match grok).
4. **Reasoning fold**: dim/italic collapsed/expanded cells matching grok transcript style (runie has this; verify glyphs/style).
5. **Gutter/cursor**: user feed cursor at column five, blue pointer without bold body (from earlier visual work — retain).

## State machine / variants

Transcript block states:
- `ToolBlock`: `running` (spinner + live activity) → `success(✓)` | `error(✗)`; then `collapsed` (header) | `expanded` (member rows).
- `ReasoningBlock`: `collapsed` | `expanded` (dim/italic).
- `ActivityGroup`: `running(label)` → `idle(label)`.
- `LineKind` variants (already): `User | Assistant | Tool | ToolOutput | Activity | Reasoning | System`.

## Acceptance

- Extend the visual snapshot suite (`visual_snapshots.rs`) with: verb-group folding header, tool error marker, code-block markdown, expanded reasoning cells.
- Compare transcript rows against the recorded grok casts (`artifacts/grok-full.cast`, `grok-rich.cast`) with zero diffs (see p19).
- `cargo test -p runie-tui` green.

## Progress

- **In progress (2026-08-05):** Extended the assistant markdown renderer with
  inline code and link styling, retaining headings, bullets, and bold text.
  Added focused render coverage and deterministic fenced-code marker styling;
  multi-line code-block state is now tracked across assistant lines with dim
  interior styling. Verb-group folding and full tool-card parity remain.
- **Parallel tool rows (2026-08-05):** Tool transcript rows are now keyed by
  `tool_call_id`, so out-of-order updates/completions from parallel core tool
  execution cannot overwrite the most recently appended tool. Added a
  regression test covering crossed update/end ordering.
- **Grok activity labels (2026-08-05):** Added exact unit coverage for the
  recorded rich-cast labels `Listing/Reading` while running and
  `Listed/Read` after completion, including pluralization.
- **Memory result typography (2026-08-06):** Memory result metadata now uses
  Grok's dim token while the result path remains a bold primary span; the
  distinction is covered by a focused renderer test.
- **Mixed verb grouping (2026-08-05):** Activity folding now tracks directory,
  file, and command tools independently, producing Grok-style mixed headers
  such as `◈ Listed 1 dir, Ran 1 command` while retaining each tool member row.
  Added `visual-activity-mixed.yaml` and registered a deterministic replay
  tool kind for the YAML runner. The full markdown/tool-card parity matrix and
  cast-wide zero-diff oracle remain open under p19.
- **YAML tool-error coverage (2026-08-05):** Added an `error` replay tool kind
  and `visual-tool-error.yaml`, which exercises the real tool executor and
  asserts the Grok-style `✗` card marker. The fixture records the observed
  contract that failed tool text is not rendered as a structured output row.
- **YAML rich-markdown coverage (2026-08-05):** Added
  `visual-markdown-rich.yaml` for headings, bullets, bold text, links, and
  fenced code. This exposed and fixed continuation-row parsing so multiline
  assistant markdown is styled in the actual TestBackend path rather than
  rendered as raw syntax.
- **YAML reasoning-fold coverage (2026-08-05):** Added the declarative
  `visual.reasoning_expanded` option and `visual-reasoning-expanded.yaml`.
  The fixture exercises the real event renderer in expanded mode and asserts
  the captured reasoning body alongside the assistant response. The existing
  `visual-reasoning.yaml` fixture simultaneously pins the collapsed `┃ Thought`
  summary, so both reasoning-fold presentation variants are YAML-covered.
- **Activity typography (2026-08-05):** Grouped activity summaries now match
  Grok's bold label styling while retaining the unbolded transcript gutter;
  a focused widget test pins the split.
- **Structured update rows (2026-08-05):** Tool updates carrying string
  `output` or `content` now render as separate indented `ToolOutput` rows;
  scalar/status updates remain attached to their tool card. Added focused
  renderer coverage for multiline structured output.
- **YAML structured-update coverage (2026-08-05):** Added the deterministic
  `structured_update` tool kind and `visual-tool-structured.yaml`; the real
  actor/event replay now exercises separate multiline output rows without
  recompiling the scenario runner.
- **Activity-group boundary (2026-08-05):** Completed activity folding's
  reducer boundary: non-consecutive tool batches now receive separate Grok
  activity headers instead of merging into the previous batch. A renderer
  regression test pins the two-group sequence.
- **Logical batch tracking (2026-08-05):** Activity grouping now retains an
  explicit open-batch flag across sequential tool start/end pairs and closes
  only at the next message boundary, preventing scheduler-dependent splits in
  the YAML Grok-feed scenario.
- **Extended inline markdown (2026-08-05):** The pure scrollback renderer now
  converts ordered-list markers to Grok-style bullets and styles italic and
  strikethrough spans. `visual-markdown-rich.yaml` exercises those forms
  through the real YAML replay path, with focused cell/style coverage.
- **Parallel result boundary (2026-08-05):** Tool-result message boundaries no
  longer close an open activity group; only user/assistant message boundaries
  do. This preserves mixed Grok headers when parallel tools finish and emit
  result messages in scheduler-dependent order, while retaining explicit
  separation between user-visible batches.
- **Cast-backed activity row (2026-08-05):** `visual-grok-feed` now derives its
  expected grouped activity row directly from `grok-rich.cast` and compares
  every rendered cell, with only the recording's transient trailing cursor
  ignored.
- **Blockquote markdown parity (2026-08-05):** Assistant blockquotes now
  render Grok's vertical quote gutter (`│`) while retaining nested inline
  emphasis styling, with focused widget coverage.
- **ATX heading parity (2026-08-05):** Assistant markdown now recognizes all
  CommonMark ATX heading levels (`#` through `######`) through the same Grok
  bold-heading path, with focused level-matrix coverage.
- **Table markdown parity (2026-08-05):** Pipe-table rows now render Grok's
  box-drawing separators and gutters (`│`, `├─┼─┤`) instead of raw pipes;
  `visual-markdown-rich.yaml` exercises the rows through the real TUI path.
- **YAML projection wiring (2026-08-05):** Visual YAML frames now consume
  `StatusBar::turn_status()` when replay events produce an active turn; the
  static typed-prompt fallback remains only for key-input fixtures with no
  active event phase. This keeps telemetry actor-owned while preserving the
  existing prompt geometry contract.
- **Subagent activity vocabulary (2026-08-05):** The Grok renderer reference
  includes grouped `Ran N subagent(s)` activity. Runie now tracks subagent
  tools as a first-class activity counter, composes them with file/directory/
  command groups, and exercises the mixed `Read 1 file, Ran 1 subagent`
  projection through `visual-activity-subagent.yaml` and the real actor
  replay path.
- **Subagent parity verification (2026-08-05):** The full `just ci` gate passes
  after this addition, including the discovered YAML fixture and the existing
  visual/asciinema matrix. The remaining p16 work is interactive fold state
  and cast-wide frame equivalence, tracked under p19.
- **Declarative activity folding (2026-08-05):** Added the actor-replayed
  `visual-activity-collapsed.yaml` scenario and an `activity_expanded` YAML
  projection option. The scrollback model now preserves the grouped activity
  summary while hiding member tool/output rows in collapsed mode; expanded
  mode remains the default. Both modes are verified without sleeps or fixture
  recompilation.
- **Fold verification (2026-08-05):** `just ci` passes with collapsed and
  expanded activity scenarios discovered automatically, alongside the full
  replay and visual/asciinema suite.
- **Selection boundary (2026-08-05):** Grok toggles fold state from its
  scrollback selection/mouse interaction, while Runie currently has no
  scrollback-selection actor or input contract. The model projection and YAML
  states are implemented; wiring a user toggle remains an explicit follow-up
  rather than inventing a non-reference key binding.
- **Feed default parity (2026-08-05):** Cast inspection showed Grok's working
  feed presenting the grouped activity summary with member cards/output rows
  collapsed. Runie's production `Scrollback` now uses that collapsed default;
  YAML render scenarios retain expanded compatibility unless they explicitly
  set `activity_expanded: false`. `visual-grok-feed.yaml` and its cast-backed
  test now assert the hidden member rows as well as the exact grouped gutter.
- **Fold action parity (2026-08-05):** Added Grok's scrollback `e` toggle for
  an empty prompt. YAML visual steps can issue `e`; the runner carries that
  reducer result into the final event-driven replay, and
  `visual-activity-collapsed.yaml` verifies the member cards disappear while
  the grouped summary remains.
- **Feed affordance (2026-08-05):** The Runie shortcut overlay now advertises
  `e fold/unfold feed`, matching the newly implemented Grok-style feed action;
  the full local CI gate remains green with 91 TUI unit tests.
- **Thinking feed row (2026-08-05):** Cast frames show Grok's in-feed
  `┃  ◆ Thinking…` row above the assistant/reasoning content; Runie previously
  exposed that state only in the footer. Added a dedicated bold
  `ThinkingStatus` transcript kind with the Grok gutter and emitted it from
  the event renderer at `AgentStart`. Four affected visual snapshots were
  regenerated from deterministic TestBackend output; `just ci` passes.
- **Thinking row ordering (2026-08-05):** Corrected the projection boundary:
  the thinking row is emitted at assistant `MessageStart`, after the user
  message, matching Grok's feed order. Narrow-feed YAML expectations were
  updated to assert the visible wrapped assistant content after the additional
  reference row; all replay and visual tests remain green.
- **Feed gutter parity (2026-08-05):** Cell-level comparison against the
  recorded Grok frame showed Runie's assistant/reasoning/thinking/tool markers
  were shifted three columns right by duplicating the layout inset. Their
  prefixes now begin at Grok's feed column (`┃`/`◆` at the inset), while the
  user `❯` remains at column five. Focused cell tests and affected snapshots
  were updated from the deterministic renderer.
- **Feed oracle strengthening (2026-08-05):** The cast-backed Grok feed YAML
  now explicitly requires the in-feed `┃  ◆ Thinking…` row in addition to
  the grouped activity header, so the thinking gutter cannot regress while
  lifecycle rows remain source-backed.

- **Vpad source audit (2026-08-05):** Grok's `UserMessageBlock` enables two
  vertical padding rows only for non-compact prompts; system, session-event,
  tool, and activity blocks disable vpad. The next renderer change must model
  this per-entry metadata instead of filtering generic empty system rows.

- **Separator metadata foundation (2026-08-05):** Empty compatibility rows
  now use an explicit `LineKind::Separator`, distinct from Grok system/session
  blocks. Existing compact/full replay behavior remains green while the
  renderer gains the metadata needed for per-entry vpad decisions.
- **Separator normalization (2026-08-05):** Activity-spacing normalization
  now inserts `LineKind::Separator` rather than reintroducing a generic system
  line, keeping the metadata distinction intact through reducer operations.
- **Entry vpad metadata (2026-08-05):** `Line` now carries explicit
  `has_vpad` metadata and user blocks opt into it. Full-mode projection reads
  that metadata instead of assuming every user-shaped row has vpad, preserving
  current replay output while enabling the source-aligned per-entry layout
  implementation.
- **Compact-threshold audit (2026-08-05):** Grok derives compact mode from
  the full terminal height (`AUTO_COMPACT_MAX_ROWS = 20`), not the scrollback
  sub-rectangle. Passing only the scrollback rect into the pure renderer is
  therefore insufficient; the next layout API must carry terminal geometry
  explicitly before changing the responsive prefix rule.
- **Compact-mode layout DSL (2026-08-05):** Added the pure
  `grok_effective_compact` rule with Grok's 16-row short-terminal and 20-row
  auto-compact tokens. Boundary tests cover unmeasured, threshold, overflow,
  and user-forced compact states before renderer wiring.

- **Terminal geometry wiring (2026-08-05):** `Scrollback::render` now has an
  explicit `render_with_terminal_height` path. `App`, the live binary, and
  YAML visual replay pass the outer frame height, so per-entry user vpad uses
  Grok's full-terminal compact rule rather than the scrollback sub-rectangle.
  A renderer regression covers full and compact terminal heights.

- **Vpad clipping guard (2026-08-05):** Matched Grok's block renderer rule
  that enables user vpad only when the available content area has at least
  three rows. Tiny panes now prioritize content over the top/bottom padding,
  with a focused renderer regression.

- **Edit diff card progress (2026-08-06):** Tool output/result rows now
  classify unified-diff insertions, deletions, and hunk headers through the
  selected Opaline success/error/accent tokens. `visual-edit-card.yaml`
  exercises the complete edit lifecycle through the YAML runner, and a
  theme-sensitive widget regression pins the three semantic row styles.

- **Workspace-relative tool paths (2026-08-06):** Absolute provider paths in
  list/read/edit/search headers now use Grok's current-workspace-relative
  projection, including `.` for the workspace root. The pure formatter has
  regression coverage for in-workspace and external paths; existing YAML
  fixtures continue to exercise the same event-driven headers.

- **Running-card animation (2026-08-06):** Runie's actor-owned scrollback
  projection now distinguishes running tool rows and advances their bullet
  phase through `ScrollbackMsg::AdvanceAnimation`, synchronized with the
  existing status animation cadence. A reducer/render test pins the first two
  Grok glyph phases (`⋅`, `:`); terminal lifecycle rows settle back to the
  static tool kind on completion.

- **Background animation demand (2026-08-06):** Scrollback now reports
  animation demand while any subagent-running row exists, so the shared
  actor event loop continues advancing the feed animation after foreground
  status becomes idle. Completion removes that demand; the reducer test pins
  both transitions.

The remaining p16 implementation gap is the full Grok block model: typed
foldable tool entries and strict
cast-wide frame reconciliation. These remain intentionally open; current
YAML fixtures now cover Grok's member-card rule that an individually expanded
expanded or truncated tool remains visible while the surrounding activity
group is collapsed. The
full typed block/navigation model and cast-wide reconciliation remain open.

Selected tool affordance (2026-08-06): selected tool headers now render the
Grok `›` fold indicator from actor-owned selection state, with a focused cell
regression covering the pure scrollback renderer. Full selection boxes and
non-tool entry selection remain open.

Selected-row palette (2026-08-06): the `›` header now applies Grok's
theme-projected selection surface (`bg.selection`) to every header span. The
GrokNight/GrokDay RGB tokens are asserted directly; no widget-level colors were
introduced.

Selected-row surface (2026-08-06): the theme token is now painted across the
entire selected terminal row, including trailing empty cells, while preserving
glyph foreground and modifiers. The focused test asserts both the indicator
and the full-width background.

Mode-specific fold affordance (2026-08-06): selected expanded tool headers now
render Grok's downward `⌄`, while selected collapsed headers render the right
`›` chevron. Both projections have focused cell-level render coverage.

Selection border token (2026-08-06): the selected non-tool entry surface now
also paints Grok's theme-projected `border.selection` rail at both row edges.
The GrokNight and GrokDay border colors live in Opaline themes; the widget does
not contain raw selection RGB values.

Reveal-on-navigation (2026-08-06): `j/k` selection now disables follow mode
and minimally adjusts the actor-owned scroll offset so the selected semantic
row is inside the viewport. A small-viewport renderer regression pins the
selected-row reveal behavior.

Grok scroll-intent parity (2026-08-06): source action definitions distinguish
bare `j/k` entry selection from `Ctrl+j`/`Ctrl+k` one-line viewport scrolling.
Runie now maps the control chords to actor-owned `ScrollBy` intents, handing
off autoscroll without mutating state in the renderer. Key mapping and reducer
tests pass; YAML step support is available for post-submit interaction
scenarios.

Post-submit YAML phase (2026-08-06): `visual.post_steps` now provides a
deterministic interaction phase after the initial prompt and event stream have
settled. It supports the scroll chords without sleeps or polling; a follow-up
fixture still needs a stable expected viewport frame before promotion to the
visual matrix.

Declarative scroll event oracle (2026-08-06): YAML `scroll: -1|1` events now
reduce through the scrollback actor, and `state.scroll_offset` asserts the
resulting logical viewport offset. `visual-scroll.yaml` pins the overflow
scenario at offset 8 while retaining its screen assertions.

Background lifecycle default audit (2026-08-06): Grok's lifecycle block has a
collapsed default, but applying that token alone exposes a deeper missing
piece in Runie's grouped lifecycle height/viewport projection: later completed
and failed workers clip out of the 24-row fixture. The default remains on the
compatibility path until grouped lifecycle layout is implemented and validated
as one event-sequence/YAML scenario.

Background lifecycle fixture clarification (2026-08-06): the three-worker
visual scenario now declares `activity_expanded: true`, matching the Grok
expanded-group capture needed to display all completed/failed/cancelled member
rows in the 24-row frame. The choice is fixture-owned YAML state, not a hidden
renderer default.

Lifecycle clipping re-audit (2026-08-06): the documented three-worker frame
now passes the discovered YAML visual suite with all completed, failed, and
cancelled rows asserted. The fixture-specific clipping gap is closed; the
remaining open item is generalized grouped-block height/reflow across larger
member sets and alternate viewport sizes.

Selection-box geometry (2026-08-06): selected non-tool rows now receive a
post-render Grok-style box overlay: theme-token `│` side rails plus `┌─┐` and
`└─┘` corners when the adjacent rows are inside the viewport. Clipped rows
retain only the visible rail, matching Grok's clipped selection-box rule.

Typed block projection (2026-08-06): added `Scrollback::tool_blocks()` as a
pure read-only projection over actor-owned lines. It preserves first-seen
parallel tool ordering, call IDs, headers, output members, error/running
classification, and `Collapsed`/`Truncated`/`Expanded` mode without introducing
a second mutable state store. A reducer test pins the projection; navigation
and specialized block rendering remain open.

The YAML runner now asserts the projection's block/member counts for structured,
error, and execute tool scenarios, making this intermediate block model part of
the event-sequence-to-state verification surface.

Specialized card projection (2026-08-06): `ToolCardKind` now distinguishes
Memory Search, Workflow, Todo, Use, and Search Tools in addition to the existing
file/search/web/background families. The specialized-tools YAML fixture asserts
the semantic kinds, not only the rendered headers.

Structured-card audit (2026-08-06): Grok's source has dedicated
`MemorySearchToolCallBlock`, `WebSearchToolCallBlock`, `WebFetchToolCallBlock`,
and `WorkflowBlock` renderers with family-specific output/result layout. Runie
currently projects these families semantically but still paints their member
rows through the generic line renderer. Exact structured-card rendering is the
next TUI parity item; taxonomy alone is not claimed as visual parity.

Entry navigation foundation (2026-08-06): the scrollback reducer now exposes
semantic selectable rows (user, assistant/reasoning, and one anchor per tool
block), with actor-owned `j/k` intents and a selected-entry index projecting
the selected tool ID when applicable. Full Grok selection-box painting and
viewport reveal behavior remain open.

Source audit (2026-08-06): Grok assigns default fold modes per concrete tool
block (`Read`/`ListDir`/`Edit`/search/fetch → `Collapsed`, execute/bash →
`Truncated` or `Collapsed`). Runie currently receives only generic tool-start
headers and explicit mode events, so its compatibility fallback remains
`Expanded`. A direct reducer default change breaks the existing background
activity viewport contract; closing this gap requires carrying the typed tool
variant/default mode through the event DSL and validating the full activity
lifecycle in YAML.

Typed default projection (2026-08-06): regular `ToolExecutionStart` events now
project the source-backed default through the actor reducer (`Collapsed` for
ordinary tools, `Truncated` for Bash-like execution). Background lifecycle rows
now also project the source-backed `Collapsed` default for the subagent block.
Fixtures needing member rows declare `tool_mode: expanded`, preserving fast
YAML-only iteration while keeping the default behavior source-correct.

User feed parity (2026-08-06): user rows now use the theme's `bg.panel` token
across the full terminal width, and subsequent submitted turns pin the newest
user entry at the top of the transcript viewport while streaming. The first
session capture remains at Grok's initial-session scroll position; reducer and
full YAML/asciinema visual gates pass.

Source color correction (2026-08-06): Grok's user prompt block uses the
theme's light panel colors (`#242424` Night and `#dedede` Day), distinct from
the selection surface. Opaline `bg.panel` tokens now preserve that distinction
and user-row cell coverage asserts both palettes.

Live follow correction (2026-08-06): user prompts now activate the actor-owned
follow anchor for every submitted prompt, including the first prompt in a fresh
feed, so a submitted input is pinned at the top while the response streams. The
YAML snapshot harness explicitly disables that live viewport mode after replay
to keep settled/cast phases deterministic. A standalone first-prompt reducer
test prevents the live path from regressing to conditional follow behavior.

Scroll handoff correction (2026-08-06): the live viewport now anchors to the
newest user row rather than the oldest prompt. While the new response still
fits, the prompt remains at the top; once incoming content exceeds the viewport,
the reducer hands follow back to the newest tail rows. Reducer tests cover
multi-turn anchor selection and continued streaming output.

Timestamped-anchor correction (2026-08-06): timestamp wrapping can produce
multiple physical rows with `LineKind::User`. The live follow projection now
walks backward from the newest user continuation to the first row of that
prompt block, preserving the full-width background and top anchor in the real
timestamped PTY path. A full lifecycle reducer test and an 80x24 tmux/asciinema
capture verify the previously missing `❯ Hey` row.

Absolute prompt lead correction (2026-08-06): the follow-anchor tests only
verified that a submitted prompt was visible, not its absolute row. Grok
always gives a submitted user block one leading separator row. `Scrollback`
now synthesizes that separator for direct/YAML user events when the event
sequence has not already emitted one, preserving live event ordering without
duplication. The focused geometry assertion requires the prompt glyph at row
1, and the full scroll snapshot covers the regression.

Edit-card diff parity (2026-08-06): added Opaline-backed GrokNight/GrokDay
insert/delete background tokens (`#063806`/`#420e14` and
`#daf2dc`/`#f5dade`). Diff rows now paint semantic foreground and full-row
background, with renderer tests covering both light-theme colors.

Generalized group truncation remains open: Grok defaults to ten visible
members and emits an `N more` header for larger contiguous groups; Runie
currently switches between all activity rows and no activity rows. A 12-member
YAML trace with a group break is required before changing this projection.

Dense-group projection (2026-08-06): Runie now groups unique consecutive tool
members in the actor-owned scrollback projection, preserves non-tool breaks,
keeps the newest ten members, and emits a semantic `╶╶ N more` row for the
hidden prefix. The 12-member YAML oracle covers the threshold path; small
groups and the existing full visual suite remain green. Explicit
Selecting a hidden member now reveals the entire dense group through the
actor-owned selection reducer; exact viewport centering remains open.

Workflow-card contract (2026-08-06): Grok's `WorkflowBlock` requires
`run_id`, `name`, `objective`, status (`running`/`done`/`failed`/`cancelled`/
`paused`), elapsed duration, phase title/state trail, current phase, and active
agent count. Runie's generic tool lifecycle currently exposes only tool name,
args, result, and error status. The missing workflow lifecycle event payload is
now explicitly identified; a renderer-only patch would lose source state and
violate the SSOT/event contract.

Workflow event foundation (2026-08-06): Runie core now defines
`WorkflowStarted`/`WorkflowProgress`/`WorkflowFinished` events carrying that
metadata, and the YAML DSL plus `visual-workflow-lifecycle.yaml` replays the
full lifecycle. The TUI now projects those events through actor-owned
`ScrollbackMsg` reducer inputs into one in-place workflow card keyed by
`run_id`; the YAML fixture asserts both the final card text and one-card
cardinality. Exact Grok phase-trail formatting and richer structured workflow
rows remain open.

Workflow objective wrapping (2026-08-06): The pure workflow formatter now
flattens newline-containing objectives to one transcript row, matching Grok's
`WorkflowBlock` output contract. A focused renderer regression covers CR/LF
input; richer structured workflow styling remains open.

Workflow terminal-state YAML matrix (2026-08-06): Added
`visual-workflow-terminal-states.yaml` to exercise failed and cancelled
workflow cards, including phase trails and elapsed values, through the real
Pi-shaped event stream and actor reducer. The parity component index now maps
the workflow card to its Grok sources and both lifecycle fixtures. Rich
per-span workflow styling and preamble expansion remain open.

The same fixture now includes Grok's paused terminal state, so all five
workflow status variants represented by the current Runie event contract are
covered declaratively: running, done, failed, cancelled, and paused.

Web-search structured-card audit (2026-08-06): `visual-web-search.yaml` now
asserts the semantic `Web Search … (N sites)` header and deduplicated
`Sources:` output through actor-projected tool headers/rows. Comparison with
Grok's `WebSearchToolCallBlock` confirms this family is covered for the
summary/source contract. Memory-search result parsing (score, source, file
range, and snippet panel) remains the next specialized-card gap.

Memory-card projection audit (2026-08-06): Grok parses `### Result N` blocks
into score/source/file-range/snippet rows. The current implementation has the
required pure parser and shared `memory_display_lines` projection in
`runie-tui-model`; both live event rendering and deterministic replay consume
that contract, and `visual-specialized-tools.yaml` asserts the two-result
header/output behavior. Snippet rows now use the muted panel-background
semantic style; exact metadata spans, full row fill, and reflow remain open.

Non-tool selection YAML oracle (2026-08-06): `visual-hey.yaml` now performs
`tool_select: entry_next` against a tool-free conversation and asserts the
actor-owned `selected_entry` index. This proves user/assistant semantic rows
participate in the same event-driven navigation contract as tool rows.

Memory-card projection slice (2026-08-06): added the renderer-independent
`parse_memory_results` contract with score, source, file range, and fenced
snippet fields. Live event rendering and YAML replay now format the same
structured rows, and `visual-specialized-tools.yaml` asserts both result
headers and snippets.

Typed-card render audit (2026-08-06): the actor/model side now preserves
Grok's `ToolCardKind`, lifecycle flags, display mode, output ownership, and
parallel-call identity. The remaining renderer gap is architectural: the
widget's `physical_rows` still flattens every specialized card into
`(LineKind, String, code_row)` tuples. Grok's `Read`, `Edit`, `WebSearch`,
`MemorySearch`, `Workflow`, and background cards each have card-specific
header spans, status glyphs, metadata rows, and selection ranges. The next
parity slice must introduce a renderer-neutral card-row vocabulary carrying
that semantic identity before Ratatui styles are applied; changing individual
colors or prefixes in `physical_rows` would lose the distinction again.

Typed-card row vocabulary slice (2026-08-06): added model-owned
`ToolCardRowKind`/`ToolCardRow` projections for header, content, and status
semantics. The existing truncation projection now consumes those rows for
read/execute output accounting, while terminal appearance remains stable.
The next slice can apply Grok per-card spans and metadata without rebuilding
identity in the widget.

Tool lifecycle replay deduplication (2026-08-06): `FeedState` now treats a
terminal `ToolEnd` payload as a replay of an already applied `ToolUpdate` when
the same tool owns the same output text, even when a terminal activity line
separates the events. YAML activity, structured-tool, and specialized-card
fixtures assert the resulting single visible output set; web-search retains
its separate source-summary projection. The model regression covers the
update/activity/end sequence directly.

Semantic row oracle expansion (2026-08-06): the activity-mixed, truncated-read,
structured-tool, and web-search YAML scenarios now assert ordered
`ToolCardRowKind` sequences in addition to headers and output text. This keeps
the renderer-neutral card contract executable across grouped, specialized,
and replay-deduplicated feed paths before any per-card styling is changed.

Memory projection SSOT (2026-08-06): the Grok memory markdown parser and its
`Result N · score · source · location` transcript-row projection now live
together in `runie-tui-model`. Live tool completion and structured-update
paths call the same pure function; terminal styling remains a renderer concern.

Terminal error-row semantics (2026-08-06): Grok's explicit `✗` completion
marker is projected as `ToolCardRowKind::Status` even when replay has not yet
observed the compatibility error-kind mutation. `visual-tool-error.yaml`
asserts this model-level status role without introducing a second event or
changing the actor ordering.

Read completion-mode parity (2026-08-06): Grok's `ReadToolCallBlock` declares
`finished_display_mode = Collapsed`, so a read card returns to its title-only
projection after completion even if it was expanded while running. The
canonical `FeedState` reducer now owns this transition, with a model event
sequence regression. The existing YAML fold fixture remains the post-completion
user-intent oracle; adding an in-flight mode event to the YAML runner remains a
separate ordering improvement because control declarations are currently
applied after the generated core event stream.

Read ranged-header projection (2026-08-06): the scrollback actor now retains
`ToolExecutionStart.args` by call ID and uses Pi/Grok-compatible completion
metadata to project ranged Read headers (`start-end of total`) through the
existing event reduction path. This closes the common text-read range case;
typed media and error metadata still require an explicit completion payload.

The YAML DSL now accepts per-tool `output`, `details`, and `error` fields, so
the ranged case is replayed from `visual-read-range.yaml` without recompiling
the runner. The same argument-retention rule is applied in both the actor
projection and compatibility renderer paths; the fixture exercises the live
actor path and asserts the complete header text.

The same fixture DSL now supports `media: image/png`, producing the existing
Pi `ToolResultContent::Image` variant. `visual-read-media.yaml` asserts the
Grok image suffix without adding renderer-owned state.

YAML tool registration is now one shared declarative path: both state replay
and visual-buffer replay call `register_scenario_tool`. This removes the
previous duplicated match table that could silently make a fixture's visual
and non-visual results diverge.

Semantic paint boundary (2026-08-06): `ToolCardRow` now exposes a pure
`ToolCardPaintIntent` (`Header`, `Content`, `Success`, `Error`, `Muted`). The
model owns lifecycle-to-role classification; the renderer must resolve these
roles through theme tokens. A focused model test covers header/content/error
states without terminal types. Wiring every specialized card's spans through
this intent remains the next renderer slice.

Paint-intent resolution (2026-08-06): the scrollback renderer now resolves
exact, unwrapped typed card rows through `PaintIntent` and Opaline theme tokens
before terminal painting. Existing span modifiers and bold action-name
structure are preserved; rows that are reflowed or wrapped remain on the
renderer's normal text path until their physical-row identity is carried
explicitly. This keeps the partial migration honest while making header,
content, success, error, and muted semantics available at the render boundary.

Wrapped-row carryover (2026-08-06): physical rows now resolve card identity
for wrapped segments when their source text is recoverable from the logical
line. This extends semantic token resolution to common wrapped tool output
without changing row geometry. Exact identity propagation through every
reflow operation remains the stronger follow-up for duplicate-text cards.

Duplicate-row disambiguation (2026-08-06): paint-intent lookup now includes
the physical row's prior-occurrence index, selecting the corresponding nth
logical source row when duplicate tool text appears. This removes the
first-match ambiguity without changing actor-owned feed state or wrapping
geometry.

Specialized header typography (2026-08-06): source inspection of Grok's
`WebSearch`, `MemorySearch`, and search-tools blocks showed the complete action
label is one bold span. Runie now preserves the full `Web Search`, `Memory
Search`, and `Search Tools` labels as bold spans while keeping query/details in
the normal theme-token style. Focused span tests cover the distinction.

Path-span typography (2026-08-06): Grok's file-oriented cards use the
dedicated path token for `Read`, `List`, and `Edit` header operands. Runie now
splits those headers into bold action, separator, and theme-resolved path
spans, with a GrokDay token assertion; no RGB values are owned by the widget.

Search-scope typography (2026-08-06): search headers now split the bold
`Search` action, query, `in` separator, and scope path; the scope resolves
through the same semantic path token as file cards. A focused GrokNight span
assertion pins the separation.

Web-source typography (2026-08-06): Grok's web-search card renders the
`Sources:` label in the muted token while source domains use the primary body
token. Runie now preserves that label/domain distinction in the output-row
renderer, with a GrokDay token assertion.

Memory-result typography (2026-08-06): Grok's memory card renders result
metadata muted and the file/range operand as bold primary text. Runie now
splits `Result N · score · source · path:range` into equivalent semantic spans,
with a focused path-style assertion.

Web-source truncation (2026-08-06): source-domain spans remain primary while
the `(+N more)` overflow suffix resolves to the muted token, matching Grok's
source summary row instead of treating the suffix as another domain.

Memory-snippet semantics (2026-08-06): memory-card content rows now project
the muted paint intent from the model-owned `MemorySearch` card kind, matching
Grok's muted snippet preview while ordinary tool output remains primary.

Workflow-card typography (2026-08-06): source inspection of Grok's
`WorkflowBlock::output` shows a bold `Workflow ` label, muted body/phase
metadata, and a dimmed cancelled body. Runie now resolves those spans through
Opaline semantic tokens for both running and cancelled workflow rows, with a
focused light/dark renderer test. Workflow phase-trail layout and richer
structured metadata remain separate open parity work.

Workflow terminal phase markers (2026-08-06): workflow phase state is now
reduced through the shared model formatter for `active`/`running`,
`done`/`completed`, and `failed`/`error`/`interrupted` states. Failed phases
emit Grok's explicit `✗` marker instead of the pending `○` fallback; the
terminal-state YAML replay oracle covers the visible result. The remaining
workflow gap is per-span phase styling/layout and richer structured metadata,
not state delivery or terminal glyph selection.

Workflow phase semantic spans (2026-08-06): workflow trail markers now retain
their model-produced meaning through the pure renderer boundary. Success,
active, error, and pending markers resolve through Opaline theme tokens while
the phase names, delimiters, objective, and agent-count suffix stay in the
event-derived muted body style. The focused workflow tests verify both marker
selection and token roles; exact Grok spacing remains open.

Typed member identity (2026-08-06): `ToolCardRow` now carries a pure
per-card `member_index` assigned during model projection, so duplicate text
and dense-group members remain distinguishable without renderer string
matching. The mixed-activity YAML fixture asserts the ordered member indices
through the same actor snapshot path. This is the first slice of the remaining
typed block/member model; fold range semantics and full cast reconciliation
remain open.

Member-index correction (2026-08-06): the ordinal is now assigned once per
logical tool call and shared by all of that member's header/content/status rows;
it is no longer a physical-row counter. The mixed fixture caught and pins this
distinction (`[0, 1, 0, 0, 0, 1]` for two interleaved projected members).

Selected-member projection (2026-08-06): the immutable feed snapshot now
derives `selected_member_index` from the actor-owned selected transcript entry
when that entry belongs to a tool member. Non-tool selections intentionally
project `None`; no renderer-side cursor state is introduced. YAML supports the
assertion field, while a fixture that ends on a non-tool entry confirms the
negative case. Tool-selected member assertions remain the next navigation
fixture slice.

Tool-selection synchronization (2026-08-06): `SelectNextTool` and
`SelectPreviousTool` now update `selected_tool_id` and `selected_entry` as one
reducer transition, then reveal the corresponding dense group. The
`visual-tool-row-identity.yaml` replay asserts the selected compatibility row
and member index. This closes the prior split-brain selection projection;
viewport styling and richer per-member fold ranges remain open.

Fold-anchor boundary (2026-08-07): the existing YAML dense-group fixtures
prove member reveal, hidden-prefix policy, centering intent, and collapsed-tail
behavior. Exact Grok parity for arbitrary fold/reflow transitions still needs
an actor-delivered physical layout measurement (wrapped member heights plus a
stable anchor identity) before the reducer can preserve the viewport across a
fold. The current `FeedSnapshot` intentionally contains semantic rows and
scroll intent, not renderer measurements; adding a heuristic offset would
violate the declarative/state-versus-render boundary. The next implementation
slice is therefore a typed layout-measurement event and YAML frame oracle,
followed by reducer anchor restoration.

Measurement contract increment (2026-08-07): `ScrollbackMsg::LayoutMeasured`
now transfers `(content_rows, viewport_rows, anchor_row)` through the
`ScrollbackActor` reducer into `FeedSnapshot`. The contract is pure and
covered by a model test; renderer emission, YAML frame capture, and anchor
restoration remain the next slices.

Anchor restoration increment (2026-08-07): measured manual-scroll anchors now
recenter after a `ToggleToolMode` fold transition in the pure feed reducer.
Autoscroll and unmeasured compatibility paths remain unchanged. The remaining
parity work is multi-member reflow across arbitrary wrapped rows and a
cast-wide frame oracle.

Renderer identity handoff (2026-08-06): semantic paint-intent lookup now
resolves the source line's logical member ordinal and requires the matching
`ToolCardRow.member_index` before applying theme intent. Duplicate text can no
longer silently borrow the first card's paint role; the renderer consumes the
actor/model identity instead of rebuilding card ownership from text alone.

Member identity SSOT (2026-08-06): logical member ordinal derivation is now
centralized in the pure `runie-tui-model::logical_tool_member_index` helper.
Feed snapshots, compatibility snapshots, and renderer paint resolution all use
that same function; no layer independently reconstructs member ownership.

Running-card paint intent (2026-08-06): running tool headers now project a
model-owned `Running` semantic paint role, distinct from settled headers. The
renderer resolves it through the existing Opaline accent token; no running
state color is hardcoded or inferred from header text. The typed-card model
test covers running and settled header roles, while lifecycle YAML and visual
suites retain the event/state coverage.
