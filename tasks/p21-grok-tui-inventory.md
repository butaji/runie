# p21 — Grok TUI parity inventory

Status: active. Audited `/Users/admin/Code/agents/grok-build`, especially
`crates/codegen/xai-grok-pager` and `xai-grok-pager-render`.

**Construction audit (2026-08-05):** Grok composes the agent view from a
two-column outer horizontal pad, a scrollback `HorizontalLayout` with a
one-cell accent rail, two-cell left block padding, flexible content, and a
one-cell right pad. Entries are measured before painting; display mode and
vertical padding affect height, and timestamps are overlaid on the first
content row rather than appended to message text. Runie now names and tests
these source-derived layout tokens and uses them in the chat layout.

## Theme-token parity (2026-08-05)

Opaline semantic style projections now accept the actor-selected `ThemeKind`;
status styling no longer resolves `GrokNight` unconditionally. A GrokDay
regression verifies that foreground, background, and accent cells come from
the day palette's tokens. The feed projection now receives the selected theme
through its pure render path; broader Grok state/variant coverage remains
before this inventory can be marked complete.

The selected theme is now also propagated to the feed actor on
`ThemeChanged`; `visual-theme-day.yaml` exercises the complete event-driven
path without recompilation. Feed style unit coverage verifies assistant and
activity cells resolve day-theme tokens.

**Command-palette boundary (2026-08-05):** Cast inventory exposed Grok's
`Echo Command Query Title` modal. Runie now has an actor-owned
`command_palette_open` MVU state, `Ctrl+P`/`?` actions, themed modal chrome,
and Esc close behavior. Query text, filtering, and selection movement are also
actor-owned, with a pure `CommandPaletteWidget` shared by the live binary and
YAML visual runner. `visual-command-palette.yaml` covers the filtered modal;
full Grok command registry/action execution remains open.

Typed palette actions (2026-08-06): UI activation now emits a typed
`PaletteAction` through the actor command bus. The live binary and YAML replay
match `NewSession`, `KeyboardShortcuts`, and `Quit` variants directly; the
visible labels remain a pure palette projection.

`New Session` now follows the event boundary: palette activation is reduced
by the UI actor, and the binary/YAML paths publish the existing
`AgentEvent::Reset`. `visual-command-palette-activate.yaml` covers that action;
other Grok commands remain unimplemented until their corresponding core/TUI
event contracts are added.

The pure palette registry now mirrors the first 15 stable Grok labels
(`New Session`, worktree/session actions, context actions, model switch, and
shortcuts), with filtering exercised by YAML. Action dispatch remains an
explicit follow-up because it must publish core/UI events rather than mutate
loop or session state from the modal.

## Findings

- Grok has 402 pager source files and a typed `RenderBlock` projection:
  `UserPrompt`, `AgentMessage`, `ToolCall`, `Thinking`, `System`,
  `SessionEvent`, `BgTask`, `Subagent`, `Workflow`, `Btw`, `ContextInfo`,
  `CreditLimit`, and `Stub`.
- Tool rendering is specialized for execute, read, edit, list-dir, search,
  web-search, web-fetch, memory-search, lifecycle, generic use, and other
  tools. Blocks own output, grouping, selection, display mode, accent/bullet,
  and running/finished behavior.
- `BlockContext` carries display mode (`Collapsed`, `Truncated`, `Expanded`),
  wrapping mode, selection, width, raw mode, appearance, and cwd to pure
  renderers. This is the correct reference model for replacing Runie's flat
  `LineKind` projection.
- Typed turn activity includes Thinking, Responding, ToolRunning,
  AutoCompacting, Retrying, and WaitingReason variants for model, subagent,
  task output, tasks complete, and sleep. Runie's generic waiting status is
  insufficient for parity.
- `xai-grok-pager-render` provides GrokNight, GrokDay, TokyoNight,
  RosePineMoon, OscuraMidnight, and Auto themes, with truecolor definitions and
  terminal-capability quantization. Appearance configuration covers animation,
  block backgrounds, layout, prompt, scroll, scrollbar, tool bullets, edit
  headers, todos, mermaid, and terminal behavior.
- Animation is demand-driven: the event loop schedules ticks only while a
  view reports demand. Shared ticks drive spinner frames, animated accents,
  running task bullets, thinking, workflows, and overlays. The source also has
  explicit tmux/color capability probes.
- PTY tests cover waiting labels, spinner resumption, verb-group fold/expand,
  streaming/thinking folds, prompt geometry, resize, scroll, queue/interjection
  lifecycles, cancellation, overlays, welcome variants, and terminal color
  paths. This is the authoritative feature/state matrix, not isolated row
  snapshots.

## Runie gap map and order

1. Replace ad-hoc visual flags with an actor-owned typed presentation model
   carrying block/display/waiting state.
2. Add typed waiting reasons and animation demand/frame events to the event bus.
3. Add theme/config and terminal capability projections with canonical truecolor
   comparison.
4. Model tool groups/member visibility as foldable blocks with exact gap
   geometry.
5. Add YAML event/state scenarios and full PTY captures for every state and
   animation variant; keep TestBackend as the fast deterministic path.

Implementation progress: `runie-core::types::WaitingReason` now mirrors
Grok's model/subagent/task-output/tasks-complete/sleep variants, and
`AgentEvent::Waiting` is projected by the TUI status actor. The event kind is
covered by the shared DSL and YAML event oracle. `just ci` passes after this
change.

The YAML runner now accepts `waiting: { waiting: model }` and the other typed
variants as declarative events. It omits control events from the provider
stream, then appends them to the recorded bus sequence before replay, so
scenario edits do not require recompiling the test harness.

Animation scheduling now follows the same demand boundary: the renderer owns
one restartable timer and arms it only while the status actor projects an
animated state. Idle, ready, aborted, and error states do not schedule timer
wakes. The demand predicate is covered by a unit test.

The event contract now also includes Grok's six named theme variants
(`GrokNight`, `GrokDay`, `TokyoNight`, `RosePineMoon`, `OscuraMidnight`, and
`Auto`). YAML can declare `theme: grok_day`; the status actor owns the active
projection. Full palette propagation and terminal color quantization remain
open.

Feed progress: completed directory/file tool cards now retain Grok's semantic
card headers and cardinality (`List … (entries)`, `Read … (lines)`) instead of
the generic `→ ✓` suffix. Generic tools retain their status marker. This is
covered by renderer tests and the visual tool replay.

Working-view baseline progress: the literal `Hey` → short assistant response
scenario is pinned in `visual-hey.yaml`. The full-mode layout now uses Grok's
transcript rail, assistant body alignment, and quiet branch/path
header; the fixture explicitly rejects the prior `user>`/`assistant>` feed
labels.

The same replay now verifies the turn lifecycle: the provisional `◆
Thinking…` row folds to `❙  Thought` when a non-empty assistant response
commits, while empty tool/waiting captures retain their reference chrome.

Live submitted prompts now carry a wall-clock timestamp through the user
message event and render Grok's short `h:mm AM/PM` prompt-row format. Replay
fixtures with synthetic timestamps remain deterministic; the formatter has
direct unit coverage. YAML also accepts `prompt_timestamp`, and the `Hey`
fixture pins the complete timestamped user row without recompilation.

Color parity progress: the live frame now paints the GrokNight truecolor base
(`bg #141414`, `fg #e1e1e1`) before widgets render, and feed semantics use the
Grok accent/muted/success/error palette instead of terminal-default colors.
Color-sensitive visual snapshots were refreshed and the palette projection is
unit-tested; full theme switching and terminal quantization remain open.

Live `just tui` audit also found and fixed a separate input-path issue: the
binary discarded Shift+Char events, so typing `Hey` produced `y`. The direct
terminal path now forwards shifted characters to the prompt actor; tmux
replay shows `❯ Hey` and the timestamped row before submission.

The reference feed component matrix is now recorded explicitly:

| Family | Grok variants/states | Runie coverage |
|---|---|---|
| User/assistant | prompt, markdown, tables, code, links, streaming | YAML + snapshots |
| Thinking | running spinner, collapsed `Thought`, expanded reasoning | YAML + renderer tests |
| Tool cards | execute, read, edit, list-dir, search, web, lifecycle, generic | partial; typed core events, line projection |
| Tool display | collapsed, truncated, expanded, running/finished/error | collapsed/expanded activity fixtures; block model open |
| Verb groups | Read, Listed, Searched, Ran, subagent counts; running/past verbs | activity summary projection |
| Background work | subagent, workflow, task output, waiting reasons | waiting reasons; block rendering open |
| Chrome | header meter, status telemetry, prompt/footer, doctor hint | strict feed/waiting frames |
| Effects | braille/dot spinners, animated accents, overlays, terminal capability paths | demand-driven spinner; broader effects open |

The strict fixed-grid oracle now covers both the Grok feed and waiting frames;
component/state fixtures remain the next expansion target for the partial tool
and effect families above.

Per-tool display state is now event-driven: `ToolDisplayModeChanged` carries a
tool-call ID and `Collapsed`/`Truncated`/`Expanded` mode. Tool lines retain
their originating call ID. Collapsed cards keep their semantic header while
hiding output/result rows; truncated cards keep the first output/result row.
The mixed activity and truncated activity YAML fixtures exercise these rules
independently, including a complete fixed-grid replay assertion.

The immediate task remains broader tool-card parity (specialized execute/edit/
search/web/background cards), followed by theme palette propagation and the
remaining YAML state/effect matrix. Strict feed, waiting, collapsed, mixed, and
truncated fixed-grid replays are green.

Typed tool-block projection (2026-08-06): Runie now exposes a pure
`Scrollback::tool_blocks()` projection over the actor snapshot, aligned with
Grok's block/member model for parallel IDs and display modes. This is the
foundation for the remaining foldable navigation and specialized card layout;
it does not claim those behaviors are complete.

Declarative block payload assertions (2026-08-06): YAML scenarios now pin the
ordered semantic headers and per-block output rows in addition to block count
and display mode. This verifies the actor projection's payload, while keeping
rendered truncation separate from retained state; specialized renderers and
selection/navigation remain open.

Command-palette projection progress (2026-08-06): aligned the modal heading
with Grok's `Commands` vocabulary and bounded keyboard selection to the
currently filtered entry set. The state remains UI-actor-owned and the visual
YAML scenarios exercise the rendered heading without introducing a direct core
mutation path.

Command activation events (2026-08-06): palette activation now emits a typed
`UiCommand` from the UI actor after the reducer acknowledgement. The YAML
runner consumes that event to publish `Reset` for `New Session`; command
execution is no longer inferred from a mutable UI snapshot.

Web-fetch card progress (2026-08-06): Fetch completion output now uses the
Grok tool-output projection (indented neutral metadata) instead of the generic
green result-arrow row. A YAML scenario pins the URL header, HTTP metadata,
and output line kinds through the real actor replay.

Web-search card progress (2026-08-06): completion cardinality now follows
Grok's distinct-site vocabulary (`site`/`sites`) and deduplicates citation
domains before rendering the header. YAML replay covers repeated `docs.rs`
citations and a second domain.

Background-work event progress (2026-08-06): added typed core lifecycle events
for background work start/progress/finish. The scrollback projection maps them
to actor-owned semantic Subagent rows, and YAML replays the running and
completed states without inferring lifecycle from a generic tool name.

Failure-branch coverage (2026-08-06): background lifecycle wire round-trips
now pin camelCase `workId` and the YAML scenario also replays a failed worker,
ensuring the typed error state survives the core-to-feed boundary.

Source audit: Grok's `UserMessageBlock` is the only standard conversation block
that enables prompt vpad; system, session-event, thinking, background, and all
specialized tool blocks explicitly disable it. The renderer also suppresses
vpad when the available content area is smaller than three rows. Runie's
`Line::has_vpad` metadata and terminal-height-aware renderer now preserve that
distinction, with unit coverage for full, compact, and undersized panes.

Execute-card progress: shell/exec tool headers now use Grok's semantic `Run`
label with the command argument, and `visual-execute.yaml` exercises the path
through the real event replay and rendered screen assertions.

Integration discovery progress: `search_tools` now uses Grok's `Search Tools`
header and `(N results)` completion cardinality, covered by
`visual-search-tools.yaml`.

Subagent-card progress: subagent/agent/task tools now render explicit
`Subagent started:` and `Subagent completed:` lifecycle text, covered by
`visual-subagent-card.yaml`. Duration text remains intentionally deferred to
the actor-owned tool timing projection.

Theme-token audit: the compact welcome title no longer uses terminal-default
`Color::Cyan`; it now resolves through the GrokNight Opaline accent token, so
welcome chrome follows the same theme projection as the feed and status bar.

Full-screen theme propagation (2026-08-06): both live binary draw paths now
derive the terminal background from the actor-owned `StatusBar` theme
snapshot. A `ThemeChanged` event therefore updates the background as well as
feed, status, and prompt token consumers; the previous hardcoded GrokNight
background was a real palette propagation gap. The complete local gate passes.

Prompt theme projection (2026-08-06): `PromptWidget` now stores the active
theme as actor-owned view state and consumes `ThemeChanged` through
`PromptActor`. Prompt borders, cursor, placeholder, and body styles resolve
the same Opaline tokens as the rest of the screen; a day-theme render test
guards against regression.

Prompt actor event regression (2026-08-06): an async actor test now publishes
`ThemeChanged(GrokDay)`, renders the actor snapshot, and asserts the day token
at the cursor cell. This verifies the event-to-view path, not only the pure
widget reducer.

Command-palette theme propagation (2026-08-06): the palette now accepts an
explicit theme token projection, and both live draw paths pass the
actor-owned status theme. The default constructor remains GrokNight for YAML
compatibility; production overlays no longer bypass theme changes.

Live overlay theme propagation (2026-08-06): the header meter, doctor hint,
shortcut bar, and ready footer now receive the actor-owned theme explicitly
and resolve all foreground/background/muted styles through Opaline token
projections. The active-screen helper layer no longer silently falls back to
GrokNight after a theme change. `just ci` passes with 119 TUI unit tests and
the complete replay/visual suite.

Specialized tool-card progress (2026-08-06): memory search and workflow calls
now use Grok-specific semantic headers and completion vocabulary, and the YAML
runner can exercise both without recompilation. Transport-only running status
updates are treated as card state rather than transcript text, preventing
`{"status":"running"}` from leaking into the feed. The new
`visual-specialized-tools.yaml` fixture and the full-mode tool snapshot cover
the corrected event sequence and rendered output.

Status chrome theme propagation (2026-08-06): `TurnStatus` and the status
footer now resolve spinner, label, shortcut, and loading styles from the
actor-selected theme. A GrokDay regression renders both the footer and active
turn row and asserts their Opaline foreground tokens; the previous helpers
were still using terminal-default styles after a theme event. The GrokNight
default path intentionally preserves Grok's terminal-default foreground
attributes, while alternate themes use explicit Opaline tokens.

Architecture audit note: `PromptActor` and `UiActor` own mailbox/watch state,
but `Scrollback` and `StatusBar` are still shared behind `parking_lot::Mutex`
and are mutated by `EventRenderer` and the render loop. This is a remaining
SSOT/MVU migration target, not evidence of completed actor parity.

The first migration seam is now in place for status: `StatusMsg` is the
explicit reducer input, `StatusBar::apply` is the pure transition boundary,
and the reducer has regression coverage. The next step is moving that reducer
behind a mailbox/watch actor and replacing the compatibility mutex with its
read-only snapshot.

`StatusActor` now provides that mailbox/watch boundary with owned task
lifetime, acknowledged `StatusMsg` application, and a snapshot regression
test. Existing rendering still uses the compatibility path; migrating
`EventRenderer` and the live/YAML renderers to this handle is the next step.

Theme variant preservation (2026-08-06): the status actor now stores the
declared `ThemeKind` as its source of truth rather than reconstructing it from
an Opaline theme display name. TokyoNight, RosePineMoon, OscuraMidnight, and
Auto no longer silently collapse back to GrokNight when the status view is
rendered; a unit matrix covers every declared variant.

Terminal capability progress (2026-08-06): added a pure `ColorLevel` model
(`None`, `Basic`, `Ansi256`, `TrueColor`) with deterministic RGB quantization
and environment detection matching Grok's `COLORTERM`/`TERM` boundary. The
live binary quantizes its completed frame before writing to the PTY; test/YAML
buffers remain truecolor so reference casts stay deterministic. Unit tests
cover all levels and preservation of indexed/default colors.

Typed waiting parity (2026-08-06): Grok's distinct waiting subjects are now
preserved from `AgentEvent::Waiting` through the status actor into
`TurnStatus`; model, subagent, task-output, tasks-complete, and sleep labels
are no longer rendered as one generic response wait. YAML replays all typed
events in `visual-waiting-reasons.yaml`.

Background lifecycle parity (2026-08-06): typed events now model Grok's
terminal `cancelled` state and deterministic elapsed durations. YAML replay
accepts optional `elapsed_ms` on terminal events plus `background_cancel`, and
the visual fixture asserts completed, failed, and cancelled labels including
duration text and error styling.

Semantic boundary audit (2026-08-06): the four subagent lifecycle events and
actor-owned running row are now documented in `parity/tui/tool-card.md`. The
remaining gap is Grok's full foldable block/member-card navigation model; no
duplicate presentation state was introduced in the reducer.

Failure-detail parity (2026-08-06): failed background events preserve an
optional provider error and render Grok's `(error)` suffix; the YAML fixture
asserts the exact terminal row.

Actor-owned fold intent (2026-08-06): added `ScrollbackMsg::ToggleToolMode`,
which cycles a selected tool block between Grok-compatible expanded and
collapsed states while preserving truncated output as retained state. The
transition is pure reducer state owned by `ScrollbackActor`; selected-entry
navigation and the live key-to-selection wiring remain open.

Typed card-family projection (2026-08-06): `ToolBlock` now exposes a
theme-independent `ToolCardKind` (`execute`, `read`, `edit`, `list_dir`,
`search`, web, background, or generic), and mixed/truncated YAML replays pin
the ordered family vector. This makes the next specialized renderer work an
explicit event/state contract rather than header-string matching in tests.

Web-card classification correction (2026-08-06): web-search is checked before
the generic search prefix, matching Grok's distinct `Web Search` card family.
The web-search and web-fetch YAML scenarios now assert their typed families.
