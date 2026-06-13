use super::*;

fn render_content(state: &mut AppState) -> String {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| view(f, state)).expect("draw");
    let buf = terminal.backend().buffer();
    buf.content.iter().map(|c| c.symbol()).collect()
}

fn dispatch(state: &mut AppState, events: &[Event]) {
    for e in events {
        state.update(e.clone());
    }
}

#[test]
fn e2e_toggle_collapses_all_thoughts_and_tools() {
    let mut state = AppState::default();
    state.agent.streaming = true;

    state.update(Event::AgentThinking {
        id: "req.0".to_string(),
    });
    state.update(Event::AgentResponse {
        id: "req.0".to_string(),
        content: "I'll list files.".to_string(),
    });
    state.update(Event::AgentResponse {
        id: "req.0".to_string(),
        content: "TOOL:list_dir:.".to_string(),
    });
    state.update(Event::AgentThoughtDone {
        id: "req.0".to_string(),
    });

    let before = render_content(&mut state);
    assert!(
        before.contains("I'll list files"),
        "Should show reasoning before toggle"
    );

    state.update(Event::ToggleExpand);
    assert!(state.view.all_collapsed, "Toggle should set global collapse");

    let after = render_content(&mut state);
    assert!(
        after.contains("[+]") || !after.contains("I'll list files"),
        "Should hide reasoning after toggle"
    );
}

#[test]
fn e2e_toggle_expands_back_on_second_press() {
    let mut state = AppState::default();
    state.agent.streaming = true;

    state.update(Event::AgentThinking {
        id: "req.0".to_string(),
    });
    state.update(Event::AgentResponse {
        id: "req.0".to_string(),
        content: "I'll list files.\n".to_string(),
    });
    state.update(Event::AgentResponse {
        id: "req.0".to_string(),
        content: "TOOL:list_dir:.".to_string(),
    });
    state.update(Event::AgentThoughtDone {
        id: "req.0".to_string(),
    });

    state.update(Event::ToggleExpand);
    let collapsed = render_content(&mut state);
    assert!(
        collapsed.contains("[+]") || !collapsed.contains("I'll list files"),
        "Collapsed thought should hide reasoning"
    );

    state.update(Event::ToggleExpand);
    let expanded = render_content(&mut state);
    assert!(!state.view.all_collapsed, "Second toggle should expand all");
    assert!(
        expanded.contains("I'll list files"),
        "Expanded thought should show reasoning"
    );
}

#[test]
fn e2e_all_collapsed_stays_collapsed_through_tool_execution() {
    let mut state = AppState::default();
    state.agent.streaming = true;
    dispatch(
        &mut state,
        &[
            Event::AgentThinking { id: "req.0".into() },
            Event::AgentResponse { id: "req.0".into(), content: "I'll list files.\n".into() },
            Event::AgentResponse { id: "req.0".into(), content: "TOOL:list_dir:.".into() },
            Event::AgentThoughtDone { id: "req.0".into() },
        ],
    );
    state.update(Event::ToggleExpand);
    assert!(state.view.all_collapsed);
    dispatch(
        &mut state,
        &[
            Event::AgentToolStart { id: "req.0".into(), name: "list_dir".into() },
            Event::AgentToolEnd { duration_secs: 0.5, output: "file1\nfile2".into() },
        ],
    );
    let during_tool = render_content(&mut state);
    assert!(
        !during_tool.contains("I'll list files"),
        "Thought should stay collapsed during tool execution"
    );
    assert!(
        !during_tool.contains("file1"),
        "Completed tool should also be collapsed with global flag"
    );
}

#[test]
fn e2e_all_collapsed_stays_collapsed_after_agent_response() {
    let mut state = AppState::default();
    state.agent.streaming = true;
    dispatch(
        &mut state,
        &[
            Event::AgentThinking { id: "req.0".into() },
            Event::AgentResponse { id: "req.0".into(), content: "I'll list files.\n".into() },
            Event::AgentResponse { id: "req.0".into(), content: "TOOL:list_dir:.".into() },
            Event::AgentThoughtDone { id: "req.0".into() },
        ],
    );
    state.update(Event::ToggleExpand);
    assert!(state.view.all_collapsed);
    dispatch(
        &mut state,
        &[
            Event::AgentToolStart { id: "req.0".into(), name: "list_dir".into() },
            Event::AgentToolEnd { duration_secs: 0.5, output: "file1".into() },
            Event::AgentResponse { id: "req.0".into(), content: "Done.".into() },
            Event::AgentDone { id: "req.0".into() },
        ],
    );
    let after_done = render_content(&mut state);
    assert!(
        !after_done.contains("I'll list files"),
        "Thought should stay collapsed after agent done"
    );
    assert!(!after_done.contains("file1"), "Tool should stay collapsed after agent done");
}

#[test]
fn e2e_new_thought_respects_global_collapse() {
    let mut state = AppState::default();
    state.agent.streaming = true;

    state.update(Event::AgentThinking {
        id: "req.0".to_string(),
    });
    state.update(Event::AgentResponse {
        id: "req.0".to_string(),
        content: "First.".to_string(),
    });
    state.update(Event::AgentThoughtDone {
        id: "req.0".to_string(),
    });

    state.update(Event::ToggleExpand);
    assert!(state.view.all_collapsed);

    state.update(Event::AgentThinking {
        id: "req.1".to_string(),
    });
    state.update(Event::AgentResponse {
        id: "req.1".to_string(),
        content: "Second.".to_string(),
    });
    state.update(Event::AgentThoughtDone {
        id: "req.1".to_string(),
    });

    let after = render_content(&mut state);
    // Both thoughts should be collapsed — only summary lines visible
    let marker_count = after.matches("Thought").count();
    let summary_count = after.matches("[+]").count();
    assert!(
        summary_count >= 2 || marker_count < 2,
        "Both thoughts should be collapsed with global flag"
    );
}

#[test]
fn e2e_running_tool_always_expanded() {
    let mut state = AppState::default();
    state.agent.streaming = true;

    state.update(Event::AgentToolStart {
        id: "req.0".to_string(),
        name: "ls".to_string(),
    });
    state.update(Event::ToggleExpand);

    assert!(
        state.view.all_collapsed,
        "Global flag should flip even with running tool"
    );

    let out = render_content(&mut state);
    assert!(
        out.contains("Running"),
        "Running tool should still show as running"
    );
}

#[test]
fn e2e_global_toggle_collapses_mixed_thought_and_tool() {
    let mut state = AppState::default();
    state.agent.streaming = true;

    state.update(Event::AgentThinking {
        id: "req.0".to_string(),
    });
    state.update(Event::AgentResponse {
        id: "req.0".to_string(),
        content: "A".to_string(),
    });
    state.update(Event::AgentThoughtDone {
        id: "req.0".to_string(),
    });

    state.update(Event::AgentToolStart {
        id: "req.0".to_string(),
        name: "ls".to_string(),
    });
    state.update(Event::AgentToolEnd {
        duration_secs: 0.5,
        output: "file1".to_string(),
    });

    state.update(Event::ToggleExpand);
    assert!(
        state.view.all_collapsed,
        "Toggle should collapse ALL thoughts and tools globally"
    );

    let out = render_content(&mut state);
    assert!(!out.contains("file1"), "Tool should be collapsed");
}

#[test]
fn e2e_full_turn_with_global_toggle() {
    let mut state = AppState::default();
    state.agent.streaming = true;
    dispatch(
        &mut state,
        &[
            Event::AgentThinking { id: "req.0".into() },
            Event::AgentResponse { id: "req.0".into(), content: "I'll list files.\n".into() },
            Event::AgentResponse { id: "req.0".into(), content: "TOOL:list_dir:.".into() },
            Event::AgentThoughtDone { id: "req.0".into() },
        ],
    );
    let r1 = render_content(&mut state);
    assert!(r1.contains("I'll list files"), "Should show reasoning in thought");
    state.update(Event::ToggleExpand);
    let r2 = render_content(&mut state);
    assert!(r2.contains("[+]") || !r2.contains("I'll list files"), "Thought should collapse");
    dispatch(&mut state, &[
        Event::AgentToolStart { id: "req.0".into(), name: "list_dir".into() },
        Event::AgentToolEnd { duration_secs: 0.5, output: "file1\nfile2".into() },
    ]);
    let r3 = render_content(&mut state);
    assert!(!r3.contains("file1"), "Tool should be collapsed with global flag");
    state.update(Event::ToggleExpand);
    let r4 = render_content(&mut state);
    assert!(r4.contains("file1") && r4.contains("file2"), "Tool should be expanded");
    assert!(r4.contains("I'll list files"), "Thought should also be expanded");
    dispatch(&mut state, &[
        Event::AgentResponse { id: "req.0".into(), content: "Done.".into() },
        Event::AgentDone { id: "req.0".into() },
    ]);
    let r5 = render_content(&mut state);
    assert!(r5.contains("Done."), "Agent response should be visible");
    assert!(r5.contains("I'll list files"), "Thought stays expanded after done");
    assert!(r5.contains("file1"), "Tool stays expanded after done");
}
