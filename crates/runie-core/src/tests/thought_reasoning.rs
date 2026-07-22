//! End-to-end thought tests: provider thinking events (ThinkingStart /
//! ThinkingDelta / ThinkingEnd, as forwarded by runie-agent for MiniMax and
//! other reasoning providers) must land in the Thought message so the feed
//! renders an expandable summary — not a duration-only dead `[+]`.
#![allow(clippy::too_many_lines)]

use crate::model::AppState;
use crate::view::{Element, LazyCache, PostKind};

fn feed_elements(state: &AppState) -> Vec<Element> {
    LazyCache::feed(state).elements
}

fn drive_reasoning_turn(state: &mut AppState, reasoning: &str, answer: &str) {
    let id = "req.0".to_string();
    state.update(crate::Event::Thinking { id: id.clone() });
    state.update(crate::Event::ThinkingStart { id: id.clone() });
    state.update(crate::Event::ThinkingDelta { id: id.clone(), content: reasoning.to_string() });
    state.update(crate::Event::ThinkingEnd { id: id.clone() });
    state.update(crate::Event::TextStart { id: id.clone() });
    state.update(crate::Event::ResponseDelta { id: id.clone(), content: answer.to_string() });
    state.update(crate::Event::ThoughtDone { id: id.clone() });
    state.update(crate::Event::TurnComplete { id, duration_secs: 1.0 });
}

#[test]
fn streamed_reasoning_lands_in_thought_message() {
    let mut state = AppState::default();
    drive_reasoning_turn(&mut state, "Let me think about this.", "The answer");

    let thought = state
        .session
        .messages
        .iter()
        .find(|m| m.role == crate::model::Role::Thought)
        .expect("a Thought message must exist after a reasoning turn");
    assert!(
        thought.content().contains("Let me think about this."),
        "thought must carry the streamed reasoning, got: {:?}",
        thought.content()
    );
}

#[test]
fn collapsed_feed_shows_expandable_thought_summary() {
    let mut state = AppState::default();
    state.view.all_collapsed = true;
    drive_reasoning_turn(&mut state, "Let me think about this.", "The answer");

    let has_expandable = feed_elements(&state)
        .iter()
        .any(|e| matches!(e, Element::ThoughtSummary { expandable: true, .. }));
    assert!(
        has_expandable,
        "collapsed feed must show an EXPANDABLE thought summary when reasoning exists"
    );
    let has_dead_summary = feed_elements(&state)
        .iter()
        .any(|e| matches!(e, Element::ThoughtSummary { expandable: false, .. }));
    assert!(
        !has_dead_summary,
        "a reasoning turn must not render a duration-only dead summary"
    );
}

#[test]
fn answer_text_is_not_polluted_by_reasoning() {
    let mut state = AppState::default();
    drive_reasoning_turn(&mut state, "Let me think about this.", "The answer");

    let answer = state
        .session
        .messages
        .iter()
        .find(|m| m.role == crate::model::Role::Assistant)
        .expect("assistant message must exist");
    assert!(
        !answer.content().contains("Let me think"),
        "reasoning must not leak into the visible answer: {:?}",
        answer.content()
    );
    assert!(answer.content().contains("The answer"));
}

/// Multi-iteration turn (text + tool call, then a second model cycle): each
/// iteration's reasoning must fold into its own Thought message with a proper
/// "Thought for Xs" header — never render as a bare reasoning fragment that
/// looks like a duplicated assistant post.
#[test]
fn multi_iteration_turn_gives_each_reasoning_its_own_thought_message() {
    let mut state = AppState::default();
    let id = "req.0".to_string();

    // Iteration 1: reasoning + "I'll verify" + tool call.
    state.update(crate::Event::Thinking { id: id.clone() });
    state.update(crate::Event::ThinkingStart { id: id.clone() });
    state.update(crate::Event::ThinkingDelta { id: id.clone(), content: "Checking primality.".into() });
    state.update(crate::Event::ThinkingEnd { id: id.clone() });
    state.update(crate::Event::TextStart { id: id.clone() });
    state
        .update(crate::Event::ResponseDelta { id: id.clone(), content: "I'll verify this with a quick check.".into() });
    state.update(crate::Event::ThoughtDone { id: id.clone() });

    // Tool call + execution between iterations.
    state.update(crate::Event::ToolStart {
        id: "call_1".into(),
        name: "bash".into(),
        input: serde_json::json!({"command": "python3 primality.py"}),
    });
    state.update(crate::Event::tool_end(
        "call_1".to_string(),
        0.5,
        "97 is prime".to_string(),
    ));

    // Iteration 2: reasoning + final answer (same request id, as the agent
    // loop reuses the command id across iterations).
    state.update(crate::Event::Thinking { id: id.clone() });
    state.update(crate::Event::ThinkingStart { id: id.clone() });
    state
        .update(crate::Event::ThinkingDelta { id: id.clone(), content: "I'll verify this with a quick check.".into() });
    state.update(crate::Event::ThinkingEnd { id: id.clone() });
    state.update(crate::Event::TextStart { id: id.clone() });
    state.update(crate::Event::ResponseDelta { id: id.clone(), content: "Yes, 97 is prime.".into() });
    state.update(crate::Event::ThoughtDone { id: id.clone() });
    state.update(crate::Event::TurnComplete { id: id.clone(), duration_secs: 2.0 });
    state.update(crate::Event::Done { id });

    let thoughts: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == crate::model::Role::Thought)
        .collect();
    assert_eq!(
        thoughts.len(),
        2,
        "each iteration must produce its own Thought message"
    );
    for t in &thoughts {
        assert!(
            t.content().starts_with("◆ Thought for "),
            "thought must carry the duration header, got: {:?}",
            t.content()
        );
    }
    assert!(
        thoughts[1]
            .content()
            .contains("I'll verify this with a quick check."),
        "iteration-2 reasoning must live in the second thought: {:?}",
        thoughts[1].content()
    );

    // No orphan reasoning parts may remain on assistant messages — those
    // render as bare "◆ <first line>" posts that look like duplicates.
    for m in &state.session.messages {
        if m.role == crate::model::Role::Assistant {
            for part in &m.parts {
                assert!(
                    !matches!(part, crate::message::Part::Reasoning { .. }),
                    "reasoning part left on assistant message renders as a duplicate-looking post"
                );
            }
        }
    }
}

/// Grok parity (GROK.md: "Thinking Block — collapsed by default, toggle with
/// Ctrl+E"; verified live against the grok TUI): a thought must render as a
/// one-line expandable summary by default, and Enter on the selected thought
/// expands its body individually.
#[test]
fn thought_is_collapsed_by_default_and_expands_individually() {
    let mut state = AppState::default();
    drive_reasoning_turn(&mut state, "Let me think about this.", "The answer");

    // Default: one-line expandable summary, body hidden.
    let has_summary = feed_elements(&state)
        .iter()
        .any(|e| matches!(e, Element::ThoughtSummary { expandable: true, .. }));
    assert!(
        has_summary,
        "thought must default to an expandable one-line summary (grok parity), got: {:?}",
        feed_elements(&state)
    );
    let body_visible = feed_elements(&state)
        .iter()
        .any(|e| matches!(e, Element::ThoughtMarker { content, .. } if content.contains("Let me think")));
    assert!(
        !body_visible,
        "thought body must be hidden by default, got: {:?}",
        feed_elements(&state)
    );

    // Expand the thought post individually (what Enter does in nav mode).
    let feed = LazyCache::feed(&state);
    let thought_post = feed
        .posts
        .iter()
        .position(|p| p.kind == PostKind::Thought)
        .expect("thought post must exist");
    state.view_mut().expanded_posts.insert(thought_post);

    let body_visible = feed_elements(&state)
        .iter()
        .any(|e| matches!(e, Element::ThoughtMarker { content, .. } if content.contains("Let me think")));
    assert!(
        body_visible,
        "individually expanded thought must show its body, got: {:?}",
        feed_elements(&state)
    );
}

/// Regression (live-found, BUG 19): grok renders "◆ Thought for Xs" ABOVE
/// the answer. The feed sorts by timestamp, but the thought's timestamp was
/// set at ThoughtDone time — strictly after the last answer-text append —
/// so the thought sorted below the answer. The thought must be anchored to
/// when thinking STARTED.
#[test]
fn thought_sorts_before_the_answer_like_grok() {
    let mut state = AppState::default();
    let id = "req.0".to_string();
    state.update(crate::Event::Thinking { id: id.clone() });
    state.update(crate::Event::ThinkingStart { id: id.clone() });
    state.update(crate::Event::ThinkingDelta { id: id.clone(), content: "reasoning".into() });
    state.update(crate::Event::ThinkingEnd { id: id.clone() });
    state.update(crate::Event::ResponseDelta { id: id.clone(), content: "The answer".into() });
    // Real streams take time: the last text append bumps the assistant
    // timestamp strictly before ThoughtDone fires.
    std::thread::sleep(std::time::Duration::from_millis(2));
    state.update(crate::Event::ThoughtDone { id: id.clone() });
    state.update(crate::Event::TurnComplete { id: id.clone(), duration_secs: 1.0 });
    state.update(crate::Event::Done { id });

    let elements = feed_elements(&state);
    let thought_pos = elements
        .iter()
        .position(|e| {
            matches!(
                e,
                Element::ThoughtMarker { .. } | Element::ThoughtSummary { .. }
            )
        })
        .expect("thought element must exist");
    let answer_pos = elements
        .iter()
        .position(|e| matches!(e, Element::AgentMessage { .. }))
        .expect("answer element must exist");
    assert!(
        thought_pos < answer_pos,
        "grok renders the thought above the answer; got thought at {thought_pos}, answer at {answer_pos}"
    );
}

/// Regression (live-found): a multi-iteration reasoning turn whose tool call
/// arrives as an inline `TOOL:` marker in the streamed text must keep BOTH
/// iterations' assistant text visible in the feed — exactly once each, with
/// no tool markers left behind.
///
/// Root cause: with reasoning, the assistant message ends up with several
/// Text parts ([Text(t1+marker), Reasoning(r2), Text(t2)] → reasoning taken
/// → [Text(t1+marker), Text(t2)]). `strip_tools_from_assistant` wrote the
/// stripped full content via `set_text_part`, which replaces only the LAST
/// Text part: the first part kept the raw marker and the content was
/// duplicated. `should_skip_msg` then saw `content_has_tool_markers` and hid
/// the entire assistant message from the feed — both texts vanished.
#[test]
fn reasoning_marker_tool_turn_keeps_all_assistant_text_in_feed() {
    let mut state = AppState::default();
    let id = "req.0".to_string();

    // Iteration 1: reasoning + text + inline TOOL marker (the real mock /
    // MiniMax text-parsed tool path, not a native ToolCall event).
    state.update(crate::Event::Thinking { id: id.clone() });
    state.update(crate::Event::ThinkingStart { id: id.clone() });
    state.update(crate::Event::ThinkingDelta { id: id.clone(), content: "Deciding to run a check.".into() });
    state.update(crate::Event::ThinkingEnd { id: id.clone() });
    state.update(crate::Event::ResponseDelta {
        id: id.clone(),
        content: "I'll verify this with a quick check.\n".into(),
    });
    state.update(crate::Event::ResponseDelta { id: id.clone(), content: "TOOL:list_dir:.".into() });
    state.update(crate::Event::ThoughtDone { id: id.clone() });

    state.update(crate::Event::ToolStart {
        id: id.clone(),
        name: "list_dir".into(),
        input: serde_json::json!({"path": "."}),
    });
    state.update(crate::Event::tool_end(
        id.clone(),
        0.0,
        "src/\ntests/".to_string(),
    ));

    // Iteration 2: reasoning + final answer (same request id; the agent's
    // second `Thinking` for the same id is idempotent-skipped).
    state.update(crate::Event::Thinking { id: id.clone() });
    state.update(crate::Event::ThinkingStart { id: id.clone() });
    state.update(crate::Event::ThinkingDelta { id: id.clone(), content: "The check confirmed it.".into() });
    state.update(crate::Event::ThinkingEnd { id: id.clone() });
    state.update(crate::Event::ResponseDelta { id: id.clone(), content: "Yes, verified.\n".into() });
    state.update(crate::Event::ThoughtDone { id: id.clone() });
    state.update(crate::Event::TurnComplete { id: id.clone(), duration_secs: 2.0 });
    state.update(crate::Event::Done { id });

    // Session state: the assistant message must not retain a tool marker and
    // must not duplicate content across parts.
    let assistant = state
        .session
        .messages
        .iter()
        .find(|m| m.role == crate::model::Role::Assistant)
        .expect("assistant message must exist");
    assert!(
        !assistant.content().contains("TOOL:"),
        "tool marker must be stripped from the assistant message: {:?}",
        assistant.content()
    );
    assert_eq!(
        assistant
            .content()
            .matches("I'll verify this with a quick check.")
            .count(),
        1,
        "iteration-1 text must appear exactly once: {:?}",
        assistant.content()
    );
    assert_eq!(
        assistant.content().matches("Yes, verified.").count(),
        1,
        "iteration-2 text must appear exactly once: {:?}",
        assistant.content()
    );

    // Feed: both texts must be visible in AgentMessage elements.
    let agent_texts: Vec<String> = feed_elements(&state)
        .into_iter()
        .filter_map(|e| match e {
            Element::AgentMessage { content, .. } => Some(content),
            _ => None,
        })
        .collect();
    assert!(
        agent_texts
            .iter()
            .any(|t| t.contains("I'll verify this with a quick check.")),
        "iteration-1 text must render in the feed, got: {agent_texts:?}"
    );
    assert!(
        agent_texts.iter().any(|t| t.contains("Yes, verified.")),
        "iteration-2 text must render in the feed, got: {agent_texts:?}"
    );
    assert!(
        agent_texts.iter().all(|t| !t.contains("TOOL:")),
        "feed must never show raw tool markers: {agent_texts:?}"
    );
}
