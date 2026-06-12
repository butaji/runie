use super::*;

fn render(state: &mut AppState, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| view(f, state)).expect("draw");
    let buf = terminal.backend().buffer();
    buf.content.iter().map(|c| c.symbol()).collect()
}

#[test]
fn agent_response_visible_after_large_tool() {
    let mut state = AppState::default();
    state.streaming = true;

    // Simulate mock provider: agent response BEFORE tool
    state.update(Event::AgentResponse {
        id: "req.0".into(),
        content: "Done!".into(),
    });
    state.update(Event::AgentToolStart {
        id: "req.0".into(),
        name: "ls".into(),
    });
    let output = (1..=20)
        .map(|i| format!("file{}.txt", i))
        .collect::<Vec<_>>()
        .join("\n");
    state.update(Event::AgentToolEnd {
        duration_secs: 0.5,
        output,
    });
    state.update(Event::AgentTurnComplete {
        id: "req.0".into(),
        duration_secs: 1.0,
    });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();
    state.view.scroll = 0;

    // Terminal 40x20 -> chat panel ~16 inner rows
    let out = render(&mut state, 40, 20);
    assert!(
        out.contains("Done!"),
        "Final agent 'Done!' must be visible after tool reorder"
    );
}

#[test]
fn agent_at_bottom_tool_files_above() {
    let mut state = AppState::default();
    state.streaming = true;

    state.update(Event::AgentResponse {
        id: "req.0".into(),
        content: "Here are the files.".into(),
    });
    state.update(Event::AgentToolStart {
        id: "req.0".into(),
        name: "ls".into(),
    });
    let output = (1..=15)
        .map(|i| format!("file{}.txt", i))
        .collect::<Vec<_>>()
        .join("\n");
    state.update(Event::AgentToolEnd {
        duration_secs: 0.5,
        output,
    });
    state.update(Event::AgentTurnComplete {
        id: "req.0".into(),
        duration_secs: 1.0,
    });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();
    state.view.scroll = 0;

    let out = render(&mut state, 40, 18);

    let file15_pos = out.find("file15");
    let agent_pos = out.find("Here are");

    assert!(file15_pos.is_some(), "Latest file must be visible");
    assert!(agent_pos.is_some(), "Agent response must be visible");
    assert!(
        agent_pos.unwrap() > file15_pos.unwrap(),
        "Agent must appear AFTER tool files in render output"
    );
}

#[test]
fn turn_complete_always_last_visible() {
    let mut state = AppState::default();
    state.streaming = true;

    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentResponse {
        id: "req.0".into(),
        content: "Done!".into(),
    });
    state.update(Event::AgentToolStart {
        id: "req.0".into(),
        name: "ls".into(),
    });
    state.update(Event::AgentToolEnd {
        duration_secs: 0.5,
        output: "a\nb\nc".into(),
    });
    state.update(Event::AgentTurnComplete {
        id: "req.0".into(),
        duration_secs: 1.0,
    });
    state.update(Event::AgentDone { id: "req.0".into() });
    state.ensure_fresh();
    state.view.scroll = 0;

    let out = render(&mut state, 40, 15);

    // TurnComplete must be visible and after everything else
    let turn_pos = out.find("Turn completed");
    let agent_pos = out.find("Done!");
    assert!(turn_pos.is_some(), "TurnComplete must be visible");
    assert!(agent_pos.is_some(), "Agent must be visible");
    assert!(
        turn_pos.unwrap() > agent_pos.unwrap(),
        "TurnComplete must be after agent"
    );
}
