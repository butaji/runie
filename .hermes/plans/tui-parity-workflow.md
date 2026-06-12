# Grok parity work — TUI parity with Grok Build

## The bacon REPL loop

You have **bacon 3.23.0** installed (`/Users/admin/.cargo/bin/bacon`) and
`bacon.toml` configures 4 jobs:

| Job        | What it does                                   | Speed      |
|------------|------------------------------------------------|------------|
| `bacon`    | `cargo check -p runie-tui`                     | <1s        |
| `bacon run`| Run the actual TUI binary                      | 3-5s       |
| `bacon test`| Run `grok_parity_test` (47 → 52+ assertions)  | 3-5s       |
| `bacon check`| Full workspace `cargo check`                  | 10-30s     |

**In your terminal** (not the Hermes agent shell — bacon draws a TUI):

```bash
cd /Users/admin/Code/GitHub/runie

bacon            # default job: fast type-check on the runie-tui crate
bacon test       # see all 47+ visual assertions
bacon run        # launch the actual TUI binary
```

Inside bacon:
- `?` for help
- `t` → switch to test
- `c` → switch to clippy
- `r` → run job once more
- `s` → toggle summary
- `q` → quit

## The wireframe debugger

For layout-first work, drop a `wireframe_box(buf, "name", area)` call
right after the actual render. Enable with env var:

```bash
RUNIE_WIREFRAME=1 runie                    # overlay everything
RUNIE_WIREFRAME=top_bar runie              # overlay just top_bar
RUNIE_WIREFRAME=input,top_bar runie        # overlay specific components
```

Already wired into `pipe/render/modes.rs:60,69` for the home screen.
Add more `wireframe_box()` calls anywhere — they no-op when off.

## The parity test

`grok_parity_test` is the 47-assertion visual regression suite:

```bash
RUNIE_SKIP_BUILD_CHECKS=1 \
  cargo run -p runie-tui --bin grok_parity_test --release
```

When you change a visual element, run this FIRST. The test renders
into a `TestBackend` and asserts the output matches the Grok spec.
Add a new assertion for any new behavior.

## The dumps

`ui/dumps/grok/*.txt` is the reference: 47 captures of the actual
Grok Build TUI. When a dump disagrees with runie's output, that's
a P0. Read the dump, then read runie's source for the same component,
then add a parity test assertion to lock the fix.

`.hermes/plans/grok-look-feel-parity-v2.md` lists all open P0-P2
items with dump evidence and source locations.
