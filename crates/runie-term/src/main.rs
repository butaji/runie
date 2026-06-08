//! Runie Terminal — Non-blocking event loop with render actor
//!
//! Architecture (impossible to block by design):
//!   1. Event loop: single-threaded async, only async ops
//!   2. State: owned by event loop, mutable borrow per event
//!   3. Snapshot: immutable frame description (the UI DSL)
//!   4. Render actor: owns Terminal, receives Snapshots via channel
//!   5. If render is slow, old Snapshots are dropped — event loop never waits

use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use runie_agent::{AgentCommand, run_agent_turn};
use runie_core::{AppState, Event as CoreEvent, Snapshot};
use std::{io, time::Duration};
use tokio::sync::mpsc;

const ANIM_MS: u64 = 200;

struct Cleanup;

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> io::Result<()> {
    let _cleanup = Cleanup;
    let terminal = setup_terminal()?;
    let state = AppState::default();

    let (input_tx, input_rx) = mpsc::channel::<CoreEvent>(100);
    let (agent_tx, agent_rx) = mpsc::channel::<CoreEvent>(100);
    let (cmd_tx, cmd_rx) = mpsc::channel::<AgentCommand>(10);
    let (render_tx, render_rx) = mpsc::channel::<Snapshot>(1);

    tokio::spawn(agent_loop(cmd_rx, agent_tx));
    tokio::spawn(input_reader(input_tx));
    tokio::spawn(render_task(terminal, render_rx));

    event_loop(state, input_rx, agent_rx, cmd_tx, render_tx).await
}

fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
    Terminal::new(CrosstermBackend::new(std::io::stdout()))
}

async fn render_task(
    mut terminal: Terminal<CrosstermBackend<std::io::Stdout>>,
    mut render_rx: mpsc::Receiver<Snapshot>,
) {
    while let Some(snap) = render_rx.recv().await {
        let _ = terminal.draw(|f| runie_tui::ui::draw_snapshot(f, &snap));
    }
}

async fn input_reader(input_tx: mpsc::Sender<CoreEvent>) {
    let mut reader = EventStream::new();
    while let Some(Ok(event)) = reader.next().await {
        if let Some(evt) = convert_event(&event) {
            let is_quit = matches!(&evt, CoreEvent::Quit | CoreEvent::Reset);
            if input_tx.send(evt).await.is_err() { break; }
            if is_quit { break; }
        }
    }
}

async fn event_loop(
    mut state: AppState,
    mut input_rx: mpsc::Receiver<CoreEvent>,
    mut agent_rx: mpsc::Receiver<CoreEvent>,
    cmd_tx: mpsc::Sender<AgentCommand>,
    render_tx: mpsc::Sender<Snapshot>,
) -> io::Result<()> {
    let mut anim = tokio::time::interval(Duration::from_millis(ANIM_MS));

    // Initial draw so the user sees the app immediately, without waiting
    // for the first keyboard event.
    state.ensure_fresh();
    let _ = render_tx.try_send(state.snapshot());

    loop {
        tokio::select! {
            biased;

            Some(evt) = input_rx.recv() => {
                let was_submit = matches!(evt, CoreEvent::Submit);
                let was_followup = matches!(evt, CoreEvent::FollowUp);
                let was_quit = matches!(evt, CoreEvent::Quit | CoreEvent::Reset);
                state.update(evt);
                if was_submit || was_followup {
                    spawn_if_queued(&mut state, &cmd_tx).await;
                }
                if was_quit {
                    return Ok(());
                }
            }

            Some(evt) = agent_rx.recv() => {
                let was_done = matches!(evt, CoreEvent::AgentDone { .. } | CoreEvent::AgentError { .. });
                state.update(evt);
                if was_done {
                    spawn_if_queued(&mut state, &cmd_tx).await;
                }
            }

            _ = anim.tick(), if state.turn_active => {
                state.tick_animation();
            }
        }

        state.ensure_fresh();
        let snap = state.snapshot();
        if render_tx.try_send(snap).is_err() {
            // Render task is behind — old snapshot dropped, latest will draw
        }
    }
}

async fn agent_loop(mut cmd_rx: mpsc::Receiver<AgentCommand>, agent_tx: mpsc::Sender<CoreEvent>) {
    while let Some(cmd) = cmd_rx.recv().await {
        let agent_tx_clone = agent_tx.clone();
        let cmd_id = cmd.id.clone();

        let result = run_agent_turn(
            &cmd,
            |evt| {
                let core_evt = evt.to_core_event();
                let tx = agent_tx_clone.clone();
                tokio::spawn(async move { let _ = tx.send(core_evt).await; });
            },
            5,
        ).await;

        if let Err(e) = result {
            let _ = agent_tx.send(CoreEvent::AgentError {
                id: cmd_id,
                message: format!("Agent error: {}", e),
            }).await;
        }
    }
}

async fn spawn_if_queued(state: &mut AppState, cmd_tx: &mpsc::Sender<AgentCommand>) {
    if let Some((content, id)) = state.peek_queue() {
        let content = content.clone();
        let id = id.clone();
        state.pop_queue();
        state.streaming = true;
        state.turn_active = true;
        state.inflight += 1;
        let _ = cmd_tx.send(AgentCommand {
            content,
            id,
            provider: state.current_provider.clone(),
            model: state.current_model.clone(),
        }).await;
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, KeyEventKind};

    #[test]
    fn animation_interval_is_200ms() {
        assert_eq!(super::ANIM_MS, 200, "ANIM_MS must be 200ms for visible braille spinner, got {}", super::ANIM_MS);
    }

    #[test]
    fn ctrl_shift_e_converts_to_toggle_expand() {
        let key = KeyEvent::new(KeyCode::Char('E'), KeyModifiers::CONTROL | KeyModifiers::SHIFT);
        let event = crossterm::event::Event::Key(key);
        let result = super::convert_event(&event);
        assert!(matches!(result, Some(super::CoreEvent::ToggleExpand)), "Ctrl+Shift+E should map to ToggleExpand, got {:?}", result);
    }

    #[test]
    fn ctrl_e_converts_to_toggle_expand_for_terminals_without_shift() {
        let key = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL);
        let event = crossterm::event::Event::Key(key);
        let result = super::convert_event(&event);
        assert!(matches!(result, Some(super::CoreEvent::ToggleExpand)), "Ctrl+E should map to ToggleExpand (terminal fallback), got {:?}", result);
    }

    #[test]
    fn ctrl_c_converts_to_quit() {
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let event = crossterm::event::Event::Key(key);
        let result = super::convert_event(&event);
        assert!(matches!(result, Some(super::CoreEvent::Quit)), "Ctrl+C should map to Quit");
    }

    #[test]
    fn plain_e_not_converted() {
        let key = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::empty());
        let event = crossterm::event::Event::Key(key);
        let result = super::convert_event(&event);
        assert!(matches!(result, Some(super::CoreEvent::Input('e'))), "Plain e should map to Input");
    }

    #[test]
    fn ctrl_e_does_not_conflict_with_quit() {
        let key = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL);
        let event = crossterm::event::Event::Key(key);
        let result = super::convert_event(&event);
        assert!(!matches!(result, Some(super::CoreEvent::Quit)), "Ctrl+E should NOT map to Quit");
    }

    #[tokio::test]
    async fn spawn_if_queued_sets_turn_active_and_inflight() {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<super::AgentCommand>(10);
        let mut state = super::AppState::default();
        state.request_queue.push_back(("hello".to_string(), "req.0".to_string()));

        assert!(!state.turn_active);
        assert_eq!(state.inflight, 0);

        super::spawn_if_queued(&mut state, &tx).await;

        assert!(state.turn_active, "spawn_if_queued must set turn_active");
        assert_eq!(state.inflight, 1, "spawn_if_queued must increment inflight");
        assert!(state.request_queue.is_empty(), "Message should be popped from request_queue");

        let cmd = rx.try_recv().expect("Command should be sent to agent");
        assert_eq!(cmd.content, "hello");
    }

    #[tokio::test]
    async fn spawn_if_queued_noop_when_queue_empty() {
        let (tx, _rx) = tokio::sync::mpsc::channel::<super::AgentCommand>(10);
        let mut state = super::AppState::default();

        super::spawn_if_queued(&mut state, &tx).await;

        assert!(!state.turn_active);
        assert_eq!(state.inflight, 0);
    }

    #[test]
    fn ctrl_shift_e_on_repeat_kind_still_works() {
        let key = KeyEvent::new_with_kind(KeyCode::Char('E'), KeyModifiers::CONTROL | KeyModifiers::SHIFT, KeyEventKind::Repeat);
        let event = crossterm::event::Event::Key(key);
        let result = super::convert_event(&event);
        assert!(matches!(result, Some(super::CoreEvent::ToggleExpand)), "Ctrl+Shift+E with Repeat kind should still map to ToggleExpand, got {:?}", result);
    }

    #[test]
    fn ctrl_e_on_repeat_kind_still_works() {
        let key = KeyEvent::new_with_kind(KeyCode::Char('e'), KeyModifiers::CONTROL, KeyEventKind::Repeat);
        let event = crossterm::event::Event::Key(key);
        let result = super::convert_event(&event);
        assert!(matches!(result, Some(super::CoreEvent::ToggleExpand)), "Ctrl+E with Repeat kind should still map to ToggleExpand, got {:?}", result);
    }
}

fn convert_event(event: &Event) -> Option<CoreEvent> {
    log_key_event(event);
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat => {
            map_key_event(key)
        }
        _ => None,
    }
}

fn log_key_event(event: &Event) {
    if let Event::Key(key) = event {
        if std::env::var("RUNIE_DEBUG").is_ok() {
            use std::io::Write;
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/runie_keys.log")
                .and_then(|mut f| writeln!(f, "{:?}", key));
        }
    }
}

fn map_key_event(key: &KeyEvent) -> Option<CoreEvent> {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        map_ctrl_key(key.code)
    } else if key.modifiers.contains(KeyModifiers::ALT) {
        map_alt_key(key.code)
    } else {
        map_plain_key(key.code)
    }
}

fn map_ctrl_key(code: KeyCode) -> Option<CoreEvent> {
    match code {
        // Ctrl+E: toggle expand (also Ctrl+Shift+E on terminals with proper modifier)
        KeyCode::Char('e') | KeyCode::Char('E') => Some(CoreEvent::ToggleExpand),
        // Ctrl+A: move cursor to start (Emacs)
        KeyCode::Char('a') | KeyCode::Char('A') => Some(CoreEvent::CursorStart),
        // Ctrl+E: move cursor to end (Emacs)
        KeyCode::Char('b') | KeyCode::Char('B') => Some(CoreEvent::CursorLeft),
        // Ctrl+F: move cursor forward (Emacs) - also Ctrl+Right
        KeyCode::Char('f') | KeyCode::Char('F') => Some(CoreEvent::CursorRight),
        // Ctrl+W: delete word (Emacs)
        KeyCode::Char('w') | KeyCode::Char('W') => Some(CoreEvent::DeleteWord),
        // Ctrl+K: delete to end (Emacs)
        KeyCode::Char('k') | KeyCode::Char('K') => Some(CoreEvent::DeleteToEnd),
        // Ctrl+U: delete to start (Emacs)
        KeyCode::Char('u') | KeyCode::Char('U') => Some(CoreEvent::DeleteToStart),
        // Ctrl+D: delete char at cursor (Emacs)
        KeyCode::Char('d') | KeyCode::Char('D') => Some(CoreEvent::KillChar),
        // Ctrl+C: quit
        KeyCode::Char('c') | KeyCode::Char('C') => Some(CoreEvent::Quit),
        // Ctrl+S: abort
        KeyCode::Char('s') | KeyCode::Char('S') => Some(CoreEvent::Abort),
        _ => None,
    }
}

fn map_alt_key(code: KeyCode) -> Option<CoreEvent> {
    match code {
        KeyCode::Enter => Some(CoreEvent::FollowUp),
        _ => None,
    }
}

fn map_plain_key(code: KeyCode) -> Option<CoreEvent> {
    match code {
        KeyCode::Esc => Some(CoreEvent::Abort),
        KeyCode::Char('\t') | KeyCode::Tab | KeyCode::BackTab => Some(CoreEvent::Input('\t')),
        KeyCode::Char(c) => Some(CoreEvent::Input(c)),
        KeyCode::Backspace => Some(CoreEvent::Backspace),
        KeyCode::Enter => Some(CoreEvent::Submit),
        KeyCode::Up => Some(CoreEvent::HistoryPrev),
        KeyCode::Down => Some(CoreEvent::HistoryNext),
        KeyCode::Left => Some(CoreEvent::CursorLeft),
        KeyCode::Right => Some(CoreEvent::CursorRight),
        KeyCode::Home => Some(CoreEvent::CursorStart),
        KeyCode::End => Some(CoreEvent::CursorEnd),
        KeyCode::Delete => Some(CoreEvent::KillChar),
        _ => None,
    }
}
