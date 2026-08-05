# Step 08: App loop

**Status:** pending
**Depends on:** 07

## Goal
The `App` struct + `run()` function that wires input + bus + tasks together.

## Changes
- `crates/runie-tui/src/app.rs`:
  - `pub struct App { scrollback, prompt, status, loop_actor: LoopActor, bus_rx, input_rx }`.
  - `pub async fn run(&mut self, terminal: &mut Terminal<...>) -> AppExit`:
    - Spawns `event_renderer::run` on the bus receiver.
    - Spawns input reader that forwards crossterm key events to an `mpsc::Sender<Action>`.
    - Main loop: `tokio::select!` over `action_rx`, `renderer_done`, terminal resize.
    - On `Submit(text)`: call `loop_actor.prompt(...)`. Awaiting the result drives the bus → scrollback updates.
    - On `Abort`: `loop_actor.abort()`.
    - On `Quit` / `Ctrl+D`: break.
    - On every iteration: `terminal.draw(|f| f.render_widget(...))` — render scrollback, prompt, status into the layout regions.
  - `AppExit` enum: `Quit`, `Error(anyhow::Error)`.

## Verification
- Integration test in step 09 drives the App via TestBackend; here, just `cargo check -p runie-tui`.

## Notes
- The `LoopActor` from `runie-core` already publishes events to the bus; we just subscribe.
- For redraw cadence, draw on every action processed (no `sleep` — keep deterministic).