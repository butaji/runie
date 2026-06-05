mod app;
mod ui;

use app::App;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use runie_agent::{
    engine::AgentLoop,
    provider::MockProvider,
    types::Message,
};
use std::{
    io,
    sync::Arc,
};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let provider = Arc::new(MockProvider);
    let agent = AgentLoop::new(provider);

    let (agent_tx, mut agent_rx) = mpsc::channel::<runie_agent::types::AgentEvent>(128);

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        if crossterm::event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.quit = true;
                        }
                        KeyCode::Char('q') | KeyCode::Char('Q') => {
                            app.quit = true;
                        }
                        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.quit = true;
                        }
                        KeyCode::Esc => {
                            app.quit = true;
                        }
                        KeyCode::Char(c) if !app.streaming => {
                            app.input.push(c);
                        }
                        KeyCode::Backspace if !app.streaming => {
                            app.input.pop();
                        }
                        KeyCode::Enter if !app.streaming && !app.input.is_empty() => {
                            let _user_text = app.input.clone();
                            app.push_user_message();

                            let messages = build_messages(&app.messages);
                            let mut stream = agent.run(messages);
                            let tx = agent_tx.clone();

                            tokio::spawn(async move {
                                while let Some(event) = stream.next().await {
                                    let _ = tx.send(event).await;
                                }
                            });
                        }
                        _ => {}
                    }
                }
            }
        }

        while let Ok(event) = agent_rx.try_recv() {
            app.handle_event(&event);
        }

        if app.quit {
            break;
        }
    }

    Ok(())
}

fn build_messages(chat: &[app::ChatMessage]) -> Vec<Message> {
    let mut msgs = vec![Message::System {
        content: "You are a helpful assistant.".into(),
    }];
    for m in chat {
        let msg = match m.role.as_str() {
            "user" => Message::User {
                content: m.content.clone(),
            },
            "assistant" => Message::Assistant {
                content: m.content.clone(),
                tool_calls: vec![],
            },
            _ => continue,
        };
        msgs.push(msg);
    }
    msgs
}
