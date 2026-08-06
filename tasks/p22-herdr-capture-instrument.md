# p22 — Herdr full-fidelity capture instrument

## Contract

The canonical timed artifact is now an asciinema cast recorded inside a
private fixed-geometry tmux PTY:

```sh
just tmux-cast 62 32 /tmp/grok-hey.cast grok Hey C-c
just tmux-cast 62 32 /tmp/runie-hey.cast target/debug/runie Hey C-q
```

The cast preserves output timing, animations, alternate-screen transitions,
RGB SGR sequences, cursor operations, and input-capable PTY behavior. A raw
ANSI frame can still be captured from a live pane with
`just herdr-dump PANE PREFIX` for diagnostics, but it is not the canonical
replay artifact.

The capture helper explicitly resizes the tmux window after creation and
passes `--window-size` to asciinema as well; the cast header is the
authoritative geometry check and must equal the requested columns and rows.

Validated with private paired captures on 2026-08-05:
`/tmp/grok-hey-private.cast` and `/tmp/runie-hey-private.cast`, both with
`62×32` headers. Input capture is enabled so the `Hey` submit and termination
keys are represented in the recordings.

`herdr-dump` captures a pane's complete visible terminal grid without
trimming blank cells or stripping SGR attributes:

- `PREFIX.ansi`: raw ANSI frame, including RGB foreground/background and
  text modifiers;
- `PREFIX.pane.json`: Herdr's pane response;
- `PREFIX.herdr.json`: workspace snapshot for layout/context;
- `PREFIX.meta.json`: normalized pane identity and viewport metadata.

The tmux/asciinema helper additionally emits a `.raw` replay stream beside
the `.cast`; this is useful for feeding a virtual terminal parser directly.

Compare the timed casts directly:

```sh
just cast-compare /tmp/grok-hey-private.cast /tmp/runie-hey-private.cast
```

The native comparator replays both casts through `vt100`, validates geometry,
and reports glyph versus attribute-only differences across the full grid.

Use the required four-size matrix for parity claims:

```sh
just capture-matrix /tmp/grok-matrix grok C-c
just capture-matrix /tmp/runie-matrix target/debug/runie C-q
for size in 62x32 80x24 100x30 120x36; do
  just cast-compare /tmp/grok-matrix/$size.cast /tmp/runie-matrix/$size.cast
done
```

The four defaults cover the Herdr viewport, narrow terminal, standard
terminal, and wide terminal. A parity change is not complete if it only
matches one geometry.

Runie matrix smoke validation produced valid v2 cast headers for all four
geometries. Grok capture readiness uses the same rendered `❯` prompt probe;
the capture helper now rejects a run that misses either the input prompt or
the completed-turn marker. Each pair must still be compared with
`cast-compare` before declaring parity.

The native comparator selects the last non-empty application frame, avoiding
false diffs from alternate-screen teardown. It compares every cell's glyph,
foreground/background color, and modifiers, then reports row and coordinate
hotspots.

`PREFIX.meta.json` includes the authoritative Herdr rectangle (`cols` and
`rows`), so trailing blank cells are not lost when a frame is inspected.

The instrument deliberately uses Herdr's `visible` source. `recent` and
`recent-unwrapped` are useful for text inspection but are not authoritative
for pixel/cell parity because they do not represent the visible viewport.

## Reproducible capture

Fix the Herdr pane geometry first, run the scenario, then capture both sides
with the same prefix convention:

```sh
just herdr-dump w73:p38 /tmp/grok-hey
just herdr-dump w73:p38 /tmp/runie-hey
```

YAML replay consumes the cast: its `reference.exact_screen` and
`reference.exact_attributes` checks compare the complete fixed cell grid
through `vt100`.

Compare captures with:

```sh
just herdr-compare /tmp/grok-hey.ansi /tmp/runie-hey.ansi 63 32
```

The command exits non-zero until every cell matches and reports total glyph,
style-only, and per-row differences.
