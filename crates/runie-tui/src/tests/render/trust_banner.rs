//! Layer 3 rendering tests for the trust banner.

use ratatui::{backend::TestBackend, Terminal};

use runie_core::model::{AppState, ChatMessage, Role};
use runie_core::Event;
use runie_core::Part;

fn render_state(state: &mut AppState, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| crate::ui::view(f, state)).expect("draw");
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect()
}

fn add_trust_banner(state: &mut AppState) {
    state.config.read_only = true;
    state.config.current_provider = "openai".into();
    state.config.current_model = "gpt-4o".into();
    state.session.messages.push(ChatMessage {
        role: Role::System,
        parts: vec![Part::Text {
            content: "Welcome to runie in someproject.\n\nThis project is not yet trusted. \
                  Run /trust to enable write tools, or /untrust to enforce read-only mode."
                .into(),
        }],
        timestamp: 0.0,
        id: "trust_welcome".into(),
        ..Default::default()
    });
    state.messages_changed();
}

#[test]
fn trust_banner_renders_when_untrusted() {
    let mut state = AppState::default();
    crate::tests::connect_model(&mut state);
    add_trust_banner(&mut state);
    state.ensure_fresh();
    state.view.scroll = 0;

    let out = render_state(&mut state, 60, 20);
    assert!(
        out.contains("not yet trusted"),
        "Trust banner should render for untrusted project: {}",
        out
    );
    assert!(
        out.contains("🔒"),
        "Status bar should show read-only indicator: {}",
        out
    );
}

#[test]
fn trust_command_removes_banner_and_read_only_indicator() {
    let mut state = AppState::default();
    crate::tests::connect_model(&mut state);
    add_trust_banner(&mut state);
    state.ensure_fresh();
    state.view.scroll = 0;

    state.update(Event::Input('/'));
    state.update(runie_core::event::Event::PaletteFilter('t'));
    state.update(runie_core::event::Event::PaletteFilter('r'));
    state.update(runie_core::event::Event::PaletteFilter('u'));
    state.update(runie_core::event::Event::PaletteFilter('s'));
    state.update(runie_core::event::Event::PaletteFilter('t'));
    state.update(runie_core::event::Event::PaletteSelect);
    state.ensure_fresh();
    state.view.scroll = 0;

    assert!(
        !state.config.read_only,
        "/trust should disable read-only mode"
    );
    let out = render_state(&mut state, 60, 20);
    assert!(
        !out.contains("not yet trusted"),
        "Trust banner should disappear after /trust: {}",
        out
    );
    assert!(
        !out.contains("🔒"),
        "Read-only indicator should disappear from status bar: {}",
        out
    );
}
