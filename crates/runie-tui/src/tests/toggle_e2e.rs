#![allow(clippy::too_many_lines)]
use super::*;
use runie_core::Event;

fn render_content(state: &mut AppState) -> String {
    render_with_size(state, 60, 20)
}

fn dispatch(state: &mut AppState, events: &[Event]) {
    for e in events {
        state.update(e.clone());
    }
}

fn setup_thought_and_tool(state: &mut AppState) {
    state.agent.streaming = true;
    dispatch(
        state,
        &[
            Event::Thinking { id: "req.0".into() },
            Event::Response {
                id: "req.0".into(),
                content: "I'll list files.\n".into(),
                role: String::new(),
                timestamp: 0.0,
                provider: String::new(),
            },
            Event::Response {
                id: "req.0".into(),
                content: "TOOL:list_dir:.".into(),
                role: String::new(),
                timestamp: 0.0,
                provider: String::new(),
            },
            Event::ThoughtDone { id: "req.0".into() },
        ],
    );
}

fn run_tool(state: &mut AppState, output: &str) {
    dispatch(
        state,
        &[
            Event::ToolStart { id: "req.0".into(), name: "list_dir".into(), input: serde_json::Value::Null },
            Event::ToolEnd { id: "".to_string(), input: None, duration_secs: 0.5, output: output.into() },
        ],
    );
}

fn assert_collapsed(content: &str, label: &str) {
    assert!(
        content.contains("[+]") || !content.contains("I'll list files"),
        "{}: should hide reasoning after toggle",
        label
    );
}

fn assert_tools_expanded(content: &str) {
    assert!(
        content.contains("file1") && content.contains("file2"),
        "Tool should be expanded"
    );
}

fn finish_turn(state: &mut AppState) {
    dispatch(
        state,
        &[
            Event::Response {
                id: "req.0".into(),
                content: "Done.".into(),
                role: String::new(),
                timestamp: 0.0,
                provider: String::new(),
            },
            Event::Done { id: "req.0".into() },
        ],
    );
}

#[test]
fn e2e_toggle_collapses_all_thoughts_and_tools() {
    let mut state = AppState::default();
    setup_thought_and_tool(&mut state);
    run_tool(&mut state, "file1\nfile2");

    // New default (grok parity): the thought renders as a one-line summary
    // (reasoning hidden) while tool output is fully expanded.
    let before = render_content(&mut state);
    assert!(
        !before.contains("I'll list files"),
        "Thought reasoning should be hidden by default"
    );
    assert!(
        before.contains("file1"),
        "Tool output should be expanded by default"
    );

    // Ctrl+O collapses the tools globally; the thought stays summarized.
    state.update(Event::ToggleExpand);
    assert!(
        state.view.all_collapsed,
        "Toggle should set global collapse"
    );

    let after = render_content(&mut state);
    assert_collapsed(&after, "after toggle");
    assert!(
        !after.contains("file1"),
        "Tool output should collapse on global toggle"
    );
}

#[test]
fn e2e_enter_expands_thought_and_collapses_again() {
    let mut state = AppState::default();
    setup_thought_and_tool(&mut state);

    // Default: the thought is summarized (Ctrl+O no longer expands
    // thoughts — grok's per-item model uses Enter in feed nav).
    let collapsed = render_content(&mut state);
    assert_collapsed(&collapsed, "default summary");

    // Esc enters feed nav (the first Esc aborts the in-flight turn, the
    // second enters nav and selects the bottom post — the thought);
    // Enter expands it individually.
    state.update(Event::DialogBack);
    state.update(Event::DialogBack);
    assert!(state.view.vim_nav_mode, "Esc should enter feed navigation");
    state.update(Event::Submit);
    let expanded = render_content(&mut state);
    assert!(
        expanded.contains("I'll list files"),
        "Enter should expand the thought and show its reasoning"
    );

    // A second Enter collapses the thought back to its summary.
    state.update(Event::Submit);
    let recollapsed = render_content(&mut state);
    assert_collapsed(&recollapsed, "thought re-collapsed by second Enter");
}

#[test]
fn e2e_all_collapsed_stays_collapsed_through_tool_execution() {
    let mut state = AppState::default();
    setup_thought_and_tool(&mut state);
    state.update(Event::ToggleExpand);
    assert!(state.view.all_collapsed);

    run_tool(&mut state, "file1\nfile2");
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
    setup_thought_and_tool(&mut state);
    state.update(Event::ToggleExpand);
    assert!(state.view.all_collapsed);

    run_tool(&mut state, "file1");
    dispatch(
        &mut state,
        &[
            Event::Response {
                id: "req.0".into(),
                content: "Done.".into(),
                role: String::new(),
                timestamp: 0.0,
                provider: String::new(),
            },
            Event::Done { id: "req.0".into() },
        ],
    );
    let after_done = render_content(&mut state);
    assert!(
        !after_done.contains("I'll list files"),
        "Thought should stay collapsed after agent done"
    );
    assert!(
        !after_done.contains("file1"),
        "Tool should stay collapsed after agent done"
    );
}

#[test]
fn e2e_new_thought_respects_global_collapse() {
    let mut state = AppState::default();
    state.agent.streaming = true;

    state.update(Event::Thinking { id: "req.0".to_string() });
    state.update(Event::Response {
        id: "req.0".to_string(),
        content: "First.".to_string(),
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),
    });
    state.update(Event::ThoughtDone { id: "req.0".to_string() });

    state.update(Event::ToggleExpand);
    assert!(state.view.all_collapsed);

    state.update(Event::Thinking { id: "req.1".to_string() });
    state.update(Event::Response {
        id: "req.1".to_string(),
        content: "Second.".to_string(),
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),
    });
    state.update(Event::ThoughtDone { id: "req.1".to_string() });

    let after = render_content(&mut state);
    let marker_count = after.matches("Thought").count();
    let summary_count = after.matches("[+]").count();
    assert!(
        marker_count >= 2,
        "Both thoughts should still render their summary lines when collapsed"
    );
    // These thoughts are duration-only (no reasoning body): the collapsed
    // view must NOT render the [+] affordance for them — it would be dead.
    assert_eq!(
        summary_count, 0,
        "duration-only thoughts must not show the [+] expand affordance"
    );
}

#[test]
fn e2e_running_tool_always_expanded() {
    let mut state = AppState::default();
    state.agent.streaming = true;

    state.update(Event::ToolStart { id: "req.0".to_string(), name: "ls".to_string(), input: serde_json::Value::Null });
    state.update(Event::ToggleExpand);

    assert!(
        state.view.all_collapsed,
        "Global flag should flip even with running tool"
    );

    let out = render_content(&mut state);
    assert!(
        out.contains("Run"),
        "Running tool should still show as running"
    );
}

#[test]
fn e2e_global_toggle_collapses_mixed_thought_and_tool() {
    let mut state = AppState::default();
    state.agent.streaming = true;

    state.update(Event::Thinking { id: "req.0".to_string() });
    state.update(Event::Response {
        id: "req.0".to_string(),
        content: "A".to_string(),
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),
    });
    state.update(Event::ThoughtDone { id: "req.0".to_string() });

    state.update(Event::ToolStart { id: "req.0".to_string(), name: "ls".to_string(), input: serde_json::Value::Null });
    state.update(Event::ToolEnd { id: "".to_string(), input: None, duration_secs: 0.5, output: "file1".to_string() });

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
    setup_thought_and_tool(&mut state);

    // Default (grok parity): the thought renders as a one-line summary.
    let r1 = render_content(&mut state);
    assert!(
        !r1.contains("I'll list files"),
        "Thought should be summarized by default"
    );

    // Per-post expansion (Enter in feed nav) reveals the reasoning. The
    // first Esc aborts the in-flight turn, the second enters feed nav and
    // selects the bottom post (the thought).
    state.update(Event::DialogBack);
    state.update(Event::DialogBack);
    state.update(Event::Submit);
    let r2 = render_content(&mut state);
    assert!(
        r2.contains("I'll list files"),
        "Enter should expand the thought body"
    );

    // Ctrl+O collapses tools globally and clears per-post expansions.
    state.update(Event::ToggleExpand);
    run_tool(&mut state, "file1\nfile2");
    let r3 = render_content(&mut state);
    assert!(
        !r3.contains("file1"),
        "Tool should be collapsed with global flag"
    );
    assert!(
        !r3.contains("I'll list files"),
        "Ctrl+O should clear the per-post thought expansion"
    );

    // Second Ctrl+O restores tool output; the thought stays summarized —
    // Ctrl+O no longer expands thoughts.
    state.update(Event::ToggleExpand);
    assert!(
        !state.view.all_collapsed,
        "Second toggle should expand tools"
    );
    assert_tools_expanded(&render_content(&mut state));

    finish_turn(&mut state);
    let r5 = render_content(&mut state);
    assert!(r5.contains("Done."), "Agent response should be visible");
    assert!(r5.contains("file1"), "Tool stays expanded after done");
    assert!(
        !r5.contains("I'll list files"),
        "Thought stays summarized after done"
    );
}
