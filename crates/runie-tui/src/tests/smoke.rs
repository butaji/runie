//! TUI smoke tests — verify view() integrates with core state

use crate::ui::view;
use ratatui::{backend::TestBackend, Terminal};
use runie_core::event::{AgentEvent, ControlEvent, InputEvent};
use runie_core::{AppState, Event};

pub(crate) fn draw_state(state: &mut AppState) -> String {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, state)).unwrap();
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect()
}

#[test]
fn empty_state_renders_input_prompt() {
    let mut state = AppState::default();
    let content = draw_state(&mut state);
    assert!(
        content.contains("❯ "),
        "Empty state should show input prompt"
    );
}

#[test]
fn user_message_renders() {
    let mut state = AppState::default();
    state.update(Event::Input(InputEvent::Input('H')));
    state.update(Event::Input(InputEvent::Input('i')));
    state.update(Event::Input(InputEvent::Submit));
    let content = draw_state(&mut state);
    assert!(content.contains("❯ Hi"), "Should render user prefix");
    assert!(content.contains("Hi"), "Should render message content");
}

#[test]
fn agent_response_renders() {
    let mut state = AppState::default();
    state.agent.streaming = true;
    state.update(Event::Agent(AgentEvent::Thinking {
        id: "req.0".to_string(),
    }));
    state.update(Event::Agent(AgentEvent::ThoughtDone {
        id: "req.0".to_string(),
    }));
    state.update(Event::Agent(AgentEvent::Response {
        id: "req.0".to_string(),
        content: "Hello".to_string(),
    }));
    let content = draw_state(&mut state);
    assert!(content.contains("→ Hello"), "Should render agent prefix");
}

#[test]
fn tool_done_renders() {
    let mut state = AppState::default();
    state.update(Event::Agent(AgentEvent::ToolStart {
        id: "req.0".to_string(),
        name: "list_files".to_string(),
    }));
    state.update(Event::Agent(AgentEvent::ToolEnd {
        duration_secs: 0.5,
        output: String::new(),
    }));
    let content = draw_state(&mut state);
    assert!(content.contains("✓"), "Should render tool done");
    assert!(content.contains("list_files"), "Should show tool name");
}

#[test]
fn reset_clears_messages() {
    let mut state = AppState::default();
    state.update(Event::Input(InputEvent::Input('T')));
    state.update(Event::Input(InputEvent::Submit));
    state.update(Event::Control(ControlEvent::Reset));
    let content = draw_state(&mut state);
    let count = content.matches("❯ ").count();
    assert_eq!(
        count, 1,
        "Reset should clear messages, keep only input prompt"
    );
}

#[test]
fn status_shows_provider_model() {
    let mut state = AppState::default();
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4".to_string();
    let backend = TestBackend::new(60, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content: String = (0..buf.area().height)
        .map(|y| {
            (0..buf.area().width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
        })
        .collect();
    assert!(content.contains("openai"), "Status should show provider");
    assert!(content.contains("gpt-4"), "Status should show model");
}

#[test]
fn status_shows_thinking_badge_when_active() {
    let mut state = AppState::default();
    state.config.thinking_level = runie_core::model::ThinkingLevel::Medium;
    let backend = TestBackend::new(100, 10); // 60-wide too narrow for worktree + badge
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content: String = (0..buf.area().height)
        .map(|y| {
            (0..buf.area().width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
        })
        .collect();
    assert!(
        content.contains("Think: medium"),
        "Status should show thinking level badge: {}",
        content
    );
}

#[test]
fn status_hides_thinking_badge_when_off() {
    let mut state = AppState::default();
    state.config.thinking_level = runie_core::model::ThinkingLevel::Off;
    let backend = TestBackend::new(60, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content: String = (0..buf.area().height)
        .map(|y| {
            (0..buf.area().width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
        })
        .collect();
    assert!(
        !content.contains("Think:"),
        "Status should NOT show thinking badge when off: {}",
        content
    );
}

#[test]
fn empty_state_shows_hint() {
    let mut state = AppState::default();
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content: String = (0..buf.area().height)
        .map(|y| {
            (0..buf.area().width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
        })
        .collect();
    assert!(
        content.contains("Type a message"),
        "Empty state should show hint text"
    );
}
