//! Runie Terminal - Async Binary Entry Point
//! Main thread owns state, UI thread renders snapshots via channel

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use runie_agent::{get_fake_file_list, needs_tool_execution, AgentCommand, MockProvider, Provider};
use runie_core::{AppState, Event as CoreEvent};
use std::{
    io,
    sync::{Arc, atomic::{AtomicBool, Ordering}},
    thread,
    time::Duration,
};
use tokio::sync::mpsc;

struct Cleanup;

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

fn main() -> io::Result<()> {
    let _cleanup = Cleanup;
    let mut stdout = std::io::stdout();
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(&mut stdout, crossterm::terminal::EnterAlternateScreen)?;

    let (render_tx, render_rx) = std::sync::mpsc::channel::<AppState>();
    let running = Arc::new(AtomicBool::new(true));
    let ui_running = running.clone();

    // UI thread — 60fps render loop
    let _ui_handle = thread::spawn(move || {
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut last_state: Option<AppState> = None;

        while ui_running.load(Ordering::Relaxed) {
            match render_rx.try_recv() {
                Ok(state) => last_state = Some(state),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }

            if let Some(ref state) = last_state {
                let _ = terminal.draw(|f| runie_tui::ui::view(f, state));
            }

            thread::sleep(Duration::from_millis(16));
        }
    });

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        let mut state = AppState::default();
        let _ = render_tx.send(state.clone());

        let (input_tx, mut input_rx) = mpsc::channel::<CoreEvent>(100);
        let (agent_tx, mut agent_rx) = mpsc::channel::<CoreEvent>(100);
        let (cmd_tx, cmd_rx) = mpsc::channel::<AgentCommand>(10);

        tokio::spawn(agent_loop(cmd_rx, agent_tx));

        let input_tx_clone = input_tx.clone();
        tokio::spawn(async move {
            let mut reader = EventStream::new();
            while let Some(Ok(event)) = reader.next().await {
                if let Some(evt) = convert_event(&event) {
                    if input_tx_clone.send(evt).await.is_err() { break; }
                }
            }
        });

        let mut anim_interval = tokio::time::interval(Duration::from_millis(16));

        loop {
            tokio::select! {
                biased;

                Some(evt) = input_rx.recv() => {
                    state = runie_core::update::update(state, evt.clone());
                    state.ensure_fresh();

                    if matches!(evt, CoreEvent::Submit) {
                        if let Some((content, id)) = state.peek_queue() {
                            state.pop_queue();
                            state.streaming = true;
                            let _ = cmd_tx.send(AgentCommand { content, id }).await;
                        }
                    }
                    if matches!(evt, CoreEvent::Quit) { break; }
                }

                Some(evt) = agent_rx.recv() => {
                    state = runie_core::update::update(state, evt);
                    state.ensure_fresh();
                }

                _ = anim_interval.tick() => {
                    if state.turn_active {
                        state.animation_frame = state.animation_frame.wrapping_add(1);
                        state.ensure_fresh();
                    }
                }
            }

            if render_tx.send(state.clone()).is_err() { break; }

            if matches!(state.messages.last(), Some(runie_core::ChatMessage { role, .. }) if role == "quit") {
                break;
            }
        }

        running.store(false, Ordering::Relaxed);
    });

    Ok(())
}

async fn agent_loop(mut cmd_rx: mpsc::Receiver<AgentCommand>, agent_tx: mpsc::Sender<CoreEvent>) {
    while let Some(cmd) = cmd_rx.recv().await {
        if needs_tool_execution(&cmd.content) {
            run_tool_flow(&cmd, &agent_tx).await;
        } else {
            run_simple_flow(&cmd, &agent_tx).await;
        }
        let _ = agent_tx.send(CoreEvent::AgentDone { id: cmd.id }).await;
    }
}

async fn run_simple_flow(cmd: &AgentCommand, agent_tx: &mpsc::Sender<CoreEvent>) {
    let _ = agent_tx.send(CoreEvent::AgentThinking { id: cmd.id.clone() }).await;
    if !cfg!(test) { tokio::time::sleep(Duration::from_millis(500)).await; }
    let _ = agent_tx.send(CoreEvent::AgentThoughtDone { id: cmd.id.clone() }).await;

    let provider = MockProvider;
    let messages = vec![runie_agent::Message::User { content: cmd.content.clone() }];
    for chunk in provider.generate(messages) {
        let _ = agent_tx.send(CoreEvent::AgentResponse { id: cmd.id.clone(), content: chunk.content }).await;
        if !cfg!(test) { tokio::time::sleep(Duration::from_millis(50)).await; }
    }
}

async fn run_tool_flow(cmd: &AgentCommand, agent_tx: &mpsc::Sender<CoreEvent>) {
    use std::time::Instant;
    let turn_start = Instant::now();

    let _ = agent_tx.send(CoreEvent::AgentThinking { id: cmd.id.clone() }).await;
    if !cfg!(test) { tokio::time::sleep(Duration::from_millis(500)).await; }
    let _ = agent_tx.send(CoreEvent::AgentThoughtDone { id: cmd.id.clone() }).await;

    let _ = agent_tx.send(CoreEvent::AgentToolStart { id: cmd.id.clone(), name: "list_files".to_string() }).await;
    if !cfg!(test) { tokio::time::sleep(Duration::from_millis(1000)).await; }
    let _ = get_fake_file_list();
    let _ = agent_tx.send(CoreEvent::AgentToolEnd { duration_secs: 1.0 }).await;
    if !cfg!(test) { tokio::time::sleep(Duration::from_millis(50)).await; }

    let _ = agent_tx.send(CoreEvent::AgentThinking { id: cmd.id.clone() }).await;
    if !cfg!(test) { tokio::time::sleep(Duration::from_millis(500)).await; }
    let _ = agent_tx.send(CoreEvent::AgentThoughtDone { id: cmd.id.clone() }).await;

    let _ = agent_tx.send(CoreEvent::AgentResponse { id: cmd.id.clone(), content: "Files:\n".to_string() }).await;
    if !cfg!(test) { tokio::time::sleep(Duration::from_millis(50)).await; }

    let _ = agent_tx.send(CoreEvent::AgentTurnComplete { id: cmd.id.clone(), duration_secs: turn_start.elapsed().as_secs_f64() }).await;
}

fn convert_event(event: &Event) -> Option<CoreEvent> {
    use crossterm::event::KeyModifiers;
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                match key.code {
                    KeyCode::Char('c') | KeyCode::Char('C')
                    | KeyCode::Char('q') | KeyCode::Char('Q')
                    | KeyCode::Char('d') | KeyCode::Char('D') => Some(CoreEvent::Quit),
                    _ => None,
                }
            } else {
                match key.code {
                    KeyCode::Esc => Some(CoreEvent::Quit),
                    KeyCode::Char(c) => Some(CoreEvent::Input(c)),
                    KeyCode::Backspace => Some(CoreEvent::Backspace),
                    KeyCode::Enter => Some(CoreEvent::Submit),
                    KeyCode::Up => Some(CoreEvent::ScrollUp),
                    KeyCode::Down => Some(CoreEvent::ScrollDown),
                    _ => None,
                }
            }
        }
        _ => None,
    }
}
