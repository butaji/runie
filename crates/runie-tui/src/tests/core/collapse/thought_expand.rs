//! Feed-navigation expansion of thought items carrying streamed reasoning.
//!
//! Regression tests for the live MiniMax bug: reasoning streams as native
//! `ThinkingDelta` parts (never as `<think>` tags in content), so the thought
//! item rendered as a dead "◆ Thought for 2.3s" — Enter and Ctrl+O did
//! nothing to it, and on reasoning-only turns the reasoning was deleted
//! together with the empty assistant message.

use runie_core::model::Role;
use runie_core::view::elements::PostKind;
use runie_core::{AppState, Event, Part};
use runie_testing::fresh_state;

use crate::tests::render_with_size;

const REQ: &str = "req.0";
const WIDTH: u16 = 80;

/// MiniMax-style turn: native reasoning (`ThinkingDelta`, no `<think>` tags
/// in content) followed by a tool call and no visible answer — exactly what
/// `fixtures/minimax/*.sse` replay through the OpenAI provider filter.
fn run_reasoning_tool_turn(state: &mut AppState) {
    state.agent.streaming = true;
    state.update(Event::Thinking { id: REQ.into() });
    state.update(Event::ThinkingStart { id: REQ.into() });
    state.update(Event::ThinkingDelta {
        id: REQ.into(),
        content: "The user wants the directory listing. I will run ls -la.".into(),
    });
    state.update(Event::ThoughtDone { id: REQ.into() });
    state.update(Event::ToolStart {
        id: REQ.into(),
        name: "bash".into(),
        input: serde_json::json!({ "command": "ls -la" }),
    });
    state.update(Event::ToolEnd {
        id: REQ.into(),
        duration_secs: 0.5,
        output: "file1\nfile2".into(),
        input: None,
    });
    state.update(Event::Done { id: REQ.into() });
}

/// Buffer rows as strings — `render_with_size` concatenates cells row-major
/// without newlines, so split by the render width.
fn rendered_lines(rendered: &str) -> Vec<String> {
    let chars: Vec<char> = rendered.chars().collect();
    chars
        .chunks(WIDTH as usize)
        .map(|row| row.iter().collect::<String>())
        .collect()
}

/// Whitespace-insensitive view of the render: wrapped rows are padded with
/// spaces, which would break naive `contains` on multi-word phrases.
fn squashed(rendered: &str) -> String {
    rendered.chars().filter(|c| !c.is_whitespace()).collect()
}

#[test]
fn nav_enter_expands_collapsed_native_reasoning_thought() {
    let mut state = fresh_state();
    run_reasoning_tool_turn(&mut state);

    // The reasoning must live in the thought message, not vanish with the
    // (now empty) assistant message at turn finish.
    let thought = state
        .session
        .messages
        .iter()
        .find(|m| m.role == Role::Thought)
        .expect("thought message");
    assert!(
        thought.content().contains("directory listing"),
        "thought should carry the streamed reasoning, got: {:?}",
        thought.content()
    );

    // Collapse the feed: the thought renders as a one-line summary that
    // hides the reasoning body.
    state.update(Event::ToggleExpand);
    assert!(state.view.all_collapsed);
    let collapsed = render_with_size(&mut state, WIDTH, 24);
    assert!(
        !squashed(&collapsed).contains("directorylisting"),
        "collapsed thought must hide the reasoning body"
    );

    // Enter feed navigation (Esc), select the thought post, press Enter —
    // the hint bar advertises "Enter expand".
    state.update(Event::DialogBack);
    assert!(state.view.vim_nav_mode, "Esc should enter feed navigation");
    state.update(Event::Input('k'));
    let selected = state.view.selected_post.expect("a post is selected");
    let snap = state.snapshot();
    assert_eq!(
        snap.posts[selected].kind,
        PostKind::Thought,
        "k should select the thought post"
    );
    state.update(Event::Submit);

    let expanded = render_with_size(&mut state, WIDTH, 24);
    assert!(
        squashed(&expanded).contains("directorylisting"),
        "Enter on the selected thought must reveal the reasoning inline"
    );

    // Enter again collapses it back to the one-line summary.
    state.update(Event::Submit);
    let recollapsed = render_with_size(&mut state, WIDTH, 24);
    assert!(
        !squashed(&recollapsed).contains("directorylisting"),
        "second Enter must collapse the thought again"
    );
    assert!(
        recollapsed.contains("Thought for"),
        "collapsed thought with a body must render its summary line"
    );
    assert!(
        !recollapsed.contains("[+]"),
        "collapsed thoughts no longer render the retired [+] affordance"
    );
}

#[test]
fn native_reasoning_moves_to_thought_and_answer_is_kept() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(Event::Thinking { id: REQ.into() });
    state.update(Event::ThinkingStart { id: REQ.into() });
    state.update(Event::ThinkingDelta {
        id: REQ.into(),
        content: "I should read the README first.".into(),
    });
    state.update(Event::ResponseDelta {
        id: REQ.into(),
        content: "Here is the summary.".into(),
    });
    state.update(Event::ThoughtDone { id: REQ.into() });
    state.update(Event::Done { id: REQ.into() });

    let thought = state
        .session
        .messages
        .iter()
        .find(|m| m.role == Role::Thought)
        .expect("thought message");
    assert!(
        thought.content().contains("read the README"),
        "thought should carry the native reasoning, got {:?}",
        thought.content()
    );

    let assistant = state
        .session
        .messages
        .iter()
        .find(|m| m.role == Role::Assistant)
        .expect("assistant answer preserved");
    assert_eq!(assistant.content(), "Here is the summary.");
    assert!(
        !assistant
            .parts
            .iter()
            .any(|p| matches!(p, Part::Reasoning { .. })),
        "reasoning must render from the thought only, not duplicated on the assistant message"
    );

    // Grok parity: the thought renders as a one-line summary by default, so
    // the reasoning body is hidden until the post is individually expanded.
    let rendered = render_with_size(&mut state, WIDTH, 24);
    assert!(
        !squashed(&rendered).contains("readtheREADME"),
        "collapsed thought should hide the reasoning body"
    );

    // Individually expanding the thought post (Enter in feed nav) reveals
    // the reasoning inline.
    let thought_idx = state
        .snapshot()
        .posts
        .iter()
        .position(|p| p.kind == PostKind::Thought)
        .expect("thought post");
    state.view.expanded_posts.insert(thought_idx);
    state.messages_changed();
    let rendered = render_with_size(&mut state, WIDTH, 24);
    assert!(
        squashed(&rendered).contains("readtheREADME"),
        "expanded thought should show the reasoning inline"
    );
}

#[test]
fn duration_only_thought_renders_without_expand_affordance() {
    let mut state = fresh_state();
    state.agent.streaming = true;
    state.update(Event::Thinking { id: REQ.into() });
    state.update(Event::ResponseDelta {
        id: REQ.into(),
        content: "plain answer, no reasoning".into(),
    });
    state.update(Event::ThoughtDone { id: REQ.into() });
    state.update(Event::Done { id: REQ.into() });

    // Even in the collapsed view, a thought with no body must still render
    // its summary line — and never an expand affordance.
    state.update(Event::ToggleExpand);
    assert!(state.view.all_collapsed);
    let rendered = render_with_size(&mut state, WIDTH, 24);
    let thought_line = rendered_lines(&rendered)
        .into_iter()
        .find(|l| l.contains("Thought for"))
        .unwrap_or_default();
    assert!(
        !thought_line.is_empty(),
        "duration-only thought summary should still render"
    );
    assert!(
        !thought_line.contains("[+]"),
        "thought summaries must not render the retired [+] affordance, got line: {thought_line:?}"
    );
}
