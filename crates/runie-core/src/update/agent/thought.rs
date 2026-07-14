//! Thought extraction from assistant content, including `<think>` reasoning blocks.

use crate::labels::thought_with_time;
use crate::message::Part;
use crate::model::ChatMessage;
use crate::update::{content_has_tool_markers, strip_tool_markers};

/// Drain all `Part::Reasoning` contents from `msg`, concatenated in order.
///
/// Native reasoning (provider `ThinkingDelta` / `reasoning_content`) lives in
/// these parts, which [`ChatMessage::content`] does not see. `add_thought`
/// moves them into the thought message so the reasoning has a single home
/// and survives cleanup of an otherwise-empty assistant message.
pub(crate) fn take_reasoning_parts(msg: &mut ChatMessage) -> String {
    let mut reasoning = String::new();
    msg.parts.retain(|part| {
        if let Part::Reasoning { content } = part {
            if !reasoning.is_empty() && !content.is_empty() {
                reasoning.push('\n');
            }
            reasoning.push_str(content);
            false
        } else {
            true
        }
    });
    reasoning
}

#[derive(Debug)]
pub(crate) struct ThoughtPlan {
    pub thought_content: String,
    pub visible_content: Option<String>,
    pub remove_assistant: bool,
}

impl ThoughtPlan {
    pub fn plain(duration: f64) -> Self {
        Self {
            thought_content: thought_with_time(duration),
            visible_content: None,
            remove_assistant: false,
        }
    }

    /// Thought extracted incrementally during streaming: the visible text
    /// parts were already cleaned by [`ThinkStreamFilter`], so only the
    /// accumulated reasoning needs reporting.
    pub fn streamed(duration: f64, reasoning: &str, visible_empty: bool) -> Self {
        Self {
            thought_content: format!("{}\n{}", thought_with_time(duration), reasoning),
            visible_content: None,
            remove_assistant: visible_empty,
        }
    }
}

pub(crate) fn plan_thought(content: &str, duration: f64) -> ThoughtPlan {
    let has_tools = content_has_tool_markers(content);
    let stripped = strip_tool_markers(content);
    let (visible, reasoning) = split_think_blocks(&stripped);
    if has_tools && !stripped.trim().is_empty() {
        return ThoughtPlan {
            thought_content: format!("{}\n{}", thought_with_time(duration), stripped),
            visible_content: None,
            remove_assistant: true,
        };
    }
    if let Some(reasoning) = reasoning {
        let thought_content = format!("{}\n{}", thought_with_time(duration), reasoning);
        if visible.trim().is_empty() {
            return ThoughtPlan {
                thought_content,
                visible_content: None,
                remove_assistant: true,
            };
        }
        return ThoughtPlan {
            thought_content,
            visible_content: Some(visible),
            remove_assistant: false,
        };
    }
    ThoughtPlan {
        thought_content: thought_with_time(duration),
        visible_content: None,
        remove_assistant: false,
    }
}

/// Split `<think>...</think>` reasoning blocks out of model text.
/// Returns `(visible_text, optional_reasoning)`. Unclosed `<think>` tags
/// are treated as reasoning that continues to the end of the string.
pub(crate) fn split_think_blocks(content: &str) -> (String, Option<String>) {
    static THINK_REGEX: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"(?s)<think>(.*?)</think>").unwrap());

    let caps: Vec<_> = THINK_REGEX.captures_iter(content).collect();
    let has_complete = !caps.is_empty();
    let block_reasoning = extract_block_reasoning(&caps);
    let unclosed_reasoning = if !has_complete && content.contains("<think>") {
        {
            content
                .find("<think>")
                .map(|p| &content[p + 7..])
                .unwrap_or("")
        }
    } else {
        ""
    };

    if block_reasoning.is_empty() && unclosed_reasoning.is_empty() {
        return (content.to_string(), None);
    }

    let visible = if has_complete {
        THINK_REGEX.replace_all(content, "").to_string()
    } else {
        content
            .find("<think>")
            .map_or(content.to_string(), |p| content[..p].to_string())
    };

    let all_reasoning = format!("{block_reasoning}{unclosed_reasoning}");
    if all_reasoning.is_empty() {
        (visible, None)
    } else {
        (visible, Some(all_reasoning))
    }
}

fn extract_block_reasoning(caps: &[regex::Captures]) -> String {
    caps.iter()
        .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
        .collect::<Vec<_>>()
        .join("")
}

const OPEN_TAG: &str = "<think>";
const CLOSE_TAG: &str = "</think>";

/// Incremental `<think>...</think>` splitter for streamed response deltas.
///
/// Holds partial-tag tails back across delta boundaries so raw `<think>` /
/// `</think>` markup (or a fragment of either) never reaches the visible
/// assistant text mid-stream. Reasoning accumulates inside the filter and is
/// handed to `add_thought` at turn finish. This is the streaming counterpart
/// of [`split_think_blocks`], which only runs on complete content.
#[derive(Debug, Default, Clone)]
pub struct ThinkStreamFilter {
    /// Held-back tail that may be a partial tag (at most one byte shorter
    /// than the tag currently being scanned for).
    pending: String,
    /// Whether the stream is currently inside a `<think>` block.
    in_think: bool,
    /// Reasoning accumulated from think blocks so far.
    reasoning: String,
}

/// Output of feeding one delta through [`ThinkStreamFilter`].
#[derive(Debug, Default)]
pub(crate) struct ThinkStreamOutput {
    /// Text safe to append to the visible assistant message.
    pub visible: String,
    /// Reasoning resolved by this feed (also accumulated in the filter).
    pub reasoning: String,
}

impl ThinkStreamFilter {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Feed one streamed delta. Text outside think blocks is returned as
    /// visible once tag boundaries are resolved; text inside blocks is
    /// accumulated as reasoning.
    pub(crate) fn push_delta(&mut self, delta: &str) -> ThinkStreamOutput {
        let mut text = std::mem::take(&mut self.pending);
        text.push_str(delta);
        let mut out = ThinkStreamOutput::default();
        loop {
            let tag = if self.in_think { CLOSE_TAG } else { OPEN_TAG };
            match text.find(tag) {
                Some(pos) => {
                    self.push_segment(&mut out, &text[..pos]);
                    self.in_think = !self.in_think;
                    text = text[pos + tag.len()..].to_owned();
                }
                None => {
                    let hold = trailing_tag_prefix_len(&text, tag);
                    let (emit, keep) = text.split_at(text.len() - hold);
                    self.push_segment(&mut out, emit);
                    self.pending = keep.to_owned();
                    return out;
                }
            }
        }
    }

    /// Resolve held-back state at turn finish. A pending tail inside an
    /// unclosed think block is reasoning; outside a block it is plain text
    /// (a partial tag that never completed is emitted verbatim).
    pub(crate) fn finish(&mut self) -> ThinkStreamOutput {
        let mut out = ThinkStreamOutput::default();
        let pending = std::mem::take(&mut self.pending);
        self.push_segment(&mut out, &pending);
        self.in_think = false;
        out
    }

    /// All reasoning accumulated so far, including unclosed blocks.
    #[cfg(test)]
    pub(crate) fn reasoning(&self) -> &str {
        &self.reasoning
    }

    /// Take all reasoning accumulated so far. Called by `add_thought` so a
    /// turn with multiple thinking cycles reports each cycle's reasoning
    /// once instead of re-emitting earlier cycles.
    pub(crate) fn take_reasoning(&mut self) -> String {
        std::mem::take(&mut self.reasoning)
    }

    /// Clear all per-turn state.
    pub(crate) fn reset(&mut self) {
        self.pending.clear();
        self.in_think = false;
        self.reasoning.clear();
    }

    fn push_segment(&mut self, out: &mut ThinkStreamOutput, segment: &str) {
        if segment.is_empty() {
            return;
        }
        if self.in_think {
            self.reasoning.push_str(segment);
            out.reasoning.push_str(segment);
        } else {
            out.visible.push_str(segment);
        }
    }
}

/// Length of the longest suffix of `s` that is a proper prefix of `tag`.
/// Such a suffix may complete into a full tag in a later delta, so it must
/// be held back. Non-ASCII suffixes can never match (tags are ASCII), so
/// candidate lengths that would split a multi-byte character are skipped.
fn trailing_tag_prefix_len(s: &str, tag: &str) -> usize {
    let max = (tag.len() - 1).min(s.len());
    for len in (1..=max).rev() {
        let start = s.len() - len;
        if s.is_char_boundary(start) && tag.starts_with(&s[start..]) {
            return len;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AppState, Role};

    #[test]
    fn split_think_blocks_extracts_reasoning() {
        let (visible, reasoning) = split_think_blocks("<think>reason</think>answer");
        assert_eq!(visible, "answer");
        assert_eq!(reasoning, Some("reason".to_string()));
    }

    #[test]
    fn split_think_blocks_handles_unclosed_tag() {
        let (visible, reasoning) = split_think_blocks("visible<think>still reasoning");
        assert_eq!(visible, "visible");
        assert_eq!(reasoning, Some("still reasoning".to_string()));
    }

    #[test]
    fn split_think_blocks_preserves_text_without_tags() {
        let (visible, reasoning) = split_think_blocks("just an answer");
        assert_eq!(visible, "just an answer");
        assert_eq!(reasoning, None);
    }

    #[test]
    fn plan_thought_keeps_visible_answer_and_reasoning() {
        let plan = plan_thought("<think>reason</think>answer", 1.5);
        assert!(plan.thought_content.contains("reason"));
        assert_eq!(plan.visible_content, Some("answer".to_string()));
        assert!(!plan.remove_assistant);
    }

    #[test]
    fn plan_thought_removes_assistant_when_only_reasoning() {
        let plan = plan_thought("<think>reason</think>", 1.0);
        assert!(plan.remove_assistant);
        assert!(plan.thought_content.contains("reason"));
    }

    #[test]
    fn think_tags_are_split_into_thought_and_answer() {
        let mut state = AppState::default();
        state.set_thinking("req.0".into());
        state.append_response(
            "req.0".into(),
            "<think>\nreasoning\n</think>\nanswer".into(),
        );
        state.add_thought("req.0".into());
        state.finish_turn("req.0".into());

        let thoughts: Vec<_> = state
            .session
            .messages
            .iter()
            .filter(|m| m.role == Role::Thought)
            .collect();
        assert_eq!(thoughts.len(), 1);
        assert!(thoughts[0].content().contains("reasoning"));
        assert!(!thoughts[0].content().contains("<think>"));
        assert!(!thoughts[0].content().contains("</think>"));

        let assistants: Vec<_> = state
            .session
            .messages
            .iter()
            .filter(|m| m.role == Role::Assistant)
            .collect();
        assert_eq!(assistants.len(), 1);
        assert_eq!(assistants[0].content(), "answer");
    }

    // ---------------------------------------------------------------------
    // Streaming: MiniMax-style `<think>` tags inside ResponseDelta content.
    // Raw tags (or partial-tag fragments) must NEVER be observable in the
    // visible assistant text mid-stream, no matter where deltas split.
    // ---------------------------------------------------------------------

    fn streaming_test_state() -> AppState {
        let mut state = AppState::default();
        let config = crate::config::Config::default();
        state.apply_config(&config);
        state
    }

    fn feed_delta(state: &mut AppState, id: &str, content: &str) {
        state.update(crate::Event::ResponseDelta {
            id: id.to_string(),
            content: content.to_string(),
        });
    }

    fn assistant_texts(state: &AppState) -> Vec<String> {
        state
            .session
            .messages
            .iter()
            .filter(|m| m.role == Role::Assistant)
            .map(|m| m.content())
            .collect()
    }

    fn thought_texts(state: &AppState) -> Vec<String> {
        state
            .session
            .messages
            .iter()
            .filter(|m| m.role == Role::Thought)
            .map(|m| m.content())
            .collect()
    }

    /// No assistant text may ever contain a complete or partial think tag.
    fn assert_no_tag_leak(state: &AppState) {
        for text in assistant_texts(state) {
            for partial in ["<think>", "</think>", "<think", "</think", "<thi", "</thi"] {
                assert!(
                    !text.contains(partial),
                    "visible assistant text leaked {partial:?}: {text:?}"
                );
            }
        }
    }

    /// The exact scenario observed live with MiniMax: reasoning streams inside
    /// content deltas, then the answer follows in the same stream.
    #[test]
    fn streamed_think_block_never_leaks_tags_mid_stream() {
        let mut state = streaming_test_state();
        let id = "req.stream.1";
        state.update(crate::Event::Thinking { id: id.into() });

        feed_delta(&mut state, id, "<think>let me calc");
        assert_no_tag_leak(&state);
        assert!(
            assistant_texts(&state).iter().all(|t| t.is_empty()),
            "nothing may be visible while inside a think block, got {:?}",
            assistant_texts(&state)
        );

        feed_delta(&mut state, id, "ulate</think>The ans");
        assert_no_tag_leak(&state);
        assert_eq!(assistant_texts(&state), vec!["The ans".to_string()]);

        feed_delta(&mut state, id, "wer is 42");
        assert_no_tag_leak(&state);
        assert_eq!(
            assistant_texts(&state),
            vec!["The answer is 42".to_string()]
        );

        state.update(crate::Event::ThoughtDone { id: id.into() });
        state.update(crate::Event::Done { id: id.into() });

        let thoughts = thought_texts(&state);
        assert_eq!(thoughts.len(), 1, "expected one thought, got {thoughts:?}");
        assert!(thoughts[0].contains("let me calculate"));
        assert!(!thoughts[0].contains("<think>"));
        assert_eq!(
            assistant_texts(&state),
            vec!["The answer is 42".to_string()]
        );
    }

    /// The closing tag itself is split across two deltas.
    #[test]
    fn streamed_split_closing_tag_does_not_leak() {
        let mut state = streaming_test_state();
        let id = "req.stream.2";
        state.update(crate::Event::Thinking { id: id.into() });

        feed_delta(&mut state, id, "<think>reason</thi");
        assert_no_tag_leak(&state);
        assert!(
            assistant_texts(&state).iter().all(|t| t.is_empty()),
            "partial closing tag must be held back, got {:?}",
            assistant_texts(&state)
        );

        feed_delta(&mut state, id, "nk>answer");
        assert_no_tag_leak(&state);
        assert_eq!(assistant_texts(&state), vec!["answer".to_string()]);

        state.update(crate::Event::ThoughtDone { id: id.into() });
        state.update(crate::Event::Done { id: id.into() });

        let thoughts = thought_texts(&state);
        assert_eq!(thoughts.len(), 1, "expected one thought, got {thoughts:?}");
        assert!(thoughts[0].contains("reason"));
        assert!(!thoughts[0].contains("</thi"));
        assert_eq!(assistant_texts(&state), vec!["answer".to_string()]);
    }

    /// The opening tag is split across two deltas, mid visible text.
    #[test]
    fn streamed_split_opening_tag_does_not_leak() {
        let mut state = streaming_test_state();
        let id = "req.stream.3";
        state.update(crate::Event::Thinking { id: id.into() });

        feed_delta(&mut state, id, "ans<thi");
        assert_no_tag_leak(&state);
        assert_eq!(
            assistant_texts(&state),
            vec!["ans".to_string()],
            "partial opening tag must be held back"
        );

        feed_delta(&mut state, id, "nk>r</think>wer");
        assert_no_tag_leak(&state);
        assert_eq!(assistant_texts(&state), vec!["answer".to_string()]);

        state.update(crate::Event::ThoughtDone { id: id.into() });
        state.update(crate::Event::Done { id: id.into() });

        let thoughts = thought_texts(&state);
        assert_eq!(thoughts.len(), 1, "expected one thought, got {thoughts:?}");
        assert!(thoughts[0].contains('r'));
        assert_eq!(assistant_texts(&state), vec!["answer".to_string()]);
    }

    /// A think block that never closes: all reasoning, no visible text.
    #[test]
    fn streamed_unclosed_think_block_becomes_thought_only() {
        let mut state = streaming_test_state();
        let id = "req.stream.4";
        state.update(crate::Event::Thinking { id: id.into() });

        feed_delta(&mut state, id, "<think>reason only");
        assert_no_tag_leak(&state);
        assert!(
            assistant_texts(&state).iter().all(|t| t.is_empty()),
            "unclosed think content must not be visible, got {:?}",
            assistant_texts(&state)
        );

        state.update(crate::Event::ThoughtDone { id: id.into() });
        state.update(crate::Event::Done { id: id.into() });

        let thoughts = thought_texts(&state);
        assert_eq!(thoughts.len(), 1, "expected one thought, got {thoughts:?}");
        assert!(thoughts[0].contains("reason only"));
        assert!(!thoughts[0].contains("<think>"));
        assert!(
            assistant_texts(&state).is_empty(),
            "reasoning-only turn must not leave an assistant message, got {:?}",
            assistant_texts(&state)
        );
    }

    /// Filter state must not bleed into the next turn.
    #[test]
    fn streamed_think_filter_resets_between_turns() {
        let mut state = streaming_test_state();

        state.update(crate::Event::Thinking {
            id: "req.turn.1".into(),
        });
        feed_delta(&mut state, "req.turn.1", "<think>hidden</think>first");
        state.update(crate::Event::ThoughtDone {
            id: "req.turn.1".into(),
        });
        state.update(crate::Event::Done {
            id: "req.turn.1".into(),
        });

        state.update(crate::Event::Thinking {
            id: "req.turn.2".into(),
        });
        feed_delta(&mut state, "req.turn.2", "second answer");
        assert_no_tag_leak(&state);
        state.update(crate::Event::ThoughtDone {
            id: "req.turn.2".into(),
        });
        state.update(crate::Event::Done {
            id: "req.turn.2".into(),
        });

        let assistants = assistant_texts(&state);
        assert_eq!(
            assistants,
            vec!["first".to_string(), "second answer".to_string()]
        );
        let thoughts = thought_texts(&state);
        assert_eq!(thoughts.len(), 2, "expected two thoughts, got {thoughts:?}");
        assert!(thoughts[0].contains("hidden"));
        assert!(
            !thoughts[1].contains("hidden"),
            "turn 2 thought must not contain turn 1 reasoning: {thoughts:?}"
        );
    }

    /// Two thinking cycles within one request (tool call in between): each
    /// thought must contain only its own cycle's reasoning.
    #[test]
    fn streamed_think_reasoning_does_not_repeat_across_thought_cycles() {
        let mut state = streaming_test_state();
        let id = "req.cycles";

        state.update(crate::Event::Thinking { id: id.into() });
        feed_delta(&mut state, id, "<think>first reason</think>");
        assert_no_tag_leak(&state);
        state.update(crate::Event::ThoughtDone { id: id.into() });

        state.update(crate::Event::ToolStart {
            id: id.into(),
            name: "list_dir".into(),
            input: serde_json::json!({ "path": "." }),
        });
        state.update(crate::Event::ToolEnd {
            id: id.into(),
            duration_secs: 0.5,
            output: "src/".into(),
            input: None,
        });

        // Same request id: set_thinking is idempotent-skipped, so the filter
        // is not reset between cycles — reasoning must still not repeat.
        state.update(crate::Event::Thinking { id: id.into() });
        feed_delta(&mut state, id, "<think>second reason</think>final answer");
        assert_no_tag_leak(&state);
        state.update(crate::Event::ThoughtDone { id: id.into() });
        state.update(crate::Event::Done { id: id.into() });

        let thoughts = thought_texts(&state);
        assert_eq!(thoughts.len(), 2, "expected two thoughts, got {thoughts:?}");
        assert!(thoughts[0].contains("first reason"));
        assert!(!thoughts[0].contains("second reason"));
        assert!(thoughts[1].contains("second reason"));
        assert!(
            !thoughts[1].contains("first reason"),
            "cycle 2 thought must not repeat cycle 1 reasoning: {thoughts:?}"
        );
        assert_eq!(assistant_texts(&state), vec!["final answer".to_string()]);
    }

    // ---------------------------------------------------------------------
    // ThinkStreamFilter unit tests
    // ---------------------------------------------------------------------

    #[test]
    fn think_stream_filter_passes_plain_text_through() {
        let mut f = ThinkStreamFilter::new();
        assert_eq!(f.push_delta("Hello ").visible, "Hello ");
        assert_eq!(f.push_delta("world!").visible, "world!");
        assert_eq!(f.reasoning(), "");
        assert_eq!(f.finish().visible, "");
    }

    #[test]
    fn think_stream_filter_splits_complete_block() {
        let mut f = ThinkStreamFilter::new();
        let out = f.push_delta("<think>reason</think>answer");
        assert_eq!(out.visible, "answer");
        assert_eq!(out.reasoning, "reason");
        assert_eq!(f.reasoning(), "reason");
    }

    #[test]
    fn think_stream_filter_holds_partial_close_tag() {
        let mut f = ThinkStreamFilter::new();
        assert_eq!(f.push_delta("<think>reason</thi").visible, "");
        let out = f.push_delta("nk>answer");
        assert_eq!(out.visible, "answer");
        assert_eq!(f.reasoning(), "reason");
    }

    #[test]
    fn think_stream_filter_holds_partial_open_tag() {
        let mut f = ThinkStreamFilter::new();
        assert_eq!(f.push_delta("ans<thi").visible, "ans");
        let out = f.push_delta("nk>r</think>wer");
        assert_eq!(out.visible, "wer");
        assert_eq!(f.reasoning(), "r");
    }

    /// A partial tag that never completes is not a tag: the held-back tail
    /// must surface as plain text once later deltas disambiguate it.
    #[test]
    fn think_stream_filter_releases_false_positive_partial_tag() {
        let mut f = ThinkStreamFilter::new();
        assert_eq!(f.push_delta("answer<thi").visible, "answer");
        // "<thi" + "nk more" = "<think more" — never a complete `<think>`.
        assert_eq!(f.push_delta("nk more").visible, "<think more");
        assert_eq!(f.reasoning(), "");
    }

    /// Regression for the emoji panic class fixed in the provider filter:
    /// suffix scanning must never slice inside a multi-byte character.
    #[test]
    fn think_stream_filter_handles_emoji_before_partial_tag() {
        let mut f = ThinkStreamFilter::new();
        assert_eq!(f.push_delta("wave 👋<th").visible, "wave 👋");
        let out = f.push_delta("ink>r</think>v");
        assert_eq!(out.visible, "v");
        assert_eq!(f.reasoning(), "r");
    }

    #[test]
    fn think_stream_filter_accumulates_multiple_blocks() {
        let mut f = ThinkStreamFilter::new();
        let mut visible = String::new();
        for d in ["<think>r1</think>a", "<think>r2</think>b"] {
            visible.push_str(&f.push_delta(d).visible);
        }
        assert_eq!(visible, "ab");
        assert_eq!(f.reasoning(), "r1r2");
    }

    #[test]
    fn think_stream_filter_finish_resolves_pending() {
        // Unclosed block: the pending partial close tag is reasoning.
        let mut f = ThinkStreamFilter::new();
        assert_eq!(f.push_delta("<think>reason</thi").visible, "");
        assert_eq!(f.finish().visible, "");
        assert_eq!(f.reasoning(), "reason</thi");

        // Outside a block: a pending partial open tag is plain text.
        let mut f = ThinkStreamFilter::new();
        assert_eq!(f.push_delta("answer<thi").visible, "answer");
        assert_eq!(f.finish().visible, "<thi");
        assert_eq!(f.reasoning(), "");
    }

    #[test]
    fn think_stream_filter_reset_clears_state() {
        let mut f = ThinkStreamFilter::new();
        f.push_delta("<think>reason</thi");
        f.reset();
        assert_eq!(f.push_delta("plain").visible, "plain");
        assert_eq!(f.reasoning(), "");
        assert_eq!(f.finish().visible, "");
    }
}
