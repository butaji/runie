//! Thought extraction from assistant content, including `<think>` reasoning blocks.

use crate::labels::thought_with_time;
use crate::update::{content_has_tool_markers, strip_tool_markers};

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
}
