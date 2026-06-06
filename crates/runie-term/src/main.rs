//! Runie Terminal - Async Binary Entry Point
//! Architecture: Main = async runtime, UI = background thread via channel

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

    // Channel: main thread sends state snapshots to UI thread
    let (render_tx, render_rx) = std::sync::mpsc::channel::<AppState>();
    let running = Arc::new(AtomicBool::new(true));
    let ui_running = running.clone();

    // UI thread - renders snapshots from channel, no locks!
    let _ui_handle = thread::spawn(move || {
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = match Terminal::new(backend) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("[UI] Failed to create terminal: {}", e);
                return;
            }
        };

        // Wait for first snapshot
        match render_rx.recv() {
            Ok(state) => {
                let _ = terminal.draw(|f| runie_tui::ui::view(f, &state));
            }
            Err(_) => return,
        }

        let mut frame_count = 0u64;
        let mut last_log = std::time::Instant::now();
        let mut last_state: Option<AppState> = None;

        while ui_running.load(Ordering::Relaxed) {
            // Try non-blocking recv first
            match render_rx.try_recv() {
                Ok(state) => {
                    last_state = Some(state);
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // No new state, render last known if any
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    eprintln!("[UI] Channel disconnected");
                    break;
                }
            }

            // Render if we have state
            if let Some(ref state) = last_state {
                let draw_start = std::time::Instant::now();
                match terminal.draw(|f| runie_tui::ui::view(f, state)) {
                    Ok(_) => {
                        frame_count += 1;
                        let draw_time = draw_start.elapsed();
                        if draw_time > Duration::from_millis(50) {
                            eprintln!("[UI] Slow draw: {:?} (msgs={})", draw_time, state.messages.len());
                        }
                    }
                    Err(e) => {
                        eprintln!("[UI] Draw error: {}", e);
                    }
                }
            }

            thread::sleep(Duration::from_millis(50));

            if last_log.elapsed() > Duration::from_secs(10) {
                eprintln!("[UI] {} frames rendered", frame_count);
                last_log = std::time::Instant::now();
                frame_count = 0;
            }
        }

        eprintln!("[UI] Shutdown");

        // Final render
        if let Some(state) = last_state {
            let _ = terminal.draw(|f| runie_tui::ui::view(f, &state));
        }
    });

    // Main thread = async runtime
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        // State owned by main thread only
        let mut state = AppState::default();

        // Send initial state
        let _ = render_tx.send(state.clone());

        // Channels
        let (input_tx, mut input_rx) = mpsc::channel::<CoreEvent>(100);
        let (agent_tx, mut agent_rx) = mpsc::channel::<CoreEvent>(100);
        let (cmd_tx, cmd_rx) = mpsc::channel::<AgentCommand>(10);

        // Agent loop - async
        tokio::spawn(async move {
            agent_loop(cmd_rx, agent_tx).await;
        });

        // Input reader - async
        let input_tx_clone = input_tx.clone();
        tokio::spawn(async move {
            let mut reader = EventStream::new();
            while let Some(Ok(event)) = reader.next().await {
                if let Some(evt) = convert_event(&event) {
                    if input_tx_clone.send(evt).await.is_err() {
                        break;
                    }
                }
            }
        });

        // Event loop
        let mut anim_interval = tokio::time::interval(Duration::from_millis(200));
        let mut last_render = std::time::Instant::now();

        loop {
            tokio::select! {
                biased;

                Some(evt) = input_rx.recv() => {
                    state = runie_core::update::update(state, evt.clone());

                    if matches!(evt, CoreEvent::Submit) {
                        if let Some((content, id)) = state.peek_queue() {
                            state.pop_queue();
                            state.streaming = true;
                            let _ = cmd_tx.send(AgentCommand { content, id }).await;
                        }
                    }

                    if matches!(evt, CoreEvent::Quit) {
                        break;
                    }
                }

                Some(evt) = agent_rx.recv() => {
                    state = runie_core::update::update(state, evt);
                }

                _ = anim_interval.tick() => {
                    if state.turn_active {
                        state.animation_frame = state.animation_frame.wrapping_add(1);
                    }
                }
            }

            // Send snapshot to UI thread (unbounded channel, never blocks)
            // Rate limit: max 20fps
            if last_render.elapsed() >= Duration::from_millis(50) {
                if render_tx.send(state.clone()).is_err() {
                    eprintln!("[MAIN] UI thread disconnected");
                    break;
                }
                last_render = std::time::Instant::now();
            }

            // Check quit
            if matches!(state.messages.last(), Some(runie_core::ChatMessage { role, .. }) if role == "quit") {
                break;
            }
        }

        running.store(false, Ordering::Relaxed);
    });

    Ok(())
}

/// Agent loop
async fn agent_loop(
    mut cmd_rx: mpsc::Receiver<AgentCommand>,
    agent_tx: mpsc::Sender<CoreEvent>,
) {
    while let Some(cmd) = cmd_rx.recv().await {
        if needs_tool_execution(&cmd.content) {
            run_tool_flow(&cmd, &agent_tx).await;
        } else {
            run_simple_flow(&cmd, &agent_tx).await;
        }

        let _ = agent_tx.send(CoreEvent::AgentDone { id: cmd.id }).await;
    }
}

/// Simple response flow
async fn run_simple_flow(cmd: &AgentCommand, agent_tx: &mpsc::Sender<CoreEvent>) {
    let _ = agent_tx.send(CoreEvent::AgentThinking { id: cmd.id.clone() }).await;
    thinking_delay().await;

    let _ = agent_tx.send(CoreEvent::AgentThoughtDone { id: cmd.id.clone() }).await;

    let provider = MockProvider;
    let messages = vec![runie_agent::Message::User { content: cmd.content.clone() }];
    let chunks = provider.generate(messages);

    for chunk in chunks {
        let _ = agent_tx.send(CoreEvent::AgentResponse {
            id: cmd.id.clone(),
            content: chunk.content,
        }).await;
        chunk_delay().await;
    }
}

/// Tool execution flow
async fn run_tool_flow(cmd: &AgentCommand, agent_tx: &mpsc::Sender<CoreEvent>) {
    use std::time::Instant;

    let turn_start = Instant::now();
    let tool_start = Instant::now();

    let _ = agent_tx.send(CoreEvent::AgentThinking { id: cmd.id.clone() }).await;
    thinking_delay().await;
    let _ = agent_tx.send(CoreEvent::AgentThoughtDone { id: cmd.id.clone() }).await;

    let _ = agent_tx.send(CoreEvent::AgentToolStart {
        id: cmd.id.clone(),
        name: "list_files".to_string(),
    }).await;
    if !cfg!(test) {
        tokio::time::sleep(Duration::from_millis(500 + (rand_u32() % 1500) as u64)).await;
    }
    let _ = get_fake_file_list();
    let tool_duration = tool_start.elapsed().as_secs_f64();
    let _ = agent_tx.send(CoreEvent::AgentToolEnd { duration_secs: tool_duration }).await;
    chunk_delay().await;

    let _ = agent_tx.send(CoreEvent::AgentThinking { id: cmd.id.clone() }).await;
    thinking_delay().await;
    let _ = agent_tx.send(CoreEvent::AgentThoughtDone { id: cmd.id.clone() }).await;

    let _ = agent_tx.send(CoreEvent::AgentResponse {
        id: cmd.id.clone(),
        content: "Here are the files in your project:\n".to_string(),
    }).await;
    chunk_delay().await;

    let duration = turn_start.elapsed().as_secs_f64();
    let _ = agent_tx.send(CoreEvent::AgentTurnComplete {
        id: cmd.id.clone(),
        duration_secs: duration,
    }).await;
}

async fn thinking_delay() {
    if !cfg!(test) {
        let delay_ms = 500 + (rand_u32() % 2500);
        tokio::time::sleep(Duration::from_millis(delay_ms as u64)).await;
    }
}

async fn chunk_delay() {
    if !cfg!(test) {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

fn rand_u32() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u32
}

fn convert_event(event: &Event) -> Option<CoreEvent> {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => {
            if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
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
                    KeyCode::Home | KeyCode::End => Some(CoreEvent::Reset),
                    _ => None,
                }
            }
        }
        _ => None,
    }
}
