use super::*;
use super::super::*;
use runie_core::Event;
use runie_core::Part;

#[test]
fn test_toggle_expand_changes_rendered_output() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();

    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        parts: vec![Part::Text { content: "◆ Thought 1.2s\nI'll list the files.".into() }],
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });

    // Render expanded
    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf_expanded = terminal.backend().buffer().clone();
    let expanded_text: String = buf_expanded.content().iter().map(|c| c.symbol()).collect();
    assert!(
        expanded_text.contains("I'll list the files"),
        "Expanded thought should show reasoning"
    );

    // Toggle collapse
    state.update(Event::ToggleExpand);

    // Render collapsed
    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf_collapsed = terminal.backend().buffer().clone();
    let collapsed_text: String = buf_collapsed.content().iter().map(|c| c.symbol()).collect();
    assert!(
        collapsed_text.contains("[+]"),
        "Collapsed thought should show [+] indicator"
    );
    assert!(
        !collapsed_text.contains("I'll list the files"),
        "Collapsed thought should hide reasoning"
    );
}

#[test]
fn test_toggle_expand_changes_tool_render() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();

    state.session.messages.push(ChatMessage {
        role: Role::Tool,
        parts: vec![Part::Text { content: "◆ Ran list_files 0.5s\nfile1\nfile2".into() }],
        timestamp: 0.0,
        id: "x1".into(),
        ..Default::default()
    });

    // Render expanded
    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf_expanded = terminal.backend().buffer().clone();
    let expanded_text: String = buf_expanded.content().iter().map(|c| c.symbol()).collect();
    assert!(
        expanded_text.contains("file1"),
        "Expanded tool should show output"
    );

    // Toggle collapse
    state.update(Event::ToggleExpand);

    // Render collapsed
    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf_collapsed = terminal.backend().buffer().clone();
    let collapsed_text: String = buf_collapsed.content().iter().map(|c| c.symbol()).collect();
    assert!(
        collapsed_text.contains("[+]"),
        "Collapsed tool should show [+] indicator"
    );
    assert!(
        !collapsed_text.contains("file1"),
        "Collapsed tool should hide output"
    );
}
