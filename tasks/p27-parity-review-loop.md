# p27 — Independent parity review loop

## Objective

Run a short, repeatable review after every parity change. The review is
limited to pi/core behavior and the Grok UI behavior that renders pi/core
events. It prevents a green narrow fixture from being mistaken for complete
parity and keeps the next change focused on the largest proven gap.

## Review order

1. Compare the relevant pi source path with `runie-core` event/state behavior.
2. Compare the corresponding Grok source path with the Runie view projection.
3. Require a YAML event sequence and state assertion for the behavior.
4. Require a full-screen dump assertion at all four default geometries when
   the behavior affects layout, wrapping, colors, or animation.
5. Promote symbol comparison to attribute comparison only when the reference
   capture proves truecolor capability.
6. Run the focused fixture, then `just ci`, then record the exact remaining
   gap and next source locations here or in its owning task.

## Gap ranking

Rank each open item by: source certainty, user-visible impact, number of
affected states/geometries, and test leverage. Work the highest product first;
do not spend an iteration polishing a lower-ranked component while an
unverified full-screen frame or core event family remains open.

## Current review (2026-08-06)

- Core replay gate: green for the current YAML trace inventory; exhaustive
  pi trace coverage remains open under p19.
- TUI symbol gate: green for the checked-in Grok fixtures and four-geometry
  `Hey` matrix.
- TUI attribute gate: open. `grok-rich.cast` has terminal-default SGR; a
  temporary `exact_attributes: true` promotion produced 518 differences.
  Fresh truecolor captures are required before claiming color parity.
- Fresh local capture check (2026-08-06): both default and explicit
  `--fullscreen --always-approve` Grok captures through the tmux harness still
  emitted only default/SGR-1/SGR-2 styling, with no RGB SGR sequences. This
  confirms the missing color oracle is a Grok capture/configuration issue, not
  a comparator omission; the raw artifacts are in `/tmp/` for inspection.
- Source resolution (2026-08-06): Grok's renderer engages
  `theme::cache::terminal_native_locked()` in minimal/scrollback-native mode.
  `Theme::current()` then returns the terminal-default palette and quantizes
  to the detected terminal level. Runie currently always emits explicit
  Opaline RGB styles. The next implementation seam is an actor-owned terminal
  capability/theme mode that selects the same native palette when paritying
  Grok minimal mode; forcing RGB in the capture would test a different Grok
  mode, not the one currently being compared.
- Native theme seam (2026-08-06): added `ThemeKind::TerminalNative` to the
  event/state model. Opaline semantic intents resolve to terminal-reset colors
  for this mode, while the existing GrokNight/GrokDay tokens remain unchanged.
  `visual-grok-feed.yaml` now selects the mode through a theme event, and both
  the YAML discovery test and the Grok feed visual assertion pass.
- Live selector (2026-08-06): `runie --terminal-native` now publishes the same
  `ThemeChanged` event used by YAML and waits cooperatively for the status and
  scrollback actor snapshots to acknowledge it. Full-mode behavior remains the
  default, so the four-geometry RGB fixtures are unchanged.
- Reproducible command (2026-08-06): `just tui-native` exposes that live mode
  for tmux/asciinema captures and manual Grok-vs-Runie comparison.
- Fresh native comparison (2026-08-06): paired 80×24 casts produced
  `different_attributes: 0`; the remaining 351 differences were glyph-only
  and concentrated in settled transcript rows. The Grok cast selected its
  post-exit blank frame while Runie retained the settled frame, so the next
  instrument step is marker-locked final-frame selection (`Worked for` plus
  the settled footer), not another color change.
- Comparator improvement (2026-08-06): `cast_compare --frames-after=` now
  accepts `&&`-joined markers and requires all of them to be visible before
  indexing frames. A regression test covers the combined marker path, making
  settled-frame selection data-driven rather than timing-based.
- Combined-marker result (2026-08-06): the fresh 80×24 run reaches the same
  settled phase in both casts, but still has 4 Grok frames versus 2 Runie
  frames. The first paired delta is one dynamic telemetry digit at `(69,1)`;
  this is now classified as elapsed/token-clock parity, requiring deterministic
  event-owned timing inputs before a cast-wide exact claim.
- Architecture gate: production status/scrollback projections are actor-owned
  watch snapshots; declarative view/render separation and legacy adapter
  removal remain open under p23/p26.

## Acceleration rule

Every iteration must either close one ranked gap or improve the instrument
that measures it. A passing test that does neither is not a parity iteration.
