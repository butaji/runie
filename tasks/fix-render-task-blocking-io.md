# Fix render task blocking the Tokio runtime on terminal IO

**Status**: done
**Milestone**: R3
**Category**: TUI / Rendering
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

`render_task` in `crates/runie-tui/src/main.rs` calls `terminal.size()`, `terminal.clear()`, and `terminal.draw()` from within a Tokio task. These are synchronous terminal writes that can block a worker thread. The documented goal is an event loop that cannot block; the render path should run on a dedicated OS thread.

## Acceptance Criteria

- [ ] Terminal IO no longer runs on the Tokio runtime.
- [ ] Snapshots still render on every change.
- [ ] Shutdown still terminates the render loop cleanly.
- [ ] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- N/A.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- [ ] Existing `TestBackend` rendering tests continue to pass.
- [ ] If feasible, add a smoke test that sends a `Snapshot` through the render channel and observes no panic (the TUI integration tests already exercise this).

### Layer 4 — Provider Replay / E2E
- N/A.

## Files touched

- `crates/runie-tui/src/main.rs`

## Implementation

### Step 1: Change `render_task` to spawn a dedicated thread

Convert `render_task` to a function that spawns a `std::thread` and returns a `JoinHandle`:

```rust
fn spawn_render_task(
    terminal: ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    mut render_rx: watch::Receiver<Snapshot>,
    caps: terminal::caps::TerminalCapabilities,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut last_size: Option<(u16, u16)> = None;
        loop {
            let snap = render_rx.borrow_and_update().clone();
            let new_size = terminal
                .size()
                .map(|r| (r.width, r.height))
                .unwrap_or((0, 0));
            if last_size != Some(new_size) {
                let _ = terminal.clear();
                last_size = Some(new_size);
            }
            theme::set_current_theme_with_caps(&snap.theme_name, caps);
            let _ = terminal.draw(|f| ui::draw_snapshot(f, &snap));
            if render_rx.changed().is_err() {
                break;
            }
        }
    })
}
```

Note: `render_rx.changed()` is async, but `watch::Receiver::changed` can be polled from a non-async context via `block_on` if the receiver is moved into the thread. Since the thread is not on the Tokio runtime, use a blocking receive strategy instead. Convert the `watch` channel to a `std::sync::mpsc` channel, or use `tokio::runtime::Handle::try_current().unwrap().block_on(render_rx.changed())` inside the thread (the runtime is multi-threaded, so blocking on it from a dedicated thread is acceptable).

Simpler option: keep the Tokio task as a forwarder and use a `std::sync::mpsc` channel for the dedicated thread:

```rust
async fn render_task(
    terminal: Terminal<CrosstermBackend<Stdout>>,
    render_rx: watch::Receiver<Snapshot>,
    caps: TerminalCapabilities,
) {
    let (tx, rx) = std::sync::mpsc::channel::<Snapshot>();
    std::thread::spawn(move || render_loop(terminal, rx, caps));

    let mut render_rx = render_rx;
    loop {
        let snap = render_rx.borrow_and_update().clone();
        if tx.send(snap).is_err() {
            break;
        }
        if render_rx.changed().await.is_err() {
            break;
        }
    }
}

fn render_loop(
    mut terminal: Terminal<CrosstermBackend<Stdout>>,
    rx: std::sync::mpsc::Receiver<Snapshot>,
    caps: TerminalCapabilities,
) {
    let mut last_size: Option<(u16, u16)> = None;
    while let Ok(snap) = rx.recv() {
        let new_size = terminal.size().map(|r| (r.width, r.height)).unwrap_or((0, 0));
        if last_size != Some(new_size) {
            let _ = terminal.clear();
            last_size = Some(new_size);
        }
        theme::set_current_theme_with_caps(&snap.theme_name, caps);
        let _ = terminal.draw(|f| ui::draw_snapshot(f, &snap));
    }
}
```

This keeps the async `watch::Receiver` polling in the Tokio task and moves only terminal IO to the dedicated thread.

### Step 2: Update the caller

Where `render_task(...)` is awaited, change to `tokio::spawn(render_task(...))` if not already spawned. Ensure the returned join handle is stored or dropped as appropriate.

### Step 3: Run tests

```bash
cargo test -p runie-tui
cargo test --workspace
```

### Step 4: Commit

```bash
git add crates/runie-tui/src/main.rs tasks/fix-render-task-blocking-io.md tasks/index.json
git commit -m "fix(tui): run terminal IO on a dedicated render thread"
```

## Notes

- `Terminal` is not `Send` in all Ratatui versions; verify with the compiler. If it is not `Send`, construct it inside the dedicated thread and pass only the backend setup parameters. The current code already creates the terminal before `render_task`; if `Terminal` is `Send`, this plan works.
- If `Terminal` is not `Send`, refactor `main.rs` to create the terminal inside the render thread and pass a `std::sync::mpsc` channel of snapshots into it.
