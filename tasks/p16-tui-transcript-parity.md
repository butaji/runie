# p16 — TUI: transcript rendering parity (verb-group activity folding, markdown, tool cards, reasoning fold)

**Latest parity note (2026-08-05):** Grouped activity now retains Grok's
failure suffix (`· N failed`) after failed directory/file/command tools
complete. `visual-tool-error.yaml` exercises the contract through the YAML
runner while individual tool rows continue to render `✗`.

- **tmux/asciinema Grok audit (2026-08-05):** A local `grok` session inspected
  `artifacts/grok-rich.cast` alongside the renderer. It identified the next
  concrete parity boundary as runtime turn-status telemetry (elapsed time,
  token usage, and stop reason), plus queue-key/footer variants. The existing
  pure `TurnStatus` renderer is ready for those values, but the event-driven
  runtime does not yet publish them; this remains an implementation task, not
  a cosmetic prefix substitution.

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

Typed block projection (2026-08-06): added `Scrollback::tool_blocks()` as a
pure read-only projection over actor-owned lines. It preserves first-seen
parallel tool ordering, call IDs, headers, output members, error/running
classification, and `Collapsed`/`Truncated`/`Expanded` mode without introducing
a second mutable state store. A reducer test pins the projection; navigation
and specialized block rendering remain open.

The YAML runner now asserts the projection's block/member counts for structured,
error, and execute tool scenarios, making this intermediate block model part of
the event-sequence-to-state verification surface.

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
remain on their existing compatibility path until their event payload carries
the concrete Grok block variant. Fixtures needing member rows declare
`tool_mode: expanded`, preserving fast YAML-only iteration.
